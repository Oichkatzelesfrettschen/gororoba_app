use gororoba_kernel_api::algebra::KernelBackend;
use gororoba_kernel_api::fluid::{
    AerodynamicSnapshot, CpuKernelFlavor, FluidBackendCapabilities, FluidBackendError,
    FluidBackendKind, FluidBoundaryConfig, FluidDiagnosticsSnapshot, FluidDomainConfig,
    FluidExecutionConfig, FluidFieldSnapshot, FluidKernel, GridShape3,
};
use rayon::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoxelMask3 {
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
    pub cells: Vec<bool>,
}

impl VoxelMask3 {
    pub fn new(nx: usize, ny: usize, nz: usize) -> Self {
        Self {
            nx,
            ny,
            nz,
            cells: vec![false; nx * ny * nz],
        }
    }

    pub fn from_cells(nx: usize, ny: usize, nz: usize, cells: Vec<bool>) -> Self {
        assert_eq!(cells.len(), nx * ny * nz);
        Self { nx, ny, nz, cells }
    }

    fn index(&self, x: usize, y: usize, z: usize) -> usize {
        z * (self.nx * self.ny) + y * self.nx + x
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, solid: bool) {
        let idx = self.index(x, y, z);
        self.cells[idx] = solid;
    }

    pub fn get(&self, x: usize, y: usize, z: usize) -> bool {
        self.cells[self.index(x, y, z)]
    }

    pub fn solid_count(&self) -> usize {
        self.cells.iter().filter(|&&solid| solid).count()
    }
}

const VELOCITIES: [[i32; 3]; 19] = [
    [0, 0, 0],
    [1, 0, 0],
    [-1, 0, 0],
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, 1],
    [0, 0, -1],
    [1, 1, 0],
    [-1, -1, 0],
    [1, -1, 0],
    [-1, 1, 0],
    [1, 0, 1],
    [-1, 0, -1],
    [1, 0, -1],
    [-1, 0, 1],
    [0, 1, 1],
    [0, -1, -1],
    [0, 1, -1],
    [0, -1, 1],
];

const WEIGHTS: [f32; 19] = [
    1.0 / 3.0,
    1.0 / 18.0,
    1.0 / 18.0,
    1.0 / 18.0,
    1.0 / 18.0,
    1.0 / 18.0,
    1.0 / 18.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0,
];

const OPPOSITE: [usize; 19] = [
    0, 2, 1, 4, 3, 6, 5, 8, 7, 10, 9, 12, 11, 14, 13, 16, 15, 18, 17,
];

const CS_SQ: f32 = 1.0 / 3.0;

struct NeighborTable {
    table: Box<[Vec<u32>; 19]>,
}

impl NeighborTable {
    fn build(nx: usize, ny: usize, nz: usize) -> Self {
        let n = nx * ny * nz;
        let mut table: Box<[Vec<u32>; 19]> = Box::new(std::array::from_fn(|_| vec![0u32; n]));

        for z in 0..nz {
            for y in 0..ny {
                for x in 0..nx {
                    let dst_idx = z * (nx * ny) + y * nx + x;
                    for q in 0..19 {
                        let [cx, cy, cz] = VELOCITIES[q];
                        let sx = ((x as i32 - cx).rem_euclid(nx as i32)) as usize;
                        let sy = ((y as i32 - cy).rem_euclid(ny as i32)) as usize;
                        let sz = ((z as i32 - cz).rem_euclid(nz as i32)) as usize;
                        table[q][dst_idx] = (sz * (nx * ny) + sy * nx + sx) as u32;
                    }
                }
            }
        }

        Self { table }
    }
}

fn make_domain_config(
    nx: usize,
    ny: usize,
    nz: usize,
    tau: f32,
    execution: FluidExecutionConfig,
) -> FluidDomainConfig {
    FluidDomainConfig {
        grid: GridShape3 { nx, ny, nz },
        tau,
        rho_init: 1.0,
        u_init: [0.0; 3],
        force: [0.0; 3],
        substeps: 1,
        execution,
        boundaries: FluidBoundaryConfig::default(),
    }
}

pub struct LbmSolverScalar {
    pub domain_config: FluidDomainConfig,
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
    f: Vec<f64>,
    f_scratch: Vec<f64>,
    rho: Vec<f64>,
    u: Vec<[f64; 3]>,
    tau_field: Vec<f64>,
    force_field: Option<Vec<[f64; 3]>>,
    solid: Vec<bool>,
    pub freestream_velocity: [f64; 3],
    pub freestream_density: f64,
    pub timestep: usize,
}

impl LbmSolverScalar {
    pub fn new(nx: usize, ny: usize, nz: usize, tau: f32) -> Self {
        let n = nx * ny * nz;
        let mut solver = Self {
            domain_config: make_domain_config(
                nx,
                ny,
                nz,
                tau,
                FluidExecutionConfig {
                    cpu_flavor: CpuKernelFlavor::Scalar,
                    ..Default::default()
                },
            ),
            nx,
            ny,
            nz,
            f: vec![0.0; n * 19],
            f_scratch: vec![0.0; n * 19],
            rho: vec![1.0; n],
            u: vec![[0.0; 3]; n],
            tau_field: vec![tau as f64; n],
            force_field: None,
            solid: vec![false; n],
            freestream_velocity: [0.0; 3],
            freestream_density: 1.0,
            timestep: 0,
        };
        solver.initialize_uniform(1.0, [0.0; 3]);
        solver
    }

    pub fn initialize_uniform(&mut self, rho: f32, u: [f32; 3]) {
        let rho = rho as f64;
        let u = [u[0] as f64, u[1] as f64, u[2] as f64];
        let eq = Self::equilibrium(rho, u);
        for idx in 0..self.nx * self.ny * self.nz {
            self.rho[idx] = rho;
            self.u[idx] = u;
            let base = idx * 19;
            self.f[base..base + 19].copy_from_slice(&eq);
        }
        self.domain_config.rho_init = rho as f32;
        self.domain_config.u_init = [u[0] as f32, u[1] as f32, u[2] as f32];
        self.freestream_velocity = u;
        self.freestream_density = rho;
    }

    fn equilibrium(rho: f64, u: [f64; 3]) -> [f64; 19] {
        let mut out = [0.0; 19];
        let u_sq = u[0] * u[0] + u[1] * u[1] + u[2] * u[2];
        let inv_cs_sq = 1.0 / (CS_SQ as f64);
        let inv_2cs4 = inv_cs_sq * inv_cs_sq * 0.5;
        let half_inv_cs_sq = 0.5 * inv_cs_sq;
        for q in 0..19 {
            let c = VELOCITIES[q];
            let cu = c[0] as f64 * u[0] + c[1] as f64 * u[1] + c[2] as f64 * u[2];
            let velocity_terms = cu * inv_cs_sq + cu * cu * inv_2cs4 - u_sq * half_inv_cs_sq;
            out[q] = WEIGHTS[q] as f64 * rho * (1.0 + velocity_terms);
        }
        out
    }

    fn linearize(&self, x: usize, y: usize, z: usize) -> usize {
        z * (self.nx * self.ny) + y * self.nx + x
    }

    pub fn compute_macroscopic(&mut self) {
        let f = &self.f;
        self.rho
            .par_iter_mut()
            .zip(self.u.par_iter_mut())
            .enumerate()
            .for_each(|(idx, (rho_out, u_out))| {
                let base = idx * 19;
                let mut rho = 0.0f64;
                let mut ux = 0.0f64;
                let mut uy = 0.0f64;
                let mut uz = 0.0f64;
                for q in 0..19 {
                    let fq = f[base + q];
                    rho += fq;
                    ux += fq * VELOCITIES[q][0] as f64;
                    uy += fq * VELOCITIES[q][1] as f64;
                    uz += fq * VELOCITIES[q][2] as f64;
                }
                *rho_out = rho;
                if rho.abs() > 1e-12 {
                    *u_out = [ux / rho, uy / rho, uz / rho];
                } else {
                    *u_out = [0.0; 3];
                }
            });
    }

    fn phase1_collision(&mut self) {
        self.compute_macroscopic();
        let rho = &self.rho;
        let u = &self.u;
        let tau_field = &self.tau_field;
        let force_field = &self.force_field;

        self.f
            .par_chunks_mut(19)
            .enumerate()
            .for_each(|(idx, f_chunk)| {
                let rho_local = rho[idx];
                let u_local = u[idx];
                let tau = tau_field[idx];
                let eq = Self::equilibrium(rho_local, u_local);
                for q in 0..19 {
                    f_chunk[q] -= (f_chunk[q] - eq[q]) / tau;
                }

                if let Some(force_field) = force_field {
                    let force = force_field[idx];
                    let prefactor = 1.0 - 1.0 / (2.0 * tau);
                    for q in 0..19 {
                        let c = VELOCITIES[q];
                        let ei = [c[0] as f64, c[1] as f64, c[2] as f64];
                        let ei_minus_u_dot_f = (ei[0] - u_local[0]) * force[0]
                            + (ei[1] - u_local[1]) * force[1]
                            + (ei[2] - u_local[2]) * force[2];
                        let ei_dot_u = ei[0] * u_local[0] + ei[1] * u_local[1] + ei[2] * u_local[2];
                        let ei_dot_f = ei[0] * force[0] + ei[1] * force[1] + ei[2] * force[2];
                        let s_i = ei_minus_u_dot_f * 3.0 + (ei_dot_u * ei_dot_f) * 9.0;
                        f_chunk[q] += prefactor * WEIGHTS[q] as f64 * s_i;
                    }
                }
            });
    }

    fn phase2_streaming(&mut self) {
        let nx = self.nx;
        let ny = self.ny;
        let nz = self.nz;
        let f_src = &self.f;

        for z in 0..nz {
            for y in 0..ny {
                for x in 0..nx {
                    let dst_idx = z * (nx * ny) + y * nx + x;
                    let dst_base = dst_idx * 19;
                    for q in 0..19 {
                        let c = VELOCITIES[q];
                        let sx = (x as i32 - c[0]).rem_euclid(nx as i32) as usize;
                        let sy = (y as i32 - c[1]).rem_euclid(ny as i32) as usize;
                        let sz = (z as i32 - c[2]).rem_euclid(nz as i32) as usize;
                        let src_idx = sz * (nx * ny) + sy * nx + sx;
                        self.f_scratch[dst_base + q] = f_src[src_idx * 19 + q];
                    }
                }
            }
        }

        std::mem::swap(&mut self.f, &mut self.f_scratch);
        self.compute_macroscopic();
        self.timestep += 1;
    }

    fn apply_solid_bounce_back(&mut self) {
        for idx in 0..self.nx * self.ny * self.nz {
            if !self.solid[idx] {
                continue;
            }
            let base = idx * 19;
            for (q, &opp) in OPPOSITE.iter().enumerate().skip(1) {
                if q < opp {
                    self.f.swap(base + q, base + opp);
                }
            }
        }
    }

    fn apply_bounce_back_planes(&mut self) {
        let nx = self.nx;
        let ny = self.ny;
        let nz = self.nz;

        for z in 0..nz {
            for x in 0..nx {
                let min_y = z * (nx * ny) + x;
                let max_y = z * (nx * ny) + (ny - 1) * nx + x;
                for &(q, opp) in &[(3, 4), (7, 8), (10, 9), (15, 16), (17, 18)] {
                    self.f[min_y * 19 + q] = self.f[min_y * 19 + opp];
                    self.f[max_y * 19 + opp] = self.f[max_y * 19 + q];
                }
            }
        }

        for y in 0..ny {
            for x in 0..nx {
                let min_z = y * nx + x;
                let max_z = (nz - 1) * (nx * ny) + y * nx + x;
                for &(q, opp) in &[(5, 6), (11, 12), (14, 13), (15, 16), (18, 17)] {
                    self.f[min_z * 19 + q] = self.f[min_z * 19 + opp];
                    self.f[max_z * 19 + opp] = self.f[max_z * 19 + q];
                }
            }
        }
    }

    fn apply_inlet_outlet_bc(&mut self) {
        let inlet_eq = Self::equilibrium(self.freestream_density, self.freestream_velocity);
        for z in 1..self.nz - 1 {
            for y in 1..self.ny - 1 {
                let inlet = self.linearize(0, y, z);
                let outlet = self.linearize(self.nx - 1, y, z);
                let source = self.linearize(self.nx - 2, y, z);
                self.f[inlet * 19..inlet * 19 + 19].copy_from_slice(&inlet_eq);

                let mut local_rho = 0.0f64;
                for q in 0..19 {
                    local_rho += self.f[source * 19 + q];
                }
                let scale = if local_rho.abs() > 1e-12 {
                    self.freestream_density / local_rho
                } else {
                    1.0
                };
                for q in 0..19 {
                    self.f[outlet * 19 + q] = self.f[source * 19 + q] * scale;
                }
            }
        }
    }

    pub fn set_solid_mask(&mut self, solid_mask: &[bool]) {
        assert_eq!(solid_mask.len(), self.nx * self.ny * self.nz);
        self.solid.copy_from_slice(solid_mask);
        self.apply_solid_bounce_back();
    }

    pub fn inject_boundary_from_mask(&mut self, voxels: &VoxelMask3) {
        assert_eq!(voxels.nx, self.nx);
        assert_eq!(voxels.ny, self.ny);
        assert_eq!(voxels.nz, self.nz);
        self.set_solid_mask(&voxels.cells);
    }

    pub fn set_force_field_f64(
        &mut self,
        force_field: Vec<[f64; 3]>,
    ) -> Result<(), FluidBackendError> {
        let expected_len = self.nx * self.ny * self.nz;
        if force_field.len() != expected_len {
            return Err(FluidBackendError::InvalidConfig(format!(
                "force field length mismatch: expected {expected_len}, got {}",
                force_field.len()
            )));
        }
        for &[fx, fy, fz] in &force_field {
            if !fx.is_finite() || !fy.is_finite() || !fz.is_finite() {
                return Err(FluidBackendError::InvalidConfig(
                    "force field contains non-finite values".to_string(),
                ));
            }
        }
        self.force_field = Some(force_field);
        Ok(())
    }

    pub fn evolve_with_boundaries(&mut self, substeps: usize) {
        self.domain_config.substeps = substeps;
        let has_solids = self.solid.iter().any(|&solid| solid);
        let has_freestream = self.freestream_velocity.iter().any(|v| v.abs() > 1e-12);
        for _ in 0..substeps {
            self.phase1_collision();
            self.phase2_streaming();
            if has_solids {
                self.apply_solid_bounce_back();
            }
            if has_freestream {
                self.apply_inlet_outlet_bc();
                self.apply_bounce_back_planes();
            }
        }
        if substeps > 0 {
            self.compute_macroscopic();
        }
    }

    pub fn get_macroscopic(&self, x: usize, y: usize, z: usize) -> (f32, [f32; 3]) {
        let idx = self.linearize(x, y, z);
        (
            self.rho[idx] as f32,
            [
                self.u[idx][0] as f32,
                self.u[idx][1] as f32,
                self.u[idx][2] as f32,
            ],
        )
    }

    pub fn read_density_field(&self) -> Vec<f32> {
        self.rho.iter().map(|value| *value as f32).collect()
    }

    pub fn read_velocity_field(&self) -> Vec<f32> {
        let mut out = Vec::with_capacity(self.nx * self.ny * self.nz * 3);
        for velocity in &self.u {
            out.push(velocity[0] as f32);
            out.push(velocity[1] as f32);
            out.push(velocity[2] as f32);
        }
        out
    }

    pub fn total_mass(&self) -> f64 {
        self.rho.iter().sum()
    }

    pub fn max_velocity(&self) -> f64 {
        self.u
            .iter()
            .map(|velocity| {
                (velocity[0] * velocity[0] + velocity[1] * velocity[1] + velocity[2] * velocity[2])
                    .sqrt()
            })
            .fold(0.0, f64::max)
    }

    pub fn mean_velocity(&self) -> f64 {
        let sum = self
            .u
            .iter()
            .map(|velocity| {
                (velocity[0] * velocity[0] + velocity[1] * velocity[1] + velocity[2] * velocity[2])
                    .sqrt()
            })
            .sum::<f64>();
        sum / self.u.len() as f64
    }

    pub fn is_stable(&self) -> bool {
        self.f
            .iter()
            .all(|value| *value >= -1e-12 && value.is_finite())
    }

    pub fn compute_drag_lift_from_mask(&self, solid_mask: &[bool]) -> (f64, f64) {
        assert_eq!(solid_mask.len(), self.nx * self.ny * self.nz);
        let mut drag = 0.0f64;
        let mut lift = 0.0f64;
        for z in 0..self.nz {
            for y in 0..self.ny {
                for x in 0..self.nx {
                    let idx = self.linearize(x, y, z);
                    if !solid_mask[idx] {
                        continue;
                    }
                    for q in 1..19 {
                        let c = VELOCITIES[q];
                        let nx_i = (x as i32 + c[0]).rem_euclid(self.nx as i32) as usize;
                        let ny_i = (y as i32 + c[1]).rem_euclid(self.ny as i32) as usize;
                        let nz_i = (z as i32 + c[2]).rem_euclid(self.nz as i32) as usize;
                        let neighbor_idx = self.linearize(nx_i, ny_i, nz_i);
                        if solid_mask[neighbor_idx] {
                            continue;
                        }
                        let opp = OPPOSITE[q];
                        let f_i = self.f[neighbor_idx * 19 + q];
                        let f_opp = self.f[idx * 19 + opp];
                        let momentum = f_i + f_opp;
                        drag += momentum * c[0] as f64;
                        lift += momentum * c[1] as f64;
                    }
                }
            }
        }
        (drag, lift)
    }
}

pub enum LocalCpuFluidKernel {
    SoA(LbmSolverSoA),
    Scalar(LbmSolverScalar),
}

impl LocalCpuFluidKernel {
    pub fn from_domain_config(config: &FluidDomainConfig) -> Self {
        match config.execution.cpu_flavor {
            CpuKernelFlavor::SoA => {
                let mut solver =
                    LbmSolverSoA::new(config.grid.nx, config.grid.ny, config.grid.nz, config.tau);
                solver.initialize_uniform(config.rho_init, config.u_init);
                solver.domain_config.execution = config.execution;
                solver.domain_config.boundaries = config.boundaries;
                solver.domain_config.force = config.force;
                Self::SoA(solver)
            }
            CpuKernelFlavor::Scalar => {
                let mut solver = LbmSolverScalar::new(
                    config.grid.nx,
                    config.grid.ny,
                    config.grid.nz,
                    config.tau,
                );
                solver.initialize_uniform(config.rho_init, config.u_init);
                solver.domain_config.execution = config.execution;
                solver.domain_config.boundaries = config.boundaries;
                solver.domain_config.force = config.force;
                Self::Scalar(solver)
            }
        }
    }
}

pub struct LbmSolverSoA {
    pub domain_config: FluidDomainConfig,
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
    h: Box<[Vec<f32>; 19]>,
    h_scratch: Box<[Vec<f32>; 19]>,
    pub drho: Vec<f32>,
    pub ux: Vec<f32>,
    pub uy: Vec<f32>,
    pub uz: Vec<f32>,
    pub tau: f32,
    pub solid: Vec<bool>,
    pub freestream_velocity: [f32; 3],
    pub freestream_density: f32,
    pub timestep: usize,
    neighbor: NeighborTable,
}

impl LbmSolverSoA {
    pub fn new(nx: usize, ny: usize, nz: usize, tau: f32) -> Self {
        let n = nx * ny * nz;
        let make_arrays = || Box::new(std::array::from_fn::<Vec<f32>, 19, _>(|_| vec![0.0; n]));
        Self {
            domain_config: make_domain_config(
                nx,
                ny,
                nz,
                tau,
                FluidExecutionConfig {
                    cpu_flavor: CpuKernelFlavor::SoA,
                    ..Default::default()
                },
            ),
            nx,
            ny,
            nz,
            h: make_arrays(),
            h_scratch: make_arrays(),
            drho: vec![0.0; n],
            ux: vec![0.0; n],
            uy: vec![0.0; n],
            uz: vec![0.0; n],
            tau,
            solid: vec![false; n],
            freestream_velocity: [0.0; 3],
            freestream_density: 1.0,
            timestep: 0,
            neighbor: NeighborTable::build(nx, ny, nz),
        }
    }

    pub fn initialize_uniform(&mut self, rho: f32, u: [f32; 3]) {
        let h_eq = Self::equilibrium_perturbation(rho, u);
        for (q, &heq) in h_eq.iter().enumerate() {
            for val in &mut self.h[q] {
                *val = heq;
            }
        }
        self.domain_config.rho_init = rho;
        self.domain_config.u_init = u;
        self.freestream_velocity = u;
        self.freestream_density = rho;
        self.compute_macroscopic();
    }

    #[inline(always)]
    fn equilibrium_perturbation(rho: f32, u: [f32; 3]) -> [f32; 19] {
        let u_sq = u[0] * u[0] + u[1] * u[1] + u[2] * u[2];
        let inv_cs_sq = 1.0 / CS_SQ;
        let inv_2cs4 = inv_cs_sq * inv_cs_sq * 0.5;
        let half_inv_cs_sq = 0.5 * inv_cs_sq;
        let rho_m1 = rho - 1.0;

        let mut h_eq = [0.0; 19];
        for q in 0..19 {
            let c = VELOCITIES[q];
            let cu = c[0] as f32 * u[0] + c[1] as f32 * u[1] + c[2] as f32 * u[2];
            let velocity_terms = cu * inv_cs_sq + cu * cu * inv_2cs4 - u_sq * half_inv_cs_sq;
            h_eq[q] = WEIGHTS[q] * (rho_m1 + rho * velocity_terms);
        }
        h_eq
    }

    pub fn compute_macroscopic(&mut self) {
        let n = self.nx * self.ny * self.nz;
        let h = &self.h;

        let mut drho_buf = std::mem::take(&mut self.drho);
        let mut ux_buf = std::mem::take(&mut self.ux);
        let mut uy_buf = std::mem::take(&mut self.uy);
        let mut uz_buf = std::mem::take(&mut self.uz);

        drho_buf
            .par_iter_mut()
            .zip(ux_buf.par_iter_mut())
            .zip(uy_buf.par_iter_mut())
            .zip(uz_buf.par_iter_mut())
            .enumerate()
            .for_each(|(idx, (((dr, vx), vy), vz))| {
                if idx >= n {
                    return;
                }
                let mut sum_h = 0.0f32;
                let mut sum_cx = 0.0f32;
                let mut sum_cy = 0.0f32;
                let mut sum_cz = 0.0f32;
                for q in 0..19 {
                    let hq = h[q][idx];
                    sum_h += hq;
                    sum_cx += hq * VELOCITIES[q][0] as f32;
                    sum_cy += hq * VELOCITIES[q][1] as f32;
                    sum_cz += hq * VELOCITIES[q][2] as f32;
                }
                let rho = 1.0 + sum_h;
                *dr = sum_h;
                if rho.abs() > 1e-12 {
                    *vx = sum_cx / rho;
                    *vy = sum_cy / rho;
                    *vz = sum_cz / rho;
                } else {
                    *vx = 0.0;
                    *vy = 0.0;
                    *vz = 0.0;
                }
            });

        self.drho = drho_buf;
        self.ux = ux_buf;
        self.uy = uy_buf;
        self.uz = uz_buf;
    }

    fn fused_stream_collide(&mut self) {
        let n = self.nx * self.ny * self.nz;
        let tau = self.tau;
        let omega = 1.0 / tau;
        let h_in = &self.h;
        let neighbor = &self.neighbor.table;

        self.h_scratch
            .iter_mut()
            .for_each(|buffer| buffer.resize(n, 0.0));

        let mut h_out = std::mem::replace(
            &mut self.h_scratch,
            Box::new(std::array::from_fn(|_| Vec::<f32>::new())),
        );

        h_out.par_iter_mut().enumerate().for_each(|(q, out_q)| {
            out_q.resize(n, 0.0);
            for (dst_idx, out_val) in out_q.iter_mut().enumerate() {
                let src_idx = neighbor[q][dst_idx] as usize;

                let mut local_drho = 0.0f32;
                let mut local_ux = 0.0f32;
                let mut local_uy = 0.0f32;
                let mut local_uz = 0.0f32;
                for qq in 0..19 {
                    let hqq = h_in[qq][src_idx];
                    local_drho += hqq;
                    local_ux += hqq * VELOCITIES[qq][0] as f32;
                    local_uy += hqq * VELOCITIES[qq][1] as f32;
                    local_uz += hqq * VELOCITIES[qq][2] as f32;
                }

                let rho = 1.0 + local_drho;
                let u = if rho.abs() > 1e-12 {
                    [local_ux / rho, local_uy / rho, local_uz / rho]
                } else {
                    [0.0, 0.0, 0.0]
                };

                let h_eq = Self::equilibrium_perturbation(rho, u);
                let h_streamed = h_in[q][src_idx];
                *out_val = h_streamed - omega * (h_streamed - h_eq[q]);
            }
        });

        let old_h = std::mem::replace(&mut self.h, h_out);
        self.h_scratch = old_h;
        self.timestep += 1;
    }

    fn apply_solid_bounce_back(&mut self) {
        let n = self.nx * self.ny * self.nz;
        for idx in 0..n {
            if !self.solid[idx] {
                continue;
            }
            for (q, &opp) in OPPOSITE.iter().enumerate().skip(1) {
                if q < opp {
                    let tmp = self.h[q][idx];
                    self.h[q][idx] = self.h[opp][idx];
                    self.h[opp][idx] = tmp;
                }
            }
        }
    }

    fn apply_bounce_back_planes(&mut self) {
        let nx = self.nx;
        let ny = self.ny;
        let nz = self.nz;

        for z in 0..nz {
            for x in 0..nx {
                let min_y = z * (nx * ny) + x;
                let max_y = z * (nx * ny) + (ny - 1) * nx + x;
                for &(q, opp) in &[(3, 4), (7, 8), (10, 9), (15, 16), (17, 18)] {
                    self.h[q][min_y] = self.h[opp][min_y];
                    self.h[opp][max_y] = self.h[q][max_y];
                }
            }
        }

        for y in 0..ny {
            for x in 0..nx {
                let min_z = y * nx + x;
                let max_z = (nz - 1) * (nx * ny) + y * nx + x;
                for &(q, opp) in &[(5, 6), (11, 12), (14, 13), (15, 16), (18, 17)] {
                    self.h[q][min_z] = self.h[opp][min_z];
                    self.h[opp][max_z] = self.h[q][max_z];
                }
            }
        }
    }

    fn apply_inlet_outlet_bc(&mut self) {
        let nx = self.nx;
        let ny = self.ny;
        let nz = self.nz;
        let rho0 = self.freestream_density;
        let h_eq_inlet = Self::equilibrium_perturbation(rho0, self.freestream_velocity);

        for z in 1..nz - 1 {
            for y in 1..ny - 1 {
                let idx = z * (nx * ny) + y * nx;
                for (q, &heq) in h_eq_inlet.iter().enumerate() {
                    self.h[q][idx] = heq;
                }
            }
        }

        for z in 1..nz - 1 {
            for y in 1..ny - 1 {
                let src_idx = z * (nx * ny) + y * nx + (nx - 2);
                let dst_idx = z * (nx * ny) + y * nx + (nx - 1);

                let mut local_drho = 0.0f32;
                for q in 0..19 {
                    local_drho += self.h[q][src_idx];
                }
                let local_rho = 1.0 + local_drho;
                let scale = if local_rho.abs() > 1e-12 {
                    rho0 / local_rho
                } else {
                    1.0
                };

                for (q, &w) in WEIGHTS.iter().enumerate() {
                    let f_src = self.h[q][src_idx] + w;
                    self.h[q][dst_idx] = f_src * scale - w;
                }
            }
        }
    }

    pub fn set_solid_mask(&mut self, solid_mask: &[bool]) {
        assert_eq!(solid_mask.len(), self.nx * self.ny * self.nz);
        self.solid.copy_from_slice(solid_mask);
        self.apply_solid_bounce_back();
    }

    pub fn inject_boundary_from_mask(&mut self, voxels: &VoxelMask3) {
        assert_eq!(voxels.nx, self.nx);
        assert_eq!(voxels.ny, self.ny);
        assert_eq!(voxels.nz, self.nz);
        self.set_solid_mask(&voxels.cells);
    }

    pub fn evolve_with_boundaries(&mut self, substeps: usize) {
        self.domain_config.substeps = substeps;
        let has_solids = self.solid.iter().any(|&s| s);
        let has_freestream = self.freestream_velocity.iter().any(|v| v.abs() > 1e-12);

        for _ in 0..substeps {
            self.fused_stream_collide();
            if has_solids {
                self.apply_solid_bounce_back();
            }
            if has_freestream {
                self.apply_inlet_outlet_bc();
                self.apply_bounce_back_planes();
            }
        }

        if substeps > 0 {
            self.compute_macroscopic();
        }
    }

    pub fn get_macroscopic(&self, x: usize, y: usize, z: usize) -> (f32, [f32; 3]) {
        let idx = z * (self.nx * self.ny) + y * self.nx + x;
        (
            1.0 + self.drho[idx],
            [self.ux[idx], self.uy[idx], self.uz[idx]],
        )
    }

    pub fn read_density_field(&self) -> Vec<f32> {
        self.drho.iter().map(|dr| 1.0 + dr).collect()
    }

    pub fn read_velocity_field(&self) -> Vec<f32> {
        let n = self.nx * self.ny * self.nz;
        let mut out = Vec::with_capacity(n * 3);
        for idx in 0..n {
            out.push(self.ux[idx]);
            out.push(self.uy[idx]);
            out.push(self.uz[idx]);
        }
        out
    }

    pub fn total_mass(&self) -> f64 {
        let n = self.nx * self.ny * self.nz;
        n as f64 + self.drho.iter().map(|&d| d as f64).sum::<f64>()
    }

    pub fn max_velocity(&self) -> f64 {
        let mut max_sq = 0.0f64;
        for idx in 0..self.nx * self.ny * self.nz {
            let vx = self.ux[idx] as f64;
            let vy = self.uy[idx] as f64;
            let vz = self.uz[idx] as f64;
            let sq = vx * vx + vy * vy + vz * vz;
            if sq > max_sq {
                max_sq = sq;
            }
        }
        max_sq.sqrt()
    }

    pub fn mean_velocity(&self) -> f64 {
        let n = self.nx * self.ny * self.nz;
        let mut sum = 0.0f64;
        for idx in 0..n {
            let vx = self.ux[idx] as f64;
            let vy = self.uy[idx] as f64;
            let vz = self.uz[idx] as f64;
            sum += (vx * vx + vy * vy + vz * vz).sqrt();
        }
        sum / n as f64
    }

    pub fn is_stable(&self) -> bool {
        for (q, &w) in WEIGHTS.iter().enumerate() {
            for &h in &self.h[q] {
                if h + w < -1e-6 {
                    return false;
                }
            }
        }
        true
    }

    pub fn compute_drag_lift_from_mask(&self, solid_mask: &[bool]) -> (f64, f64) {
        assert_eq!(solid_mask.len(), self.nx * self.ny * self.nz);
        let nx = self.nx;
        let ny = self.ny;
        let nz = self.nz;
        let mut drag = 0.0f64;
        let mut lift = 0.0f64;

        for z in 0..nz {
            for y in 0..ny {
                for x in 0..nx {
                    let idx = z * (nx * ny) + y * nx + x;
                    if !solid_mask[idx] {
                        continue;
                    }
                    for q in 1..19 {
                        let c = VELOCITIES[q];
                        let nx_i = (x as i32 + c[0]).rem_euclid(nx as i32) as usize;
                        let ny_i = (y as i32 + c[1]).rem_euclid(ny as i32) as usize;
                        let nz_i = (z as i32 + c[2]).rem_euclid(nz as i32) as usize;
                        let neighbor_idx = nz_i * (nx * ny) + ny_i * nx + nx_i;
                        if solid_mask[neighbor_idx] {
                            continue;
                        }
                        let opp = OPPOSITE[q];
                        let f_i = (self.h[q][neighbor_idx] + WEIGHTS[q]) as f64;
                        let f_opp = (self.h[opp][idx] + WEIGHTS[opp]) as f64;
                        let momentum = f_i + f_opp;
                        drag += momentum * c[0] as f64;
                        lift += momentum * c[1] as f64;
                    }
                }
            }
        }
        (drag, lift)
    }

    pub fn compute_drag_lift(&self, voxels: &VoxelMask3) -> (f64, f64) {
        assert_eq!(voxels.nx, self.nx);
        assert_eq!(voxels.ny, self.ny);
        assert_eq!(voxels.nz, self.nz);
        self.compute_drag_lift_from_mask(&voxels.cells)
    }
}

impl FluidKernel for LbmSolverSoA {
    fn backend(&self) -> KernelBackend {
        KernelBackend::Cpu
    }

    fn domain_config(&self) -> &FluidDomainConfig {
        &self.domain_config
    }

    fn selected_backend(&self) -> FluidBackendKind {
        FluidBackendKind::CpuSoA
    }

    fn capabilities(&self) -> FluidBackendCapabilities {
        FluidBackendCapabilities::cpu_only_detected()
    }

    fn set_voxel_mask(&mut self, solid_mask: &[bool]) {
        self.set_solid_mask(solid_mask);
    }

    fn set_force_field(&mut self, force_xyz: &[[f32; 3]]) -> Result<(), FluidBackendError> {
        if force_xyz
            .iter()
            .all(|force| force.iter().all(|component| component.abs() <= 1e-12))
        {
            self.domain_config.force = [0.0; 3];
            return Ok(());
        }
        Err(FluidBackendError::UnsupportedBackend(
            "SoA CPU kernel does not yet support nonzero force-field injection",
        ))
    }

    fn step(&mut self, substeps: usize) {
        self.evolve_with_boundaries(substeps);
    }

    fn diagnostics(&self) -> FluidDiagnosticsSnapshot {
        FluidDiagnosticsSnapshot {
            backend: KernelBackend::Cpu,
            timestep: self.timestep,
            total_mass: self.total_mass(),
            max_velocity: self.max_velocity(),
            mean_velocity: self.mean_velocity(),
            stable: self.is_stable(),
        }
    }

    fn field_snapshot(&self) -> FluidFieldSnapshot {
        FluidFieldSnapshot {
            density: self.read_density_field(),
            velocity_xyz: self.read_velocity_field(),
        }
    }

    fn aerodynamic_snapshot(
        &self,
        solid_mask: &[bool],
        freestream_velocity: [f32; 3],
        tau: f32,
    ) -> AerodynamicSnapshot {
        let (drag, lift) = self.compute_drag_lift_from_mask(solid_mask);
        let u_mag = freestream_velocity
            .iter()
            .map(|v| (*v as f64) * (*v as f64))
            .sum::<f64>()
            .sqrt();
        let area = solid_mask.iter().filter(|&&solid| solid).count() as f64;
        let nu = ((tau as f64) - 0.5) / 3.0;
        let characteristic_length = (self.nx as f64).cbrt() * 10.0;

        let mut snapshot = AerodynamicSnapshot {
            drag,
            lift,
            ..Default::default()
        };
        if u_mag > 1e-12 && area > 0.0 {
            snapshot.drag_coefficient =
                2.0 * drag.abs() / (self.freestream_density as f64 * u_mag * u_mag * area);
            snapshot.lift_coefficient =
                2.0 * lift.abs() / (self.freestream_density as f64 * u_mag * u_mag * area);
        }
        if nu > 1e-12 {
            snapshot.reynolds_number = u_mag * characteristic_length / nu;
        }
        snapshot
    }
}

impl FluidKernel for LbmSolverScalar {
    fn backend(&self) -> KernelBackend {
        KernelBackend::Cpu
    }

    fn domain_config(&self) -> &FluidDomainConfig {
        &self.domain_config
    }

    fn selected_backend(&self) -> FluidBackendKind {
        FluidBackendKind::CpuScalar
    }

    fn capabilities(&self) -> FluidBackendCapabilities {
        FluidBackendCapabilities::cpu_only_detected()
    }

    fn set_voxel_mask(&mut self, solid_mask: &[bool]) {
        self.set_solid_mask(solid_mask);
    }

    fn set_force_field(&mut self, force_xyz: &[[f32; 3]]) -> Result<(), FluidBackendError> {
        let field = force_xyz
            .iter()
            .map(|force| [force[0] as f64, force[1] as f64, force[2] as f64])
            .collect();
        self.set_force_field_f64(field)
    }

    fn step(&mut self, substeps: usize) {
        self.evolve_with_boundaries(substeps);
    }

    fn diagnostics(&self) -> FluidDiagnosticsSnapshot {
        FluidDiagnosticsSnapshot {
            backend: KernelBackend::Cpu,
            timestep: self.timestep,
            total_mass: self.total_mass(),
            max_velocity: self.max_velocity(),
            mean_velocity: self.mean_velocity(),
            stable: self.is_stable(),
        }
    }

    fn field_snapshot(&self) -> FluidFieldSnapshot {
        FluidFieldSnapshot {
            density: self.read_density_field(),
            velocity_xyz: self.read_velocity_field(),
        }
    }

    fn aerodynamic_snapshot(
        &self,
        solid_mask: &[bool],
        freestream_velocity: [f32; 3],
        tau: f32,
    ) -> AerodynamicSnapshot {
        let (drag, lift) = self.compute_drag_lift_from_mask(solid_mask);
        let u_mag = freestream_velocity
            .iter()
            .map(|v| (*v as f64) * (*v as f64))
            .sum::<f64>()
            .sqrt();
        let area = solid_mask.iter().filter(|&&solid| solid).count() as f64;
        let nu = ((tau as f64) - 0.5) / 3.0;
        let characteristic_length = (self.nx as f64).cbrt() * 10.0;

        let mut snapshot = AerodynamicSnapshot {
            drag,
            lift,
            ..Default::default()
        };
        if u_mag > 1e-12 && area > 0.0 {
            snapshot.drag_coefficient =
                2.0 * drag.abs() / (self.freestream_density * u_mag * u_mag * area);
            snapshot.lift_coefficient =
                2.0 * lift.abs() / (self.freestream_density * u_mag * u_mag * area);
        }
        if nu > 1e-12 {
            snapshot.reynolds_number = u_mag * characteristic_length / nu;
        }
        snapshot
    }
}

impl FluidKernel for LocalCpuFluidKernel {
    fn backend(&self) -> KernelBackend {
        KernelBackend::Cpu
    }

    fn domain_config(&self) -> &FluidDomainConfig {
        match self {
            Self::SoA(solver) => solver.domain_config(),
            Self::Scalar(solver) => solver.domain_config(),
        }
    }

    fn selected_backend(&self) -> FluidBackendKind {
        match self {
            Self::SoA(solver) => solver.selected_backend(),
            Self::Scalar(solver) => solver.selected_backend(),
        }
    }

    fn capabilities(&self) -> FluidBackendCapabilities {
        match self {
            Self::SoA(solver) => solver.capabilities(),
            Self::Scalar(solver) => solver.capabilities(),
        }
    }

    fn set_voxel_mask(&mut self, solid_mask: &[bool]) {
        match self {
            Self::SoA(solver) => solver.set_voxel_mask(solid_mask),
            Self::Scalar(solver) => solver.set_voxel_mask(solid_mask),
        }
    }

    fn set_force_field(&mut self, force_xyz: &[[f32; 3]]) -> Result<(), FluidBackendError> {
        match self {
            Self::SoA(solver) => solver.set_force_field(force_xyz),
            Self::Scalar(solver) => solver.set_force_field(force_xyz),
        }
    }

    fn step(&mut self, substeps: usize) {
        match self {
            Self::SoA(solver) => solver.step(substeps),
            Self::Scalar(solver) => solver.step(substeps),
        }
    }

    fn diagnostics(&self) -> FluidDiagnosticsSnapshot {
        match self {
            Self::SoA(solver) => solver.diagnostics(),
            Self::Scalar(solver) => solver.diagnostics(),
        }
    }

    fn field_snapshot(&self) -> FluidFieldSnapshot {
        match self {
            Self::SoA(solver) => solver.field_snapshot(),
            Self::Scalar(solver) => solver.field_snapshot(),
        }
    }

    fn aerodynamic_snapshot(
        &self,
        solid_mask: &[bool],
        freestream_velocity: [f32; 3],
        tau: f32,
    ) -> AerodynamicSnapshot {
        match self {
            Self::SoA(solver) => solver.aerodynamic_snapshot(solid_mask, freestream_velocity, tau),
            Self::Scalar(solver) => {
                solver.aerodynamic_snapshot(solid_mask, freestream_velocity, tau)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gororoba_kernel_api::fluid::FluidBackendKind;

    fn make_solver(nx: usize, ny: usize, nz: usize) -> LbmSolverSoA {
        let mut s = LbmSolverSoA::new(nx, ny, nz, 0.8);
        s.initialize_uniform(1.0, [0.05, 0.0, 0.0]);
        s
    }

    fn make_scalar_solver(nx: usize, ny: usize, nz: usize) -> LbmSolverScalar {
        let mut s = LbmSolverScalar::new(nx, ny, nz, 0.8);
        s.initialize_uniform(1.0, [0.05, 0.0, 0.0]);
        s
    }

    #[test]
    fn voxel_mask_tracks_solid_cells() {
        let mut mask = VoxelMask3::new(4, 4, 4);
        mask.set(1, 2, 3, true);
        assert!(mask.get(1, 2, 3));
        assert_eq!(mask.solid_count(), 1);
    }

    #[test]
    fn soa_rest_initialization() {
        let s = LbmSolverSoA::new(8, 8, 8, 0.8);
        for q in 0..19 {
            for &val in &s.h[q] {
                assert!(val.abs() < 1e-12, "h[{q}] not zero at rest: {val}");
            }
        }
    }

    #[test]
    fn soa_weights_sum_to_one() {
        let sum: f32 = WEIGHTS.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn soa_opposite_symmetric() {
        for q in 0..19 {
            assert_eq!(OPPOSITE[OPPOSITE[q]], q);
        }
    }

    #[test]
    fn soa_neighbor_table_consistent() {
        let (nx, ny, nz) = (16, 8, 8);
        let nt = NeighborTable::build(nx, ny, nz);
        for z in 0..nz {
            for y in 0..ny {
                for x in 0..nx {
                    let dst_idx = z * (nx * ny) + y * nx + x;
                    for (q, &[cx, cy, cz]) in VELOCITIES.iter().enumerate() {
                        let sx = ((x as i32 - cx).rem_euclid(nx as i32)) as usize;
                        let sy = ((y as i32 - cy).rem_euclid(ny as i32)) as usize;
                        let sz = ((z as i32 - cz).rem_euclid(nz as i32)) as usize;
                        let expected = sz * (nx * ny) + sy * nx + sx;
                        assert_eq!(nt.table[q][dst_idx] as usize, expected);
                    }
                }
            }
        }
    }

    #[test]
    fn soa_freestream_sustained() {
        let mut s = make_solver(32, 16, 16);
        s.evolve_with_boundaries(200);
        let (rho, u) = s.get_macroscopic(4, 8, 8);
        let u_mag = (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt();
        assert!(rho.is_finite());
        assert!(
            u_mag > 0.01,
            "Expected sustained freestream, got |u|={u_mag}"
        );
    }

    #[test]
    fn soa_drag_with_obstacle() {
        let mut s = make_solver(32, 16, 16);
        let mut voxels = VoxelMask3::new(32, 16, 16);
        for x in 12..16 {
            for y in 6..10 {
                for z in 6..10 {
                    voxels.set(x, y, z, true);
                }
            }
        }
        s.inject_boundary_from_mask(&voxels);
        s.evolve_with_boundaries(600);
        let (drag, _lift) = s.compute_drag_lift(&voxels);
        assert!(drag.abs() > 1e-6, "Expected nonzero drag, got {drag}");
    }

    #[test]
    fn soa_long_running_stability() {
        let mut s = make_solver(32, 16, 16);
        let mut voxels = VoxelMask3::new(32, 16, 16);
        for x in 12..16 {
            for y in 6..10 {
                for z in 6..10 {
                    voxels.set(x, y, z, true);
                }
            }
        }
        s.inject_boundary_from_mask(&voxels);

        for batch in 0..30 {
            s.evolve_with_boundaries(100);
            let (rho, u) = s.get_macroscopic(4, 8, 8);
            assert!(
                rho.is_finite() && u[0].is_finite() && u[1].is_finite() && u[2].is_finite(),
                "SoA diverged at step {} (batch {batch}): rho={rho}, u={u:?}",
                (batch + 1) * 100
            );
        }
    }

    #[test]
    fn fluid_kernel_snapshots_are_available() {
        let solver = make_solver(8, 8, 8);
        let snapshot = solver.field_snapshot();
        let diag = solver.diagnostics();
        assert_eq!(snapshot.density.len(), 8 * 8 * 8);
        assert_eq!(snapshot.velocity_xyz.len(), 8 * 8 * 8 * 3);
        assert_eq!(diag.timestep, solver.timestep);
    }

    #[test]
    fn scalar_kernel_snapshots_are_available() {
        let solver = make_scalar_solver(8, 8, 8);
        let snapshot = solver.field_snapshot();
        let diag = solver.diagnostics();
        assert_eq!(snapshot.density.len(), 8 * 8 * 8);
        assert_eq!(snapshot.velocity_xyz.len(), 8 * 8 * 8 * 3);
        assert_eq!(diag.timestep, solver.timestep);
    }

    #[test]
    fn scalar_force_field_is_accepted() {
        let mut solver = make_scalar_solver(4, 4, 4);
        let force = vec![[0.0, 0.0, 1e-4]; 4 * 4 * 4];
        solver.set_force_field(&force).unwrap();
        solver.step(2);
        let diag = solver.diagnostics();
        assert!(diag.total_mass.is_finite());
    }

    #[test]
    fn cpu_factory_selects_requested_flavor() {
        let soa = FluidDomainConfig {
            grid: GridShape3 {
                nx: 4,
                ny: 4,
                nz: 4,
            },
            tau: 0.8,
            rho_init: 1.0,
            u_init: [0.0; 3],
            force: [0.0; 3],
            substeps: 1,
            execution: FluidExecutionConfig {
                cpu_flavor: CpuKernelFlavor::SoA,
                ..Default::default()
            },
            boundaries: FluidBoundaryConfig::default(),
        };
        let scalar = FluidDomainConfig {
            execution: FluidExecutionConfig {
                cpu_flavor: CpuKernelFlavor::Scalar,
                ..soa.execution
            },
            ..soa.clone()
        };

        let soa_kernel = LocalCpuFluidKernel::from_domain_config(&soa);
        let scalar_kernel = LocalCpuFluidKernel::from_domain_config(&scalar);
        assert_eq!(soa_kernel.selected_backend(), FluidBackendKind::CpuSoA);
        assert_eq!(
            scalar_kernel.selected_backend(),
            FluidBackendKind::CpuScalar
        );
    }
}

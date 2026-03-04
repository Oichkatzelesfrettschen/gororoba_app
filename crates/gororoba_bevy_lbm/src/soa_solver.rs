// High-performance LBM solver using Structure-of-Arrays (SoA) layout
// with f32 perturbation formulation.
//
// The perturbation shift h_i = f_i - w_i centers values around zero at
// rest (rho=1, u=0), concentrating f32 precision on the physical signal
// rather than the large equilibrium bias. Combined with SoA memory
// layout (one contiguous Vec<f32> per lattice direction) and a fused
// pull-collide streaming pass, this achieves significantly higher
// throughput than the upstream AoS f64 solver.
//
// Performance optimizations over naive implementation:
// - Pre-computed neighbor lookup table eliminates rem_euclid from hot loop
// - SoA layout enables SIMD autovectorization across cells
// - Fused stream-collide avoids separate collision pass
// - target-cpu=native enables AVX2/FMA on x86_64

use rayon::prelude::*;

use crate::components::VoxelGrid;

/// D3Q19 lattice velocity vectors.
const VELOCITIES: [[i32; 3]; 19] = [
    [0, 0, 0],   // 0: rest
    [1, 0, 0],   // 1
    [-1, 0, 0],  // 2
    [0, 1, 0],   // 3
    [0, -1, 0],  // 4
    [0, 0, 1],   // 5
    [0, 0, -1],  // 6
    [1, 1, 0],   // 7
    [-1, -1, 0], // 8
    [1, -1, 0],  // 9
    [-1, 1, 0],  // 10
    [1, 0, 1],   // 11
    [-1, 0, -1], // 12
    [1, 0, -1],  // 13
    [-1, 0, 1],  // 14
    [0, 1, 1],   // 15
    [0, -1, -1], // 16
    [0, 1, -1],  // 17
    [0, -1, 1],  // 18
];

/// D3Q19 weights.
const WEIGHTS: [f32; 19] = [
    1.0 / 3.0,  // rest
    1.0 / 18.0, // axis-aligned (6)
    1.0 / 18.0,
    1.0 / 18.0,
    1.0 / 18.0,
    1.0 / 18.0,
    1.0 / 18.0,
    1.0 / 36.0, // face diagonals (12)
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

/// Opposite direction mapping for D3Q19 bounce-back.
/// opposite(i) is the index j such that c_j = -c_i.
const OPPOSITE: [usize; 19] = [
    0,  // rest -> rest
    2,  // [1,0,0] -> [-1,0,0]
    1,  // [-1,0,0] -> [1,0,0]
    4,  // [0,1,0] -> [0,-1,0]
    3,  // [0,-1,0] -> [0,1,0]
    6,  // [0,0,1] -> [0,0,-1]
    5,  // [0,0,-1] -> [0,0,1]
    8,  // [1,1,0] -> [-1,-1,0]
    7,  // [-1,-1,0] -> [1,1,0]
    10, // [1,-1,0] -> [-1,1,0]
    9,  // [-1,1,0] -> [1,-1,0]
    12, // [1,0,1] -> [-1,0,-1]
    11, // [-1,0,-1] -> [1,0,1]
    14, // [1,0,-1] -> [-1,0,1]
    13, // [-1,0,1] -> [1,0,-1]
    16, // [0,1,1] -> [0,-1,-1]
    15, // [0,-1,-1] -> [0,1,1]
    18, // [0,1,-1] -> [0,-1,1]
    17, // [0,-1,1] -> [0,1,-1]
];

/// Speed of sound squared: c_s^2 = 1/3.
const CS_SQ: f32 = 1.0 / 3.0;

/// Pre-computed neighbor lookup table for streaming.
///
/// For each destination cell `dst_idx` and direction `q`, stores the
/// source index `src_idx = neighbor[q][dst_idx]` such that the pull
/// operation reads h[q][src_idx] into the destination. This eliminates
/// `rem_euclid` from the hot loop.
struct NeighborTable {
    /// neighbor[q][dst_idx] = src_idx for direction q.
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

/// SoA-layout LBM solver with f32 perturbation formulation.
///
/// Stores h_i = f_i - w_i per direction in separate contiguous arrays.
/// At rest (rho=1, u=0) all h values are zero, concentrating f32
/// precision on the physical perturbation signal.
pub struct LbmSolverSoA {
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
    /// Perturbation distributions: h[q][cell] where h_i = f_i - w_i.
    h: Box<[Vec<f32>; 19]>,
    /// Double buffer for streaming.
    h_scratch: Box<[Vec<f32>; 19]>,
    /// Macroscopic density perturbation: drho = rho - 1 = sum_i h_i.
    pub drho: Vec<f32>,
    /// Macroscopic velocity x-component.
    pub ux: Vec<f32>,
    /// Macroscopic velocity y-component.
    pub uy: Vec<f32>,
    /// Macroscopic velocity z-component.
    pub uz: Vec<f32>,
    /// BGK relaxation time.
    pub tau: f32,
    /// Solid cell mask for bounce-back.
    pub solid: Vec<bool>,
    /// Freestream velocity (lattice units) for inlet BC.
    pub freestream_velocity: [f32; 3],
    /// Freestream density for inlet BC.
    pub freestream_density: f32,
    /// Timestep counter.
    pub timestep: usize,
    /// Pre-computed neighbor table (eliminates rem_euclid from hot loop).
    neighbor: NeighborTable,
}

impl LbmSolverSoA {
    /// Create a new solver with the given dimensions and relaxation time.
    pub fn new(nx: usize, ny: usize, nz: usize, tau: f32) -> Self {
        let n = nx * ny * nz;
        let make_arrays = || Box::new(std::array::from_fn::<Vec<f32>, 19, _>(|_| vec![0.0f32; n]));
        Self {
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

    /// Initialize to uniform density and velocity.
    ///
    /// Sets h_i = f_i^eq(rho, u) - w_i at every fluid cell.
    pub fn initialize_uniform(&mut self, rho: f32, u: [f32; 3]) {
        let h_eq = Self::equilibrium_perturbation(rho, u);
        for (q, &heq) in h_eq.iter().enumerate() {
            for val in &mut self.h[q] {
                *val = heq;
            }
        }
        self.freestream_velocity = u;
        self.freestream_density = rho;
        self.compute_macroscopic();
    }

    /// Compute h_i^eq = f_i^eq(rho, u) - w_i.
    ///
    /// f_i^eq = rho * w_i * [1 + (c_i*u)/c_s^2 + (c_i*u)^2/(2*c_s^4) - u^2/(2*c_s^2)]
    /// h_i^eq = f_i^eq - w_i
    ///        = w_i * [(rho - 1) + rho*((c_i*u)/c_s^2 + (c_i*u)^2/(2*c_s^4) - u^2/(2*c_s^2))]
    #[inline(always)]
    fn equilibrium_perturbation(rho: f32, u: [f32; 3]) -> [f32; 19] {
        let u_sq = u[0] * u[0] + u[1] * u[1] + u[2] * u[2];
        let inv_cs_sq = 1.0 / CS_SQ; // 3.0
        let inv_2cs4 = inv_cs_sq * inv_cs_sq * 0.5; // 4.5
        let half_inv_cs_sq = 0.5 * inv_cs_sq; // 1.5
        let rho_m1 = rho - 1.0;

        let mut h_eq = [0.0f32; 19];
        for q in 0..19 {
            let c = VELOCITIES[q];
            let cu = c[0] as f32 * u[0] + c[1] as f32 * u[1] + c[2] as f32 * u[2];
            let velocity_terms = cu * inv_cs_sq + cu * cu * inv_2cs4 - u_sq * half_inv_cs_sq;
            h_eq[q] = WEIGHTS[q] * (rho_m1 + rho * velocity_terms);
        }
        h_eq
    }

    /// Recover macroscopic fields from perturbation distributions.
    ///
    /// rho = 1 + sum_i h_i
    /// u_k = (1/rho) * sum_i h_i * c_i^k
    pub fn compute_macroscopic(&mut self) {
        let n = self.nx * self.ny * self.nz;
        let h = &self.h;

        // Parallel macroscopic recovery.
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
                let inv_rho = if rho.abs() > 1e-12 { 1.0 / rho } else { 1.0 };
                *dr = sum_h;
                *vx = sum_cx * inv_rho;
                *vy = sum_cy * inv_rho;
                *vz = sum_cz * inv_rho;
            });

        self.drho = drho_buf;
        self.ux = ux_buf;
        self.uy = uy_buf;
        self.uz = uz_buf;
    }

    /// Fused pull-collide: stream from neighbors and apply BGK in one pass.
    ///
    /// For each destination cell, pull h values from pre-computed source
    /// neighbors (no rem_euclid), compute macroscopic quantities, compute
    /// equilibrium perturbation, and write post-collision result to scratch.
    /// Then swap h and h_scratch.
    fn fused_stream_collide(&mut self) {
        let n = self.nx * self.ny * self.nz;
        let inv_tau = 1.0 / self.tau;
        let h = &self.h;
        let neighbor = &self.neighbor;

        // Collect scratch slices as raw pointers for Send safety.
        // Each cell writes to a unique index so there are no data races.
        struct SendPtr(*mut f32, usize);
        unsafe impl Send for SendPtr {}
        unsafe impl Sync for SendPtr {}

        let ptrs: Vec<SendPtr> = (0..19)
            .map(|q| SendPtr(self.h_scratch[q].as_mut_ptr(), self.h_scratch[q].len()))
            .collect();

        (0..n).into_par_iter().for_each(|dst_idx| {
            // Pull 19 perturbation values using pre-computed neighbor table.
            let mut h_local = [0.0f32; 19];
            for q in 0..19 {
                let src_idx = neighbor.table[q][dst_idx] as usize;
                h_local[q] = h[q][src_idx];
            }

            // Macroscopic from perturbation.
            let mut sum_h = 0.0f32;
            let mut sum_cx = 0.0f32;
            let mut sum_cy = 0.0f32;
            let mut sum_cz = 0.0f32;
            for q in 0..19 {
                let hq = h_local[q];
                sum_h += hq;
                sum_cx += hq * VELOCITIES[q][0] as f32;
                sum_cy += hq * VELOCITIES[q][1] as f32;
                sum_cz += hq * VELOCITIES[q][2] as f32;
            }
            let rho = 1.0 + sum_h;
            let inv_rho = if rho.abs() > 1e-12 { 1.0 / rho } else { 1.0 };
            let u = [sum_cx * inv_rho, sum_cy * inv_rho, sum_cz * inv_rho];

            // Equilibrium perturbation + BGK collision inline.
            let u_sq = u[0] * u[0] + u[1] * u[1] + u[2] * u[2];
            let inv_cs_sq = 3.0f32;
            let inv_2cs4 = 4.5f32;
            let half_inv_cs_sq = 1.5f32;
            let rho_m1 = rho - 1.0;

            for q in 0..19 {
                let c = VELOCITIES[q];
                let cu = c[0] as f32 * u[0] + c[1] as f32 * u[1] + c[2] as f32 * u[2];
                let velocity_terms = cu * inv_cs_sq + cu * cu * inv_2cs4 - u_sq * half_inv_cs_sq;
                let h_eq = WEIGHTS[q] * (rho_m1 + rho * velocity_terms);
                let h_new = h_local[q] - (h_local[q] - h_eq) * inv_tau;

                // SAFETY: dst_idx is unique per iteration, no data race.
                unsafe {
                    let ptr = ptrs[q].0;
                    debug_assert!(dst_idx < ptrs[q].1);
                    *ptr.add(dst_idx) = h_new;
                }
            }
        });

        // Swap buffers.
        std::mem::swap(&mut self.h, &mut self.h_scratch);
        self.timestep += 1;
    }

    /// Apply bounce-back at solid cells identified by the solid mask.
    ///
    /// For each solid cell, reflect distributions: h[opp(q)][idx] = h[q][idx].
    /// Since w_q == w_opp(q) for all D3Q19 directions, the perturbation
    /// form requires no weight correction.
    fn apply_solid_bounce_back(&mut self) {
        let n = self.nx * self.ny * self.nz;
        for idx in 0..n {
            if !self.solid[idx] {
                continue;
            }
            // Read all 19 values, then write reflected.
            let mut vals = [0.0f32; 19];
            for (q, val) in vals.iter_mut().enumerate() {
                *val = self.h[q][idx];
            }
            for (q, &val) in vals.iter().enumerate() {
                self.h[OPPOSITE[q]][idx] = val;
            }
        }
    }

    /// Apply bounce-back on boundary planes (Y and Z walls).
    fn apply_bounce_back_planes(&mut self) {
        let nx = self.nx;
        let ny = self.ny;
        let nz = self.nz;

        // Collects (idx, q, val) tuples first to avoid aliasing.
        let mut writes: Vec<(usize, usize, f32)> = Vec::new();

        for z in 0..nz {
            for x in 0..nx {
                // MinY plane (y=0)
                let idx = z * (nx * ny) + x; // y=0
                for (q, &opp) in OPPOSITE.iter().enumerate() {
                    writes.push((idx, opp, self.h[q][idx]));
                }
                // MaxY plane (y=ny-1)
                let idx = z * (nx * ny) + (ny - 1) * nx + x;
                for (q, &opp) in OPPOSITE.iter().enumerate() {
                    writes.push((idx, opp, self.h[q][idx]));
                }
            }
        }
        for y in 0..ny {
            for x in 0..nx {
                // MinZ plane (z=0)
                let idx = y * nx + x; // z=0
                for (q, &opp) in OPPOSITE.iter().enumerate() {
                    writes.push((idx, opp, self.h[q][idx]));
                }
                // MaxZ plane (z=nz-1)
                let idx = (nz - 1) * (nx * ny) + y * nx + x;
                for (q, &opp) in OPPOSITE.iter().enumerate() {
                    writes.push((idx, opp, self.h[q][idx]));
                }
            }
        }

        for (idx, opp_q, val) in writes {
            self.h[opp_q][idx] = val;
        }
    }

    /// Apply equilibrium inlet (x=0) and density-corrected outlet (x=nx-1).
    ///
    /// Inlet: reset to h_eq at freestream velocity/density.
    /// Outlet: copy from x=nx-2 with density correction to prevent mass drift.
    /// Both exclude wall rows (y=0, y=ny-1, z=0, z=nz-1) to avoid corner
    /// conflicts with bounce-back planes.
    fn apply_inlet_outlet_bc(&mut self) {
        let nx = self.nx;
        let ny = self.ny;
        let nz = self.nz;
        let rho0 = self.freestream_density;
        let h_eq_inlet =
            Self::equilibrium_perturbation(self.freestream_density, self.freestream_velocity);

        // Inlet face (x=0): equilibrium at freestream.
        for z in 1..nz - 1 {
            for y in 1..ny - 1 {
                let idx = z * (nx * ny) + y * nx; // x=0
                for (q, &heq) in h_eq_inlet.iter().enumerate() {
                    self.h[q][idx] = heq;
                }
            }
        }

        // Outlet face (x=nx-1): density-corrected Neumann.
        for z in 1..nz - 1 {
            for y in 1..ny - 1 {
                let src_idx = z * (nx * ny) + y * nx + (nx - 2);
                let dst_idx = z * (nx * ny) + y * nx + (nx - 1);

                // Local density at source cell.
                let mut local_drho = 0.0f32;
                for q in 0..19 {
                    local_drho += self.h[q][src_idx];
                }
                let local_rho = 1.0 + local_drho;

                // Scale to maintain freestream density.
                let scale = if local_rho.abs() > 1e-12 {
                    rho0 / local_rho
                } else {
                    1.0
                };

                for (q, &w) in WEIGHTS.iter().enumerate() {
                    // h_dst = (f_src * scale) - w = (h_src + w) * scale - w
                    let f_src = self.h[q][src_idx] + w;
                    self.h[q][dst_idx] = f_src * scale - w;
                }
            }
        }
    }

    /// Store voxel mask and apply initial bounce-back.
    pub fn inject_boundary_from_voxels(&mut self, voxels: &VoxelGrid) {
        assert_eq!(voxels.nx, self.nx);
        assert_eq!(voxels.ny, self.ny);
        assert_eq!(voxels.nz, self.nz);
        self.solid = voxels.cells.clone();
        self.apply_solid_bounce_back();
    }

    /// Advance the simulation by `substeps` timesteps with full BC application.
    ///
    /// Order per substep:
    /// 1. Fused stream-collide (pull + BGK in one pass)
    /// 2. Solid voxel bounce-back
    /// 3. Inlet/outlet BCs
    /// 4. Wall bounce-back planes (Y/Z)
    pub fn evolve_with_boundaries(&mut self, substeps: usize) {
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

        // Sync macroscopic fields for readback.
        if substeps > 0 {
            self.compute_macroscopic();
        }
    }

    /// Get macroscopic quantities at a specific grid point.
    pub fn get_macroscopic(&self, x: usize, y: usize, z: usize) -> (f32, [f32; 3]) {
        let idx = z * (self.nx * self.ny) + y * self.nx + x;
        let rho = 1.0 + self.drho[idx];
        (rho, [self.ux[idx], self.uy[idx], self.uz[idx]])
    }

    /// Read density field as flat Vec<f32>.
    pub fn read_density_field(&self) -> Vec<f32> {
        self.drho.iter().map(|dr| 1.0 + dr).collect()
    }

    /// Read velocity field as flat Vec<f32> with [vx, vy, vz] per cell.
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

    /// Total mass in the domain: sum of (1 + drho) over all cells.
    pub fn total_mass(&self) -> f64 {
        let n = self.nx * self.ny * self.nz;
        n as f64 + self.drho.iter().map(|&d| d as f64).sum::<f64>()
    }

    /// Maximum velocity magnitude in the domain.
    pub fn max_velocity(&self) -> f64 {
        let n = self.nx * self.ny * self.nz;
        let mut max_sq = 0.0f64;
        for idx in 0..n {
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

    /// Mean velocity magnitude in the domain.
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

    /// Check if all distributions are non-negative (within tolerance).
    pub fn is_stable(&self) -> bool {
        for (q, &w) in WEIGHTS.iter().enumerate() {
            for &h in &self.h[q] {
                // f_i = h_i + w_i; check f_i >= -1e-6 (f32 tolerance)
                if h + w < -1e-6 {
                    return false;
                }
            }
        }
        true
    }

    /// Compute drag and lift using momentum exchange method.
    ///
    /// For each solid boundary cell, sum momentum transferred from
    /// adjacent fluid cells through D3Q19 lattice directions.
    pub fn compute_drag_lift(&self, voxels: &VoxelGrid) -> (f64, f64) {
        let nx = self.nx;
        let ny = self.ny;
        let nz = self.nz;
        let mut drag = 0.0f64;
        let mut lift = 0.0f64;

        for z in 0..nz {
            for y in 0..ny {
                for x in 0..nx {
                    let idx = z * (nx * ny) + y * nx + x;
                    if !voxels.cells[idx] {
                        continue;
                    }
                    for q in 1..19 {
                        let c = VELOCITIES[q];
                        let nx_i = (x as i32 + c[0]).rem_euclid(nx as i32) as usize;
                        let ny_i = (y as i32 + c[1]).rem_euclid(ny as i32) as usize;
                        let nz_i = (z as i32 + c[2]).rem_euclid(nz as i32) as usize;
                        let neighbor_idx = nz_i * (nx * ny) + ny_i * nx + nx_i;
                        if voxels.cells[neighbor_idx] {
                            continue;
                        }
                        // Momentum exchange: f_i + f_opp (in full distribution form).
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_solver(nx: usize, ny: usize, nz: usize) -> LbmSolverSoA {
        let mut s = LbmSolverSoA::new(nx, ny, nz, 0.8);
        s.initialize_uniform(1.0, [0.05, 0.0, 0.0]);
        s
    }

    #[test]
    fn soa_rest_initialization() {
        // At rest (rho=1, u=0) all h values should be zero.
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
        assert!(
            (sum - 1.0).abs() < 1e-6,
            "Weights sum to {sum}, expected 1.0"
        );
    }

    #[test]
    fn soa_opposite_symmetric() {
        for q in 0..19 {
            assert_eq!(OPPOSITE[OPPOSITE[q]], q, "opposite(opposite({q})) != {q}");
        }
    }

    #[test]
    fn soa_opposite_negates_velocity() {
        for q in 0..19 {
            let opp = OPPOSITE[q];
            let c = VELOCITIES[q];
            let co = VELOCITIES[opp];
            assert_eq!(c[0] + co[0], 0, "q={q}: cx not negated");
            assert_eq!(c[1] + co[1], 0, "q={q}: cy not negated");
            assert_eq!(c[2] + co[2], 0, "q={q}: cz not negated");
        }
    }

    #[test]
    fn soa_neighbor_table_consistent() {
        // Verify the neighbor table matches rem_euclid computation.
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
                        assert_eq!(
                            nt.table[q][dst_idx] as usize, expected,
                            "Mismatch at ({x},{y},{z}) q={q}"
                        );
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
        assert!(rho.is_finite(), "Density not finite: {rho}");
        assert!(
            u_mag > 0.01,
            "Expected sustained freestream, got |u|={u_mag}"
        );
    }

    #[test]
    fn soa_drag_with_obstacle() {
        let mut s = make_solver(32, 16, 16);
        let mut voxels = VoxelGrid::new(32, 16, 16);
        for x in 12..16 {
            for y in 6..10 {
                for z in 6..10 {
                    voxels.set(x, y, z, true);
                }
            }
        }
        s.inject_boundary_from_voxels(&voxels);
        s.evolve_with_boundaries(600);
        let (drag, _lift) = s.compute_drag_lift(&voxels);
        assert!(drag.abs() > 1e-6, "Expected nonzero drag, got {drag}");
    }

    #[test]
    fn soa_long_running_stability() {
        let mut s = make_solver(32, 16, 16);
        let mut voxels = VoxelGrid::new(32, 16, 16);
        for x in 12..16 {
            for y in 6..10 {
                for z in 6..10 {
                    voxels.set(x, y, z, true);
                }
            }
        }
        s.inject_boundary_from_voxels(&voxels);

        for batch in 0..30 {
            s.evolve_with_boundaries(100);
            let (rho, u) = s.get_macroscopic(4, 8, 8);
            assert!(
                rho.is_finite() && u[0].is_finite() && u[1].is_finite() && u[2].is_finite(),
                "SoA diverged at step {} (batch {batch}): rho={rho}, u={u:?}",
                (batch + 1) * 100
            );
        }

        let (rho, u) = s.get_macroscopic(4, 8, 8);
        let u_mag = (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt();
        assert!(
            u_mag > 0.005,
            "Expected sustained flow after 3000 steps, got |u|={u_mag}, rho={rho}"
        );
    }

    #[test]
    fn soa_outlet_density_corrected() {
        let mut s = make_solver(16, 8, 8);
        let rho0 = s.freestream_density;
        s.evolve_with_boundaries(50);

        let nx = s.nx;
        let ny = s.ny;
        let nz = s.nz;
        for z in 1..nz - 1 {
            for y in 1..ny - 1 {
                let idx = z * (nx * ny) + y * nx + (nx - 1);
                let mut drho = 0.0f32;
                for q in 0..19 {
                    drho += s.h[q][idx];
                }
                let outlet_rho = 1.0 + drho;
                assert!(
                    (outlet_rho - rho0).abs() < 1e-4,
                    "Outlet density at y={y},z={z}: {outlet_rho}, expected {rho0}"
                );
            }
        }
    }

    #[test]
    fn soa_perturbation_precision() {
        // Verify that f32 perturbation values stay small enough for
        // adequate precision at tau=0.8, u=0.05.
        let h_eq = LbmSolverSoA::equilibrium_perturbation(1.0, [0.05, 0.0, 0.0]);
        for (q, &val) in h_eq.iter().enumerate() {
            assert!(
                val.abs() < 0.1,
                "h_eq[{q}] = {val} is too large for f32 precision",
            );
        }
    }
}

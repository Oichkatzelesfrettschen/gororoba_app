use casimir_core::energy::{
    CasimirEnergyField3D, CasimirEnergyResult, WorldlineCasimirConfig, casimir_energy_at_point,
    casimir_energy_field_3d,
};
use casimir_core::geometry::ParallelPlates;
use gororoba_kernel_api::quantum::{
    CasimirFieldRequest, CasimirFieldSnapshot, CasimirGeometry, CasimirPointSample,
    CasimirWorldlineConfig, MeraLayerSnapshot, QuantumDiagnosticsSnapshot, QuantumDomainConfig,
    QuantumKernel,
};
use quantum_core::mera::{build_mera_structure, mera_entropy_estimate};

pub type QuantumConfig = QuantumDomainConfig;
pub type QuantumCpuKernel = QuantumKernelState;

pub struct QuantumKernelState {
    pub config: QuantumDomainConfig,
    pub mera_layers: Vec<MeraLayerSnapshot>,
    pub entropy: f64,
    pub casimir_result: Option<CasimirEnergyResult>,
    pub casimir_field_3d: Option<CasimirEnergyField3D>,
}

impl QuantumKernelState {
    pub fn new(config: QuantumConfig) -> Self {
        let mera_layers = build_mera_structure(config.lattice.n_sites)
            .into_iter()
            .map(|layer| MeraLayerSnapshot {
                n_isometries: layer.n_isometries,
                n_disentanglers: layer.n_disentanglers,
            })
            .collect();

        Self {
            config,
            mera_layers,
            entropy: 0.0,
            casimir_result: None,
            casimir_field_3d: None,
        }
    }

    pub fn estimate_entropy_with_seed(&mut self, subsystem_size: usize, seed: u64) {
        self.entropy = mera_entropy_estimate(subsystem_size, self.config.lattice.local_dim, seed);
    }

    pub fn compute_casimir_parallel_plates(
        &mut self,
        separation: f64,
        point: [f64; 3],
        config: &WorldlineCasimirConfig,
    ) {
        let geometry = ParallelPlates { separation };
        self.casimir_result = Some(casimir_energy_at_point(&geometry, point, config));
    }

    pub fn compute_casimir(
        &mut self,
        plate_geometry: &CasimirGeometry,
        point: [f64; 3],
        config: &WorldlineCasimirConfig,
    ) {
        match plate_geometry {
            CasimirGeometry::ParallelPlates { separation } => {
                let geometry = ParallelPlates {
                    separation: *separation,
                };
                self.casimir_result = Some(casimir_energy_at_point(&geometry, point, config));
            }
            CasimirGeometry::SpherePlateSphere { .. } => {}
        }
    }

    pub fn compute_casimir_field_3d_parallel_plates(
        &mut self,
        separation: f64,
        bounds: (f64, f64, f64, f64, f64, f64),
        resolution: (usize, usize, usize),
        config: &WorldlineCasimirConfig,
    ) {
        let geometry = ParallelPlates { separation };
        self.casimir_field_3d = Some(casimir_energy_field_3d(
            &geometry, bounds, resolution, config,
        ));
    }

    pub fn layer_count(&self) -> usize {
        self.mera_layers.len()
    }
}

impl QuantumKernel for QuantumKernelState {
    fn domain_config(&self) -> &QuantumDomainConfig {
        &self.config
    }

    fn estimate_entropy(&mut self, subsystem_size: usize) -> f64 {
        self.estimate_entropy_with_seed(subsystem_size, self.config.lattice.seed);
        self.entropy
    }

    fn casimir_at_point(
        &mut self,
        geometry: CasimirGeometry,
        position: [f64; 3],
        config: &CasimirWorldlineConfig,
    ) -> CasimirPointSample {
        let wl = WorldlineCasimirConfig {
            n_loop_points: config.n_loop_points,
            n_loops: config.n_loops,
            t_min: config.t_min,
            t_max: config.t_max,
            n_t_points: config.n_t_points,
            seed: config.seed,
        };
        self.compute_casimir(&geometry, position, &wl);
        let result = self.casimir_result.as_ref().expect("casimir result");
        CasimirPointSample {
            position,
            energy: result.energy,
            error: result.error,
        }
    }

    fn casimir_field(
        &mut self,
        geometry: CasimirGeometry,
        request: &CasimirFieldRequest,
        config: &CasimirWorldlineConfig,
    ) -> Option<CasimirFieldSnapshot> {
        let wl = WorldlineCasimirConfig {
            n_loop_points: config.n_loop_points,
            n_loops: config.n_loops,
            t_min: config.t_min,
            t_max: config.t_max,
            n_t_points: config.n_t_points,
            seed: config.seed,
        };
        let CasimirGeometry::ParallelPlates { separation } = geometry else {
            return None;
        };
        self.compute_casimir_field_3d_parallel_plates(
            separation,
            request.bounds,
            request.resolution,
            &wl,
        );
        self.casimir_field_3d
            .as_ref()
            .map(|field| CasimirFieldSnapshot {
                bounds: request.bounds,
                resolution: field.resolution,
                data: field.data.clone(),
            })
    }

    fn diagnostics(&self) -> QuantumDiagnosticsSnapshot {
        let (casimir_energy, casimir_error) = if let Some(result) = &self.casimir_result {
            (result.energy, result.error)
        } else {
            (0.0, 0.0)
        };
        QuantumDiagnosticsSnapshot {
            backend: gororoba_kernel_api::algebra::KernelBackend::Cpu,
            entanglement_entropy: self.entropy,
            mera_layers: self.layer_count(),
            casimir_energy,
            casimir_error,
            measured_this_tick: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gororoba_kernel_api::quantum::SpinLatticeConfig;

    fn test_config() -> QuantumConfig {
        QuantumConfig {
            lattice: SpinLatticeConfig {
                n_sites: 16,
                local_dim: 2,
                seed: 42,
            },
            subsystem_size: 4,
        }
    }

    #[test]
    fn mera_structure_and_entropy_are_computable() {
        let mut kernel = QuantumKernelState::new(test_config());
        assert!(kernel.layer_count() > 0);
        kernel.estimate_entropy_with_seed(4, 42);
        assert!(kernel.entropy >= 0.0);
    }

    #[test]
    fn casimir_parallel_plates_is_negative() {
        let mut kernel = QuantumKernelState::new(test_config());
        kernel.compute_casimir_parallel_plates(
            1.0,
            [0.0, 0.5, 0.0],
            &WorldlineCasimirConfig::default(),
        );
        let result = kernel.casimir_result.as_ref().unwrap();
        assert!(result.energy < 0.0);
    }

    #[test]
    fn casimir_snapshot_contains_resolution() {
        let mut kernel = QuantumKernelState::new(test_config());
        let snapshot = kernel
            .casimir_field(
                CasimirGeometry::ParallelPlates { separation: 1.0 },
                &CasimirFieldRequest {
                    resolution: (3, 3, 3),
                    bounds: (-1.0, 1.0, 0.0, 1.0, -1.0, 1.0),
                },
                &CasimirWorldlineConfig {
                    n_loop_points: 16,
                    n_loops: 50,
                    t_min: 0.01,
                    t_max: 3.0,
                    n_t_points: 4,
                    seed: 42,
                },
            )
            .unwrap();
        assert_eq!(snapshot.resolution, (3, 3, 3));
    }
}

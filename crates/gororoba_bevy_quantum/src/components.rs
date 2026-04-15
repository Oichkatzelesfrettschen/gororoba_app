// SpinLattice, EntangledPair, CasimirPlate, and quantum domain components.
//
// These define the ECS data model for quantum simulations in Bevy.

use bevy::prelude::*;

pub use gororoba_kernel_api::quantum::CasimirGeometry as PlateGeometry;

/// Marker component for entities in a quantum simulation domain.
#[derive(Component, Default)]
pub struct QuantumDomain;

/// Spin lattice configuration for tensor network simulations.
#[derive(Component)]
pub struct SpinLattice {
    /// Number of sites in the lattice.
    pub n_sites: usize,
    /// Local Hilbert space dimension per site (2 for spin-1/2).
    pub local_dim: usize,
    /// Random seed for MERA entropy estimation.
    pub seed: u64,
}

impl Default for SpinLattice {
    fn default() -> Self {
        Self {
            n_sites: 16,
            local_dim: 2,
            seed: 42,
        }
    }
}

/// Pair of entangled sites for visualization.
#[derive(Component)]
pub struct EntangledPair {
    /// First site index.
    pub site_a: usize,
    /// Second site index.
    pub site_b: usize,
    /// Entanglement entropy between the pair.
    pub entropy: f64,
}

/// Casimir plate component for Casimir effect simulation.
#[derive(Component)]
pub struct CasimirPlate {
    /// Geometry of the plate configuration.
    pub geometry: PlateGeometry,
    /// Position offset of the plate assembly.
    pub position: [f64; 3],
}

impl Default for CasimirPlate {
    fn default() -> Self {
        Self {
            geometry: PlateGeometry::default(),
            position: [0.0, 0.0, 0.0],
        }
    }
}

/// Configuration for the worldline Casimir computation.
#[derive(Component)]
pub struct CasimirParams {
    /// Number of points per worldline loop.
    pub n_loop_points: usize,
    /// Number of Monte Carlo loops per proper-time sample.
    pub n_loops: usize,
    /// Minimum proper time for integration.
    pub t_min: f64,
    /// Maximum proper time for integration.
    pub t_max: f64,
    /// Number of Gauss-Legendre quadrature points.
    pub n_t_points: usize,
    /// Random seed for reproducibility.
    pub seed: u64,
}

impl Default for CasimirParams {
    fn default() -> Self {
        Self {
            n_loop_points: 64,
            n_loops: 1000,
            t_min: 0.01,
            t_max: 10.0,
            n_t_points: 16,
            seed: 42,
        }
    }
}

/// Configuration for 3D Casimir energy field computation.
///
/// When present on a QuantumDomain entity alongside CasimirPlate and
/// CasimirParams, the casimir_field_system will compute a full 3D
/// energy density grid suitable for volume rendering.
#[derive(Component)]
pub struct CasimirFieldConfig {
    /// Grid resolution (nx, ny, nz) for the 3D energy field.
    pub resolution: (usize, usize, usize),
    /// Spatial bounds: (x_min, x_max, y_min, y_max, z_min, z_max).
    pub bounds: (f64, f64, f64, f64, f64, f64),
    /// Whether the 3D field needs recomputation.
    pub dirty: bool,
}

impl Default for CasimirFieldConfig {
    fn default() -> Self {
        Self {
            resolution: (8, 8, 8),
            bounds: (-2.0, 2.0, -0.1, 1.1, -2.0, 2.0),
            dirty: true,
        }
    }
}

/// Quantum simulation parameters.
#[derive(Component)]
pub struct QuantumParams {
    /// Number of MERA/tensor network substeps per FixedUpdate tick.
    pub substeps: usize,
    /// Subsystem size for entropy estimation.
    pub subsystem_size: usize,
}

impl Default for QuantumParams {
    fn default() -> Self {
        Self {
            substeps: 1,
            subsystem_size: 4,
        }
    }
}

/// Diagnostic output from quantum simulations.
#[derive(Component, Default)]
pub struct QuantumDiagnostics {
    /// Entanglement entropy estimate from MERA.
    pub entanglement_entropy: f64,
    /// Number of MERA layers in the network.
    pub mera_layers: usize,
    /// Casimir energy at the evaluation point.
    pub casimir_energy: f64,
    /// Casimir energy statistical error.
    pub casimir_error: f64,
    /// Whether a measurement has been performed this tick.
    pub measured_this_tick: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spin_lattice_default() {
        let lattice = SpinLattice::default();
        assert_eq!(lattice.n_sites, 16);
        assert_eq!(lattice.local_dim, 2);
    }

    #[test]
    fn parallel_plates_default() {
        let plate = CasimirPlate::default();
        match plate.geometry {
            PlateGeometry::ParallelPlates { separation } => {
                assert!((separation - 1.0).abs() < 1e-15);
            }
            _ => panic!("expected ParallelPlates"),
        }
    }

    #[test]
    fn casimir_params_reasonable() {
        let params = CasimirParams::default();
        assert!(params.n_loops > 0);
        assert!(params.t_min < params.t_max);
    }

    #[test]
    fn quantum_params_default() {
        let params = QuantumParams::default();
        assert_eq!(params.substeps, 1);
        assert_eq!(params.subsystem_size, 4);
    }

    #[test]
    fn casimir_field_config_default() {
        let config = CasimirFieldConfig::default();
        assert_eq!(config.resolution, (8, 8, 8));
        assert!(config.dirty);
        let (x_min, x_max, _, _, _, _) = config.bounds;
        assert!(x_min < x_max);
    }
}

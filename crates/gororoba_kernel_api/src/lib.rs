pub mod algebra {
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub enum AlgebraDimension {
        Quaternion,
        Octonion,
        #[default]
        Sedenion,
        Dim32,
    }

    impl AlgebraDimension {
        pub fn dim(self) -> usize {
            match self {
                Self::Quaternion => 4,
                Self::Octonion => 8,
                Self::Sedenion => 16,
                Self::Dim32 => 32,
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum KernelBackend {
        Cpu,
        Vulkan,
        Cuda,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct ZeroDivisorSearchConfig {
        pub tolerance: f64,
        pub max_results: usize,
    }

    impl Default for ZeroDivisorSearchConfig {
        fn default() -> Self {
            Self {
                tolerance: 1e-12,
                max_results: 64,
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct ZeroDivisorPair {
        pub lhs_indices: (usize, usize),
        pub rhs_indices: (usize, usize),
        pub rhs_sign: i8,
        pub product_norm: f64,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct AlgebraSnapshot {
        pub dimension: AlgebraDimension,
        pub backend: KernelBackend,
        pub associator_norm: f64,
        pub zero_divisors: Vec<ZeroDivisorPair>,
    }

    pub trait AlgebraKernel {
        fn dimension(&self) -> AlgebraDimension;
        fn backend(&self) -> KernelBackend {
            KernelBackend::Cpu
        }
        fn multiply(&self, lhs: &[f64], rhs: &[f64]) -> Vec<f64>;
        fn associator_norm(&self, a: &[f64], b: &[f64], c: &[f64]) -> f64;
        fn norm_sq(&self, value: &[f64]) -> f64;
        fn search_zero_divisors(&self, config: &ZeroDivisorSearchConfig) -> Vec<ZeroDivisorPair>;
    }
}

pub mod projection {
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct ProjectionSpec {
        pub axes: [usize; 3],
        pub scale: f32,
    }

    impl ProjectionSpec {
        pub fn new(axes: [usize; 3], scale: f32) -> Self {
            Self { axes, scale }
        }
    }

    impl Default for ProjectionSpec {
        fn default() -> Self {
            Self {
                axes: [1, 2, 3],
                scale: 1.0,
            }
        }
    }

    #[derive(Debug, Clone, Copy, Default, PartialEq)]
    pub struct ProjectedPoint3 {
        pub x: f32,
        pub y: f32,
        pub z: f32,
    }

    impl ProjectedPoint3 {
        pub fn scaled(self, factor: f32) -> Self {
            Self {
                x: self.x * factor,
                y: self.y * factor,
                z: self.z * factor,
            }
        }
    }
}

pub mod fluid {
    use crate::algebra::KernelBackend;
    use std::error::Error;
    use std::fmt::{Display, Formatter};
    use std::fs;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct GridShape3 {
        pub nx: usize,
        pub ny: usize,
        pub nz: usize,
    }

    impl GridShape3 {
        pub fn cell_count(self) -> usize {
            self.nx * self.ny * self.nz
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum FluidBoundaryMode {
        Periodic,
        BounceBack,
        InletOutlet,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum FluidBackendPreference {
        Auto,
        CudaPreferred,
        VulkanPreferred,
        CpuOnly,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum CpuKernelFlavor {
        SoA,
        Scalar,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum FluidBackendKind {
        CpuSoA,
        CpuScalar,
        Vulkan,
        Cuda,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FluidRenderTarget {
        pub width: u32,
        pub height: u32,
    }

    impl FluidRenderTarget {
        pub fn pixel_len(self) -> usize {
            self.width as usize * self.height as usize * 4
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FluidExecutionConfig {
        pub preference: FluidBackendPreference,
        pub cpu_flavor: CpuKernelFlavor,
        pub render_target: Option<FluidRenderTarget>,
        pub allow_gpu_readback: bool,
    }

    impl Default for FluidExecutionConfig {
        fn default() -> Self {
            Self {
                preference: FluidBackendPreference::Auto,
                cpu_flavor: CpuKernelFlavor::SoA,
                render_target: None,
                allow_gpu_readback: false,
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FluidBoundaryConfig {
        pub x_neg: FluidBoundaryMode,
        pub x_pos: FluidBoundaryMode,
        pub y_neg: FluidBoundaryMode,
        pub y_pos: FluidBoundaryMode,
        pub z_neg: FluidBoundaryMode,
        pub z_pos: FluidBoundaryMode,
    }

    impl Default for FluidBoundaryConfig {
        fn default() -> Self {
            Self {
                x_neg: FluidBoundaryMode::Periodic,
                x_pos: FluidBoundaryMode::Periodic,
                y_neg: FluidBoundaryMode::BounceBack,
                y_pos: FluidBoundaryMode::BounceBack,
                z_neg: FluidBoundaryMode::Periodic,
                z_pos: FluidBoundaryMode::Periodic,
            }
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct FluidDomainConfig {
        pub grid: GridShape3,
        pub tau: f32,
        pub rho_init: f32,
        pub u_init: [f32; 3],
        pub force: [f32; 3],
        pub substeps: usize,
        pub execution: FluidExecutionConfig,
        pub boundaries: FluidBoundaryConfig,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct CpuCapabilities {
        pub logical_threads: usize,
        pub avx2: bool,
        pub fma: bool,
        pub f16c: bool,
        pub l3_cache_bytes: Option<usize>,
    }

    impl CpuCapabilities {
        pub fn detect() -> Self {
            #[cfg(target_arch = "x86_64")]
            let avx2 = std::arch::is_x86_feature_detected!("avx2");
            #[cfg(not(target_arch = "x86_64"))]
            let avx2 = false;

            #[cfg(target_arch = "x86_64")]
            let fma = std::arch::is_x86_feature_detected!("fma");
            #[cfg(not(target_arch = "x86_64"))]
            let fma = false;

            #[cfg(target_arch = "x86_64")]
            let f16c = std::arch::is_x86_feature_detected!("f16c");
            #[cfg(not(target_arch = "x86_64"))]
            let f16c = false;

            Self {
                logical_threads: std::thread::available_parallelism()
                    .map(|count| count.get())
                    .unwrap_or(1),
                avx2,
                fma,
                f16c,
                l3_cache_bytes: detect_linux_l3_cache_bytes(),
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct VulkanCapabilities {
        pub device_name: String,
        pub driver: Option<String>,
        pub api_version: Option<String>,
        pub vram_mb: Option<u32>,
        pub supports_fp16: bool,
        pub supports_fp64: bool,
        pub max_compute_shared_memory_size: Option<u32>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct CudaCapabilities {
        pub device_name: String,
        pub driver_version: Option<String>,
        pub cuda_version: Option<String>,
        pub compute_capability: Option<String>,
        pub total_memory_mb: Option<u32>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct FluidBackendCapabilities {
        pub cpu: CpuCapabilities,
        pub vulkan: Option<VulkanCapabilities>,
        pub cuda: Option<CudaCapabilities>,
    }

    impl FluidBackendCapabilities {
        pub fn cpu_only_detected() -> Self {
            Self {
                cpu: CpuCapabilities::detect(),
                vulkan: None,
                cuda: None,
            }
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct FluidFieldSnapshot {
        pub density: Vec<f32>,
        pub velocity_xyz: Vec<f32>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct FluidDiagnosticsSnapshot {
        pub backend: KernelBackend,
        pub timestep: usize,
        pub total_mass: f64,
        pub max_velocity: f64,
        pub mean_velocity: f64,
        pub stable: bool,
    }

    #[derive(Debug, Clone, Copy, Default, PartialEq)]
    pub struct AerodynamicSnapshot {
        pub drag: f64,
        pub lift: f64,
        pub drag_coefficient: f64,
        pub lift_coefficient: f64,
        pub reynolds_number: f64,
        pub mlups: f64,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct FluidRenderFrame {
        pub width: u32,
        pub height: u32,
        pub rgba8: Vec<u8>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum FluidBackendError {
        UnsupportedBackend(&'static str),
        ProbeFailed(String),
        InvalidConfig(String),
        Runtime(String),
    }

    impl Display for FluidBackendError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::UnsupportedBackend(message) => write!(f, "{message}"),
                Self::ProbeFailed(message) => write!(f, "{message}"),
                Self::InvalidConfig(message) => write!(f, "{message}"),
                Self::Runtime(message) => write!(f, "{message}"),
            }
        }
    }

    impl Error for FluidBackendError {}

    pub trait FluidKernel {
        fn backend(&self) -> KernelBackend {
            KernelBackend::Cpu
        }
        fn domain_config(&self) -> &FluidDomainConfig;
        fn selected_backend(&self) -> FluidBackendKind {
            match self.backend() {
                KernelBackend::Cpu => FluidBackendKind::CpuScalar,
                KernelBackend::Vulkan => FluidBackendKind::Vulkan,
                KernelBackend::Cuda => FluidBackendKind::Cuda,
            }
        }
        fn capabilities(&self) -> FluidBackendCapabilities {
            FluidBackendCapabilities::cpu_only_detected()
        }
        fn set_voxel_mask(&mut self, solid_mask: &[bool]);
        fn set_force_field(&mut self, force_xyz: &[[f32; 3]]) -> Result<(), FluidBackendError> {
            if force_xyz.is_empty() {
                return Ok(());
            }
            Err(FluidBackendError::UnsupportedBackend(
                "backend does not support force-field injection",
            ))
        }
        fn step(&mut self, substeps: usize);
        fn diagnostics(&self) -> FluidDiagnosticsSnapshot;
        fn field_snapshot(&self) -> FluidFieldSnapshot;
        fn render_frame(&mut self) -> Result<Option<FluidRenderFrame>, FluidBackendError> {
            Ok(None)
        }
        fn aerodynamic_snapshot(
            &self,
            solid_mask: &[bool],
            freestream_velocity: [f32; 3],
            tau: f32,
        ) -> AerodynamicSnapshot;
    }

    fn detect_linux_l3_cache_bytes() -> Option<usize> {
        let raw = fs::read_to_string("/sys/devices/system/cpu/cpu0/cache/index3/size").ok()?;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        let split_at = trimmed
            .find(|ch: char| !ch.is_ascii_digit())
            .unwrap_or(trimmed.len());
        let digits = trimmed[..split_at].parse::<usize>().ok()?;
        let suffix = trimmed[split_at..].trim().to_ascii_lowercase();
        if suffix.is_empty() || suffix == "b" {
            return Some(digits);
        }
        if suffix == "k" || suffix == "kb" {
            return Some(digits * 1024);
        }
        if suffix == "m" || suffix == "mb" {
            return Some(digits * 1024 * 1024);
        }
        None
    }
}

pub mod relativity {
    use crate::algebra::KernelBackend;

    #[derive(Debug, Clone, Copy, Default, PartialEq)]
    pub enum MetricFamily {
        #[default]
        Schwarzschild,
        Kerr {
            spin: f64,
        },
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct RelativityDomainConfig {
        pub mass: f64,
        pub metric: MetricFamily,
        pub observer_inclination: f64,
        pub observer_distance: f64,
        pub shadow_points: usize,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum GeodesicKind {
        Null,
        Timelike,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct GeodesicSnapshot {
        pub position: [f64; 4],
        pub velocity: [f64; 4],
        pub kind: GeodesicKind,
        pub step_size: f64,
        pub max_steps: usize,
        pub active: bool,
        pub proper_time: f64,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ShadowSnapshot {
        pub alpha: Vec<f64>,
        pub beta: Vec<f64>,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct RelativityDiagnosticsSnapshot {
        pub backend: KernelBackend,
        pub coordinate_time: f64,
        pub proper_time: f64,
        pub time_dilation: f64,
        pub active_geodesics: usize,
        pub shadow_points: usize,
    }

    pub trait RelativityKernel {
        fn backend(&self) -> KernelBackend {
            KernelBackend::Cpu
        }
        fn domain_config(&self) -> &RelativityDomainConfig;
        fn compute_shadow(&mut self) -> ShadowSnapshot;
        fn step_geodesics(
            &self,
            geodesics: &mut [GeodesicSnapshot],
            substeps: usize,
        ) -> RelativityDiagnosticsSnapshot;
        fn time_dilation_factor(&self, radius: f64) -> f64;
        fn event_horizon(&self) -> f64;
        fn isco_radius(&self) -> f64;
    }
}

pub mod quantum {
    use crate::algebra::KernelBackend;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SpinLatticeConfig {
        pub n_sites: usize,
        pub local_dim: usize,
        pub seed: u64,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum CasimirGeometry {
        ParallelPlates {
            separation: f64,
        },
        SpherePlateSphere {
            sphere_radius: f64,
            plate_distance: f64,
        },
    }

    impl Default for CasimirGeometry {
        fn default() -> Self {
            Self::ParallelPlates { separation: 1.0 }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct CasimirWorldlineConfig {
        pub n_loop_points: usize,
        pub n_loops: usize,
        pub t_min: f64,
        pub t_max: f64,
        pub n_t_points: usize,
        pub seed: u64,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct CasimirFieldRequest {
        pub resolution: (usize, usize, usize),
        pub bounds: (f64, f64, f64, f64, f64, f64),
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct CasimirPointSample {
        pub position: [f64; 3],
        pub energy: f64,
        pub error: f64,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MeraLayerSnapshot {
        pub n_isometries: usize,
        pub n_disentanglers: usize,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct CasimirFieldSnapshot {
        pub bounds: (f64, f64, f64, f64, f64, f64),
        pub resolution: (usize, usize, usize),
        pub data: Vec<f64>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct QuantumDomainConfig {
        pub lattice: SpinLatticeConfig,
        pub subsystem_size: usize,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct QuantumDiagnosticsSnapshot {
        pub backend: KernelBackend,
        pub entanglement_entropy: f64,
        pub mera_layers: usize,
        pub casimir_energy: f64,
        pub casimir_error: f64,
        pub measured_this_tick: bool,
    }

    pub trait QuantumKernel {
        fn backend(&self) -> KernelBackend {
            KernelBackend::Cpu
        }
        fn domain_config(&self) -> &QuantumDomainConfig;
        fn estimate_entropy(&mut self, subsystem_size: usize) -> f64;
        fn casimir_at_point(
            &mut self,
            geometry: CasimirGeometry,
            position: [f64; 3],
            config: &CasimirWorldlineConfig,
        ) -> CasimirPointSample;
        fn casimir_field(
            &mut self,
            geometry: CasimirGeometry,
            request: &CasimirFieldRequest,
            config: &CasimirWorldlineConfig,
        ) -> Option<CasimirFieldSnapshot>;
        fn diagnostics(&self) -> QuantumDiagnosticsSnapshot;
    }
}

#[cfg(test)]
mod tests {
    use crate::fluid::{FluidBoundaryConfig, GridShape3};
    use crate::quantum::{QuantumDomainConfig, SpinLatticeConfig};
    use crate::relativity::{MetricFamily, RelativityDomainConfig};

    #[test]
    fn grid_shape_cell_count_matches_dimensions() {
        let grid = GridShape3 {
            nx: 8,
            ny: 4,
            nz: 2,
        };
        assert_eq!(grid.cell_count(), 64);
    }

    #[test]
    fn fluid_boundaries_default_to_tunnel_friendly_layout() {
        let boundaries = FluidBoundaryConfig::default();
        assert!(matches!(
            boundaries.y_neg,
            crate::fluid::FluidBoundaryMode::BounceBack
        ));
        assert!(matches!(
            boundaries.x_neg,
            crate::fluid::FluidBoundaryMode::Periodic
        ));
    }

    #[test]
    fn relativity_config_supports_kerr_metrics() {
        let config = RelativityDomainConfig {
            mass: 1.0,
            metric: MetricFamily::Kerr { spin: 0.9 },
            observer_inclination: std::f64::consts::FRAC_PI_2,
            observer_distance: 50.0,
            shadow_points: 128,
        };
        assert!(matches!(config.metric, MetricFamily::Kerr { spin: 0.9 }));
    }

    #[test]
    fn quantum_domain_config_keeps_lattice_inputs_small_and_explicit() {
        let config = QuantumDomainConfig {
            lattice: SpinLatticeConfig {
                n_sites: 16,
                local_dim: 2,
                seed: 42,
            },
            subsystem_size: 4,
        };
        assert_eq!(config.lattice.n_sites, 16);
        assert_eq!(config.subsystem_size, 4);
    }
}

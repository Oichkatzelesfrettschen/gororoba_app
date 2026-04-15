use gororoba_kernel_api::relativity::{
    GeodesicKind, GeodesicSnapshot, MetricFamily, RelativityDiagnosticsSnapshot,
    RelativityDomainConfig, RelativityKernel, ShadowSnapshot,
};
use gr_core::Kerr;
use gr_core::energy_conserving::{FullGeodesicState, energy_conserving_step};
use gr_core::kerr::{self, GeodesicState};
use gr_core::metric::SpacetimeMetric;
use gr_core::schwarzschild::Schwarzschild;

pub type GrConfig = RelativityDomainConfig;
pub type RelativityCpuKernel = GrKernel;

pub struct GrKernel {
    pub config: RelativityDomainConfig,
    pub schwarzschild: Option<Schwarzschild>,
    pub kerr: Option<Kerr>,
    pub mass: f64,
    pub spin: f64,
    pub shadow_alpha: Vec<f64>,
    pub shadow_beta: Vec<f64>,
}

impl GrKernel {
    pub fn new(config: GrConfig) -> Self {
        let spin = match config.metric {
            MetricFamily::Schwarzschild => 0.0,
            MetricFamily::Kerr { spin } => spin,
        };

        let (schwarzschild, kerr_metric) = match config.metric {
            MetricFamily::Schwarzschild => (Some(Schwarzschild::new(config.mass)), None),
            MetricFamily::Kerr { spin } => (None, Some(Kerr::new(config.mass, spin))),
        };

        Self {
            config,
            schwarzschild,
            kerr: kerr_metric,
            mass: config.mass,
            spin,
            shadow_alpha: Vec::new(),
            shadow_beta: Vec::new(),
        }
    }

    pub fn compute_shadow_points(&mut self, n_points: usize, theta_o: f64) {
        let (alpha, beta) = kerr::shadow_boundary(self.spin / self.mass, n_points, theta_o);
        self.shadow_alpha = alpha;
        self.shadow_beta = beta;
    }

    pub fn initial_geodesic_state(
        &self,
        r0: f64,
        theta0: f64,
        v_r: f64,
        v_theta: f64,
    ) -> GeodesicState {
        GeodesicState {
            t: 0.0,
            r: r0,
            theta: theta0,
            phi: 0.0,
            v_r,
            v_theta,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn trace_null_geodesic(
        &self,
        e: f64,
        l: f64,
        q: f64,
        r0: f64,
        theta0: f64,
        lam_max: f64,
        sgn_r: f64,
        sgn_theta: f64,
        n_steps: usize,
    ) -> kerr::GeodesicResult {
        let a = self.spin / self.mass;
        kerr::trace_null_geodesic(a, e, l, q, r0, theta0, lam_max, sgn_r, sgn_theta, n_steps)
    }

    pub fn time_dilation_factor(&self, r: f64) -> f64 {
        let rs = 2.0 * self.mass;
        if r <= rs {
            return 0.0;
        }
        (1.0 - rs / r).sqrt()
    }

    pub fn event_horizon(&self) -> f64 {
        let a = self.spin;
        self.mass + (self.mass * self.mass - a * a).max(0.0).sqrt()
    }

    pub fn isco_radius(&self) -> f64 {
        if self.spin.abs() < 1e-12 {
            6.0 * self.mass
        } else {
            let a = self.spin / self.mass;
            let z1 = 1.0 + (1.0 - a * a).cbrt() * ((1.0 + a).cbrt() + (1.0 - a).cbrt());
            let z2 = (3.0 * a * a + z1 * z1).sqrt();
            self.mass * (3.0 + z2 - ((3.0 - z1) * (3.0 + z1 + 2.0 * z2)).sqrt())
        }
    }

    pub fn mass(&self) -> f64 {
        self.mass
    }
}

impl RelativityKernel for GrKernel {
    fn domain_config(&self) -> &RelativityDomainConfig {
        &self.config
    }

    fn compute_shadow(&mut self) -> ShadowSnapshot {
        self.compute_shadow_points(self.config.shadow_points, self.config.observer_inclination);
        ShadowSnapshot {
            alpha: self.shadow_alpha.clone(),
            beta: self.shadow_beta.clone(),
        }
    }

    fn step_geodesics(
        &self,
        geodesics: &mut [GeodesicSnapshot],
        substeps: usize,
    ) -> RelativityDiagnosticsSnapshot {
        let mut active_geodesics = 0usize;
        let mut coordinate_time: f64 = 0.0;
        let mut proper_time: f64 = 0.0;

        for geodesic in geodesics {
            if !geodesic.active {
                continue;
            }
            active_geodesics += 1;
            let target_norm = if matches!(geodesic.kind, GeodesicKind::Null) {
                0.0
            } else {
                -1.0
            };

            for _ in 0..substeps {
                let state = FullGeodesicState {
                    x: geodesic.position,
                    v: geodesic.velocity,
                };

                let new_state = if let Some(ref kerr) = self.kerr {
                    energy_conserving_step(
                        &state,
                        geodesic.step_size,
                        |x| kerr.metric_components(x),
                        |x| kerr.christoffel(x),
                        target_norm,
                    )
                } else if let Some(ref schw) = self.schwarzschild {
                    energy_conserving_step(
                        &state,
                        geodesic.step_size,
                        |x| schw.metric_components(x),
                        |x| schw.christoffel(x),
                        target_norm,
                    )
                } else {
                    break;
                };

                geodesic.position = new_state.x;
                geodesic.velocity = new_state.v;
            }

            coordinate_time = coordinate_time.max(geodesic.position[0]);
            proper_time = proper_time.max(geodesic.proper_time);
        }

        RelativityDiagnosticsSnapshot {
            backend: gororoba_kernel_api::algebra::KernelBackend::Cpu,
            coordinate_time,
            proper_time,
            time_dilation: self.time_dilation_factor(self.config.observer_distance),
            active_geodesics,
            shadow_points: self.shadow_alpha.len(),
        }
    }

    fn time_dilation_factor(&self, radius: f64) -> f64 {
        self.time_dilation_factor(radius)
    }

    fn event_horizon(&self) -> f64 {
        self.event_horizon()
    }

    fn isco_radius(&self) -> f64 {
        self.isco_radius()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schwarzschild_config() -> GrConfig {
        RelativityDomainConfig {
            mass: 1.0,
            metric: MetricFamily::Schwarzschild,
            observer_inclination: std::f64::consts::FRAC_PI_2,
            observer_distance: 50.0,
            shadow_points: 64,
        }
    }

    fn kerr_config(spin: f64) -> GrConfig {
        RelativityDomainConfig {
            mass: 1.0,
            metric: MetricFamily::Kerr { spin },
            observer_inclination: std::f64::consts::FRAC_PI_2,
            observer_distance: 50.0,
            shadow_points: 64,
        }
    }

    #[test]
    fn schwarzschild_horizon_and_isco_match_expected_values() {
        let kernel = GrKernel::new(schwarzschild_config());
        assert!((kernel.event_horizon() - 2.0).abs() < 1e-10);
        assert!((kernel.isco_radius() - 6.0).abs() < 1e-10);
    }

    #[test]
    fn kerr_shadow_frame_has_expected_shape() {
        let mut kernel = GrKernel::new(kerr_config(0.5));
        kernel.compute_shadow_points(32, std::f64::consts::FRAC_PI_2);
        assert_eq!(kernel.shadow_alpha.len(), 64);
        assert_eq!(kernel.shadow_alpha.len(), kernel.shadow_beta.len());
    }
}

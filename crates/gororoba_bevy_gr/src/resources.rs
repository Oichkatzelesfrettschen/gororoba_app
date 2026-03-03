// GR engine resource wrapping gr_core.
//
// Manages spacetime metric computations, geodesic integration,
// and shadow boundary calculation as a Bevy resource.

use bevy::prelude::*;

use gr_core::Kerr;
use gr_core::kerr::{self, GeodesicState};
use gr_core::schwarzschild::Schwarzschild;

use crate::components::MetricType;

/// CPU-based GR engine wrapping gr_core solvers.
///
/// Each SpacetimeDomain entity gets its own engine instance for
/// independent black hole simulations.
#[derive(Resource, Default)]
pub struct GrEngine {
    /// Active instances keyed by entity ID.
    pub instances: Vec<(Entity, GrInstance)>,
}

/// Per-entity GR simulation state.
pub struct GrInstance {
    /// Schwarzschild metric (used when spin = 0).
    pub schwarzschild: Option<Schwarzschild>,
    /// Kerr metric (used when spin != 0).
    pub kerr: Option<Kerr>,
    /// Mass parameter.
    pub mass: f64,
    /// Spin parameter.
    pub spin: f64,
    /// Cached shadow boundary points (alpha, beta).
    pub shadow_alpha: Vec<f64>,
    pub shadow_beta: Vec<f64>,
}

/// Configuration for creating a new GR instance.
pub struct GrConfig {
    pub mass: f64,
    pub metric: MetricType,
}

impl GrEngine {
    /// Create a new GR instance for the given entity.
    pub fn create_instance(&mut self, entity: Entity, config: &GrConfig) {
        self.instances.retain(|(e, _)| *e != entity);

        let spin = match config.metric {
            MetricType::Schwarzschild => 0.0,
            MetricType::Kerr { spin } => spin,
        };

        let (schwarzschild, kerr_metric) = match config.metric {
            MetricType::Schwarzschild => (Some(Schwarzschild::new(config.mass)), None),
            MetricType::Kerr { spin } => (None, Some(Kerr::new(config.mass, spin))),
        };

        self.instances.push((
            entity,
            GrInstance {
                schwarzschild,
                kerr: kerr_metric,
                mass: config.mass,
                spin,
                shadow_alpha: Vec::new(),
                shadow_beta: Vec::new(),
            },
        ));
    }

    /// Get mutable reference to an instance by entity.
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut GrInstance> {
        self.instances
            .iter_mut()
            .find(|(e, _)| *e == entity)
            .map(|(_, inst)| inst)
    }

    /// Get reference to an instance by entity.
    pub fn get(&self, entity: Entity) -> Option<&GrInstance> {
        self.instances
            .iter()
            .find(|(e, _)| *e == entity)
            .map(|(_, inst)| inst)
    }

    /// Remove instance for entity.
    pub fn remove(&mut self, entity: Entity) {
        self.instances.retain(|(e, _)| *e != entity);
    }
}

impl GrInstance {
    /// Compute the black hole shadow boundary.
    ///
    /// Uses gr_core::kerr::shadow_boundary for Kerr (reduces to
    /// Schwarzschild circle when spin = 0).
    pub fn compute_shadow(&mut self, n_points: usize, theta_o: f64) {
        let (alpha, beta) = kerr::shadow_boundary(self.spin / self.mass, n_points, theta_o);
        self.shadow_alpha = alpha;
        self.shadow_beta = beta;
    }

    /// Create a GeodesicState for tracing from the given initial conditions.
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

    /// Trace a null geodesic using gr_core's Kerr integrator.
    ///
    /// Returns the geodesic result with trajectory points.
    /// `sgn_r` and `sgn_theta` set initial radial/polar direction (+1 or -1).
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

    /// Compute the time dilation factor at a given radius.
    ///
    /// For Schwarzschild: sqrt(1 - 2M/r).
    /// For Kerr: depends on spin and angle.
    pub fn time_dilation_factor(&self, r: f64) -> f64 {
        let rs = 2.0 * self.mass;
        if r <= rs {
            return 0.0;
        }
        (1.0 - rs / r).sqrt()
    }

    /// Event horizon radius (outer horizon for Kerr).
    pub fn event_horizon(&self) -> f64 {
        let a = self.spin;
        self.mass + (self.mass * self.mass - a * a).max(0.0).sqrt()
    }

    /// Innermost stable circular orbit (ISCO) radius.
    ///
    /// For Schwarzschild: 6M. For Kerr: depends on spin.
    pub fn isco_radius(&self) -> f64 {
        if self.spin.abs() < 1e-12 {
            6.0 * self.mass
        } else {
            // Bardeen's formula for prograde ISCO
            let a = self.spin / self.mass;
            let z1 = 1.0 + (1.0 - a * a).cbrt() * ((1.0 + a).cbrt() + (1.0 - a).cbrt());
            let z2 = (3.0 * a * a + z1 * z1).sqrt();
            self.mass * (3.0 + z2 - ((3.0 - z1) * (3.0 + z1 + 2.0 * z2)).sqrt())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schwarzschild_config() -> GrConfig {
        GrConfig {
            mass: 1.0,
            metric: MetricType::Schwarzschild,
        }
    }

    fn kerr_config(spin: f64) -> GrConfig {
        GrConfig {
            mass: 1.0,
            metric: MetricType::Kerr { spin },
        }
    }

    #[test]
    fn engine_create_and_lookup() {
        let mut engine = GrEngine::default();
        let entity = Entity::from_bits(1);
        engine.create_instance(entity, &schwarzschild_config());

        assert!(engine.get(entity).is_some());
        assert!(engine.get(Entity::from_bits(99)).is_none());
    }

    #[test]
    fn schwarzschild_isco() {
        let mut engine = GrEngine::default();
        let entity = Entity::from_bits(1);
        engine.create_instance(entity, &schwarzschild_config());

        let inst = engine.get(entity).unwrap();
        assert!((inst.isco_radius() - 6.0).abs() < 1e-10);
    }

    #[test]
    fn schwarzschild_event_horizon() {
        let mut engine = GrEngine::default();
        let entity = Entity::from_bits(1);
        engine.create_instance(entity, &schwarzschild_config());

        let inst = engine.get(entity).unwrap();
        assert!((inst.event_horizon() - 2.0).abs() < 1e-10);
    }

    #[test]
    fn kerr_event_horizon_smaller() {
        let mut engine = GrEngine::default();
        let entity = Entity::from_bits(1);
        engine.create_instance(entity, &kerr_config(0.9));

        let inst = engine.get(entity).unwrap();
        // Kerr outer horizon: M + sqrt(M^2 - a^2)
        let expected = 1.0 + (1.0 - 0.81_f64).sqrt();
        assert!((inst.event_horizon() - expected).abs() < 1e-10);
    }

    #[test]
    fn time_dilation_at_infinity() {
        let mut engine = GrEngine::default();
        let entity = Entity::from_bits(1);
        engine.create_instance(entity, &schwarzschild_config());

        let inst = engine.get(entity).unwrap();
        // At large r, time dilation -> 1.0
        let td = inst.time_dilation_factor(1000.0);
        assert!((td - 1.0).abs() < 0.01);
    }

    #[test]
    fn shadow_computation() {
        let mut engine = GrEngine::default();
        let entity = Entity::from_bits(1);
        engine.create_instance(entity, &schwarzschild_config());

        let inst = engine.get_mut(entity).unwrap();
        inst.compute_shadow(64, std::f64::consts::FRAC_PI_2);
        assert!(!inst.shadow_alpha.is_empty());
        assert_eq!(inst.shadow_alpha.len(), inst.shadow_beta.len());
    }
}

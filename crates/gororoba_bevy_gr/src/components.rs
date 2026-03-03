// Geodesic, BlackHole, AccretionDisk, and spacetime components.
//
// These define the ECS data model for GR simulations in Bevy.

use bevy::prelude::*;

/// Marker component for entities in a GR spacetime domain.
#[derive(Component, Default)]
pub struct SpacetimeDomain;

/// Type of black hole metric.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum MetricType {
    /// Non-rotating (Schwarzschild) black hole.
    #[default]
    Schwarzschild,
    /// Rotating (Kerr) black hole with spin parameter a.
    Kerr { spin: f64 },
}

/// Black hole component defining the central mass.
#[derive(Component)]
pub struct BlackHole {
    /// Mass in geometric units (G = c = 1).
    pub mass: f64,
    /// Metric type (Schwarzschild or Kerr).
    pub metric: MetricType,
}

impl Default for BlackHole {
    fn default() -> Self {
        Self {
            mass: 1.0,
            metric: MetricType::Schwarzschild,
        }
    }
}

impl BlackHole {
    /// Event horizon radius for Schwarzschild (r_s = 2M).
    pub fn schwarzschild_radius(&self) -> f64 {
        2.0 * self.mass
    }

    /// Spin parameter a (0 for Schwarzschild).
    pub fn spin(&self) -> f64 {
        match self.metric {
            MetricType::Schwarzschild => 0.0,
            MetricType::Kerr { spin } => spin,
        }
    }
}

/// A geodesic trajectory being integrated through spacetime.
#[derive(Component)]
pub struct Geodesic {
    /// Current Boyer-Lindquist coordinates [t, r, theta, phi].
    pub position: [f64; 4],
    /// Current coordinate velocities [dt/dlambda, dr/dlambda, dtheta/dlambda, dphi/dlambda].
    pub velocity: [f64; 4],
    /// Whether this is a null (light) or timelike (massive) geodesic.
    pub geodesic_type: GeodesicType,
    /// Integration step size in affine parameter.
    pub step_size: f64,
    /// Maximum number of integration steps per tick.
    pub max_steps: usize,
    /// Whether the geodesic is still being integrated (false = terminated).
    pub active: bool,
    /// Accumulated proper time along the geodesic.
    pub proper_time: f64,
}

/// Type of geodesic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeodesicType {
    /// Null geodesic (photon trajectory).
    Null,
    /// Timelike geodesic (massive particle trajectory).
    Timelike,
}

impl Default for Geodesic {
    fn default() -> Self {
        Self {
            position: [0.0, 10.0, std::f64::consts::FRAC_PI_2, 0.0],
            velocity: [1.0, 0.0, 0.0, 0.1],
            geodesic_type: GeodesicType::Null,
            step_size: 0.01,
            max_steps: 100,
            active: true,
            proper_time: 0.0,
        }
    }
}

/// Accretion disk parameters.
#[derive(Component)]
pub struct AccretionDisk {
    /// Inner edge radius (typically ISCO).
    pub r_inner: f64,
    /// Outer edge radius.
    pub r_outer: f64,
    /// Temperature at inner edge (Kelvin, for Novikov-Thorne).
    pub t_inner: f64,
}

impl Default for AccretionDisk {
    fn default() -> Self {
        Self {
            r_inner: 6.0, // ISCO for Schwarzschild
            r_outer: 20.0,
            t_inner: 1e7,
        }
    }
}

/// GR simulation parameters.
#[derive(Component)]
pub struct GrParams {
    /// Number of geodesic integration substeps per FixedUpdate tick.
    pub substeps: usize,
    /// Whether to use energy-conserving integration (more accurate but slower).
    pub energy_conserving: bool,
    /// Observer inclination angle (radians from spin axis).
    pub observer_inclination: f64,
    /// Observer distance from black hole (in units of M).
    pub observer_distance: f64,
}

impl Default for GrParams {
    fn default() -> Self {
        Self {
            substeps: 10,
            energy_conserving: true,
            observer_inclination: std::f64::consts::FRAC_PI_2, // edge-on
            observer_distance: 50.0,
        }
    }
}

/// Diagnostic output from GR simulation.
#[derive(Component, Default)]
pub struct GrDiagnostics {
    /// Current coordinate time of the simulation.
    pub coordinate_time: f64,
    /// Accumulated proper time for the observer.
    pub proper_time: f64,
    /// Time dilation factor (dtau/dt) at observer position.
    pub time_dilation: f64,
    /// Number of active geodesics being traced.
    pub active_geodesics: usize,
    /// Shadow boundary points (alpha, beta) for visualization.
    pub shadow_points: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schwarzschild_radius() {
        let bh = BlackHole {
            mass: 1.0,
            metric: MetricType::Schwarzschild,
        };
        assert!((bh.schwarzschild_radius() - 2.0).abs() < 1e-15);
    }

    #[test]
    fn kerr_spin() {
        let bh = BlackHole {
            mass: 1.0,
            metric: MetricType::Kerr { spin: 0.9 },
        };
        assert!((bh.spin() - 0.9).abs() < 1e-15);
    }

    #[test]
    fn default_geodesic_is_null() {
        let g = Geodesic::default();
        assert_eq!(g.geodesic_type, GeodesicType::Null);
        assert!(g.active);
    }

    #[test]
    fn default_accretion_disk_isco() {
        let disk = AccretionDisk::default();
        // Schwarzschild ISCO at 6M
        assert!((disk.r_inner - 6.0).abs() < 1e-15);
    }
}

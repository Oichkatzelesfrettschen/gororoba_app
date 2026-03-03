// Player spacecraft: navigation through curved spacetime.
//
// The spacecraft moves on geodesics (or with thrust). The time
// dilation HUD shows the difference between proper time and
// coordinate time as the player approaches the black hole.

use bevy::prelude::*;

use gororoba_bevy_gr::{GrEngine, SpacetimeDomain};

use crate::states::SpaceSimState;

/// The player's spacecraft.
#[derive(Component)]
pub struct Spacecraft {
    /// Current radial distance from the black hole (units of M).
    pub radius: f64,
    /// Angular position (radians).
    pub angle: f64,
    /// Radial velocity (positive = outward).
    pub v_radial: f64,
    /// Angular velocity (radians per tick).
    pub v_angular: f64,
    /// Proper time accumulated on the spacecraft.
    pub proper_time: f64,
    /// Whether the spacecraft is actively thrusting.
    pub thrusting: bool,
}

impl Default for Spacecraft {
    fn default() -> Self {
        Self {
            radius: 30.0,
            angle: 0.0,
            v_radial: 0.0,
            v_angular: 0.01,
            proper_time: 0.0,
            thrusting: false,
        }
    }
}

/// Time dilation display for the HUD.
#[derive(Resource, Default)]
pub struct TimeDilationDisplay {
    pub factor: f64,
    pub proper_time: f64,
    pub coordinate_time: f64,
}

/// Plugin for spacecraft systems.
pub struct SpacecraftPlugin;

impl Plugin for SpacecraftPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TimeDilationDisplay>()
            .add_systems(OnEnter(SpaceSimState::Navigating), spawn_spacecraft)
            .add_systems(
                Update,
                (
                    spacecraft_input_system,
                    spacecraft_move_system,
                    spacecraft_gizmo_system,
                )
                    .chain()
                    .run_if(in_state(SpaceSimState::Navigating)),
            );
    }
}

/// Spawn the player spacecraft.
fn spawn_spacecraft(mut commands: Commands) {
    commands.spawn(Spacecraft::default());
}

/// Handle input for spacecraft thrust.
fn spacecraft_input_system(keys: Res<ButtonInput<KeyCode>>, mut craft: Query<&mut Spacecraft>) {
    for mut ship in &mut craft {
        ship.thrusting = false;
        if keys.pressed(KeyCode::KeyW) {
            ship.v_radial -= 0.001; // thrust inward
            ship.thrusting = true;
        }
        if keys.pressed(KeyCode::KeyS) {
            ship.v_radial += 0.001; // thrust outward
            ship.thrusting = true;
        }
        if keys.pressed(KeyCode::KeyA) {
            ship.v_angular += 0.001;
            ship.thrusting = true;
        }
        if keys.pressed(KeyCode::KeyD) {
            ship.v_angular -= 0.001;
            ship.thrusting = true;
        }
    }
}

/// Move the spacecraft and compute time dilation.
fn spacecraft_move_system(
    engine: Res<GrEngine>,
    domain: Query<Entity, With<SpacetimeDomain>>,
    mut craft: Query<&mut Spacecraft>,
    mut dilation: ResMut<TimeDilationDisplay>,
) {
    let gr_instance = domain.iter().next().and_then(|e| engine.get(e));

    for mut ship in &mut craft {
        // Simple orbital mechanics with gravitational pull.
        let r = ship.radius;
        if r > 2.1 {
            // Newtonian-like radial acceleration (crude approximation).
            let gravity = -1.0 / (r * r);
            let centripetal = ship.v_angular * ship.v_angular * r;
            ship.v_radial += gravity + centripetal;
        }

        ship.radius += ship.v_radial;
        ship.angle += ship.v_angular;

        // Clamp radius to avoid singularity.
        ship.radius = ship.radius.max(2.5);

        // Compute time dilation and accumulate proper time.
        if let Some(inst) = gr_instance {
            let td = inst.time_dilation_factor(ship.radius);
            dilation.factor = td;
            ship.proper_time += td;
            dilation.proper_time = ship.proper_time;
            dilation.coordinate_time += 1.0;
        }
    }
}

/// Draw the spacecraft as a gizmo.
fn spacecraft_gizmo_system(craft: Query<&Spacecraft>, mut gizmos: Gizmos) {
    for ship in &craft {
        let pos = Vec3::new(
            ship.radius as f32 * ship.angle.cos() as f32,
            0.0,
            ship.radius as f32 * ship.angle.sin() as f32,
        );

        let color = if ship.thrusting {
            Color::srgb(1.0, 0.5, 0.0)
        } else {
            Color::srgb(0.0, 1.0, 0.5)
        };

        gizmos.sphere(Isometry3d::from_translation(pos), 0.5, color);

        // Draw velocity vector.
        let vel = Vec3::new(
            ship.v_radial as f32 * ship.angle.cos() as f32 * 20.0,
            0.0,
            ship.v_radial as f32 * ship.angle.sin() as f32 * 20.0,
        );
        gizmos.line(pos, pos + vel, Color::srgb(1.0, 1.0, 0.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_spacecraft_orbit() {
        let ship = Spacecraft::default();
        assert!(ship.radius > 2.0);
        assert!(ship.v_angular > 0.0);
        assert!(!ship.thrusting);
    }

    #[test]
    fn spacecraft_radius_clamp() {
        let mut ship = Spacecraft::default();
        ship.radius = 1.0; // below event horizon
        ship.radius = ship.radius.max(2.5);
        assert!((ship.radius - 2.5).abs() < 1e-15);
    }

    #[test]
    fn default_dilation_display() {
        let display = TimeDilationDisplay::default();
        assert!((display.factor).abs() < 1e-15);
    }
}

// Non-euclidean room geometry.
//
// Rooms are connected by hypercomplex rotations from the CD algebra.
// The algebra dimension determines how many basis directions exist,
// and the associator norm drives visual distortion intensity.

use bevy::prelude::*;

use gororoba_bevy_algebra::{
    AlgebraDiagnostics, AlgebraDimension, AlgebraDomain, AlgebraParams, HypercomplexElement,
};

use crate::states::PuzzleSimState;

/// A room in the non-euclidean space.
#[derive(Component)]
pub struct Room {
    /// Room identifier for puzzle logic.
    pub id: usize,
    /// Position in world space (center of room).
    pub center: Vec3,
    /// Room half-extents for bounding box.
    pub half_extents: Vec3,
    /// The hypercomplex rotation that defines this room's orientation.
    /// When non-associative, rooms connect in surprising ways.
    pub rotation_index: usize,
}

/// The active room the player is currently in.
#[derive(Resource, Default)]
pub struct ActiveRoom {
    pub room_id: usize,
}

/// Configuration for the room layout.
#[derive(Resource)]
pub struct RoomLayout {
    /// Number of rooms to generate.
    pub room_count: usize,
    /// Room spacing in world units.
    pub spacing: f32,
    /// Algebra dimension for room rotations.
    pub dimension: AlgebraDimension,
}

impl Default for RoomLayout {
    fn default() -> Self {
        Self {
            room_count: 8,
            spacing: 20.0,
            dimension: AlgebraDimension::Sedenion,
        }
    }
}

/// Visual distortion intensity derived from the associator norm.
#[derive(Resource, Default)]
pub struct DistortionState {
    /// Current distortion intensity (0.0 = euclidean, 1.0 = max distortion).
    pub intensity: f32,
    /// Associator norm from the algebra engine.
    pub associator_norm: f64,
}

/// Plugin for room management.
pub struct RoomsPlugin;

impl Plugin for RoomsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveRoom>()
            .init_resource::<RoomLayout>()
            .init_resource::<DistortionState>()
            .add_systems(OnEnter(PuzzleSimState::Exploring), setup_rooms)
            .add_systems(OnExit(PuzzleSimState::Results), teardown_rooms)
            .add_systems(
                Update,
                (update_distortion_system, room_gizmo_system)
                    .run_if(in_state(PuzzleSimState::Exploring)),
            );
    }
}

/// Spawn rooms and the algebra domain for computing rotations.
fn setup_rooms(mut commands: Commands, layout: Res<RoomLayout>) {
    // Spawn the algebra domain that drives room connections.
    commands.spawn((
        AlgebraDomain,
        AlgebraParams {
            dimension: layout.dimension,
            ..default()
        },
        AlgebraDiagnostics::default(),
    ));

    // Spawn rooms in a ring layout.
    let angle_step = std::f32::consts::TAU / layout.room_count as f32;
    let radius = layout.spacing * layout.room_count as f32 / std::f32::consts::TAU;

    for i in 0..layout.room_count {
        let angle = angle_step * i as f32;
        let center = Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);

        commands.spawn(Room {
            id: i,
            center,
            half_extents: Vec3::splat(layout.spacing * 0.4),
            rotation_index: i % layout.dimension.dim(),
        });
    }

    // Spawn basis elements as children of the algebra domain for
    // associator computation (the algebra plugin's associator_system
    // reads child HypercomplexElement components).
    let dim = layout.dimension.dim();
    for i in 0..3.min(dim) {
        commands.spawn(HypercomplexElement::basis(dim, i + 1));
    }
}

/// Remove all rooms and algebra domain when leaving.
fn teardown_rooms(
    mut commands: Commands,
    rooms: Query<Entity, With<Room>>,
    domains: Query<Entity, With<AlgebraDomain>>,
    elements: Query<Entity, With<HypercomplexElement>>,
) {
    for entity in rooms.iter().chain(domains.iter()).chain(elements.iter()) {
        commands.entity(entity).despawn();
    }
}

/// Update the visual distortion state from algebra diagnostics.
fn update_distortion_system(
    diag_query: Query<&AlgebraDiagnostics, With<AlgebraDomain>>,
    mut distortion: ResMut<DistortionState>,
) {
    for diag in &diag_query {
        distortion.associator_norm = diag.associator_norm;
        // Map associator norm to a 0..1 distortion intensity.
        // Octonion associator norms are typically ~2.0 for basis triples.
        distortion.intensity = (diag.associator_norm as f32 / 2.0).clamp(0.0, 1.0);
    }
}

/// Draw room outlines as wireframe gizmos.
fn room_gizmo_system(
    rooms: Query<&Room>,
    active: Res<ActiveRoom>,
    distortion: Res<DistortionState>,
    mut gizmos: Gizmos,
) {
    for room in &rooms {
        let is_active = room.id == active.room_id;

        // Color rooms based on rotation_index, distortion, and active state.
        let hue = (room.rotation_index as f32 * 0.618) % 1.0; // golden ratio hue spread
        let base_color = if is_active {
            Color::srgb(0.2 + hue * 0.3, 0.8, 0.4)
        } else {
            Color::srgb(0.2 + hue * 0.2, 0.3, 0.4 + hue * 0.3)
        };

        // Warp the room extents slightly based on distortion.
        let warp = 1.0 + distortion.intensity * 0.2;
        let scale = room.half_extents * 2.0 * Vec3::new(warp, 1.0, 1.0 / warp);

        gizmos.cube(
            Transform::from_translation(room.center).with_scale(scale),
            base_color,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_layout_reasonable() {
        let layout = RoomLayout::default();
        assert!(layout.room_count > 0);
        assert!(layout.spacing > 0.0);
        assert_eq!(layout.dimension, AlgebraDimension::Sedenion);
    }

    #[test]
    fn distortion_clamps() {
        let mut distortion = DistortionState::default();
        distortion.associator_norm = 5.0;
        distortion.intensity = (distortion.associator_norm as f32 / 2.0).clamp(0.0, 1.0);
        assert!((distortion.intensity - 1.0).abs() < 1e-6);
    }

    #[test]
    fn active_room_default_zero() {
        let active = ActiveRoom::default();
        assert_eq!(active.room_id, 0);
    }
}

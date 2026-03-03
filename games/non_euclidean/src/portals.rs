// Zero-divisor portal mechanics.
//
// Portals appear at locations where the algebra has zero-divisors
// (pairs a,b where a*b = 0 but a != 0 and b != 0). Traversing a
// portal teleports the player between rooms and changes the active
// algebra basis used for puzzle solving.

use bevy::prelude::*;

use gororoba_bevy_algebra::ZeroDivisorPortal;

use crate::rooms::{ActiveRoom, Room, RoomLayout};
use crate::states::PuzzleSimState;

/// Visual representation of a portal in world space.
#[derive(Component)]
pub struct PortalVisual {
    /// Which room this portal connects from.
    pub from_room: usize,
    /// Which room this portal connects to.
    pub to_room: usize,
    /// World position of the portal.
    pub position: Vec3,
    /// Portal radius for collision detection.
    pub radius: f32,
}

/// Tracks how many portals the player has traversed.
#[derive(Resource, Default)]
pub struct PortalTraversalCount {
    pub count: usize,
}

/// Plugin for portal mechanics.
pub struct PortalsPlugin;

impl Plugin for PortalsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PortalTraversalCount>()
            .add_systems(OnEnter(PuzzleSimState::Exploring), spawn_portal_visuals)
            .add_systems(
                Update,
                (portal_gizmo_system, portal_traverse_system)
                    .run_if(in_state(PuzzleSimState::Exploring)),
            );
    }
}

/// Create visual portal entities from the algebra plugin's ZeroDivisorPortal components.
///
/// Each zero-divisor pair maps to a portal connecting two rooms.
/// The room mapping uses modular arithmetic over the basis indices.
fn spawn_portal_visuals(
    mut commands: Commands,
    zd_portals: Query<&ZeroDivisorPortal>,
    rooms: Query<&Room>,
    layout: Res<RoomLayout>,
) {
    let room_count = layout.room_count;
    if room_count == 0 {
        return;
    }

    let rooms_vec: Vec<&Room> = rooms.iter().collect();
    if rooms_vec.is_empty() {
        return;
    }

    for (idx, portal) in zd_portals.iter().enumerate() {
        if !portal.active {
            continue;
        }

        // Map zero-divisor indices to room connections.
        let from_room = (portal.a_indices.0 + portal.a_indices.1) % room_count;
        let to_room = (portal.b_indices.0 + portal.b_indices.1) % room_count;

        if from_room == to_room {
            continue;
        }

        // Place portal midway between the two rooms.
        let from_center = rooms_vec
            .iter()
            .find(|r| r.id == from_room)
            .map(|r| r.center)
            .unwrap_or(Vec3::ZERO);
        let to_center = rooms_vec
            .iter()
            .find(|r| r.id == to_room)
            .map(|r| r.center)
            .unwrap_or(Vec3::ZERO);

        let position = (from_center + to_center) * 0.5 + Vec3::Y * (idx as f32 * 0.5);

        commands.spawn(PortalVisual {
            from_room,
            to_room,
            position,
            radius: 1.5,
        });
    }
}

/// Draw portals as colored gizmo spheres.
fn portal_gizmo_system(portals: Query<&PortalVisual>, active: Res<ActiveRoom>, mut gizmos: Gizmos) {
    for portal in &portals {
        let is_reachable = portal.from_room == active.room_id || portal.to_room == active.room_id;

        let color = if is_reachable {
            Color::srgba(1.0, 0.3, 1.0, 0.8)
        } else {
            Color::srgba(0.5, 0.2, 0.5, 0.3)
        };

        gizmos.sphere(
            Isometry3d::from_translation(portal.position),
            portal.radius,
            color,
        );
    }
}

/// Handle portal traversal: pressing 'T' near a portal teleports to the connected room.
fn portal_traverse_system(
    keys: Res<ButtonInput<KeyCode>>,
    portals: Query<&PortalVisual>,
    mut active: ResMut<ActiveRoom>,
    mut traversal_count: ResMut<PortalTraversalCount>,
) {
    if !keys.just_pressed(KeyCode::KeyT) {
        return;
    }

    // Find the first portal connected to the current room.
    for portal in &portals {
        if portal.from_room == active.room_id {
            active.room_id = portal.to_room;
            traversal_count.count += 1;
            return;
        }
        if portal.to_room == active.room_id {
            active.room_id = portal.from_room;
            traversal_count.count += 1;
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_traversal_count() {
        let count = PortalTraversalCount::default();
        assert_eq!(count.count, 0);
    }

    #[test]
    fn portal_visual_fields() {
        let pv = PortalVisual {
            from_room: 0,
            to_room: 3,
            position: Vec3::new(5.0, 1.0, 5.0),
            radius: 1.5,
        };
        assert_ne!(pv.from_room, pv.to_room);
        assert!(pv.radius > 0.0);
    }
}

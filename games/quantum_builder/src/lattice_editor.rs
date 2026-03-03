// Spin lattice editor: select sites, create entangled pairs,
// and visualize the tensor network structure.
//
// The player selects lattice sites and pairs them to form
// entangled bonds. The MERA tensor network structure is
// displayed as a hierarchical gizmo overlay.

use bevy::prelude::*;

use gororoba_bevy_quantum::{EntangledPair, QuantumDomain, QuantumEngine, SpinLattice};

use crate::states::QuantumSimState;

/// Currently selected lattice site indices.
#[derive(Resource, Default)]
pub struct LatticeSelection {
    /// First selected site (None if nothing selected).
    pub site_a: Option<usize>,
    /// Second selected site (None if not yet paired).
    pub site_b: Option<usize>,
    /// Total entangled pairs created by the player.
    pub pairs_created: usize,
}

/// Plugin for lattice editing systems.
pub struct LatticeEditorPlugin;

impl Plugin for LatticeEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LatticeSelection>().add_systems(
            Update,
            (
                lattice_select_system,
                entangled_pair_gizmo_system,
                mera_layer_gizmo_system,
            )
                .run_if(in_state(QuantumSimState::Building)),
        );
    }
}

/// Select lattice sites with number keys and create entangled pairs.
///
/// Keys 1-9 select site indices. Press a site key once to select it,
/// press another to pair them. Press 0 to clear the selection.
fn lattice_select_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut selection: ResMut<LatticeSelection>,
    mut commands: Commands,
    lattice_query: Query<(Entity, &SpinLattice), With<QuantumDomain>>,
) {
    let site = if keys.just_pressed(KeyCode::Digit1) {
        Some(0)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(1)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(2)
    } else if keys.just_pressed(KeyCode::Digit4) {
        Some(3)
    } else if keys.just_pressed(KeyCode::Digit5) {
        Some(4)
    } else if keys.just_pressed(KeyCode::Digit6) {
        Some(5)
    } else if keys.just_pressed(KeyCode::Digit7) {
        Some(6)
    } else if keys.just_pressed(KeyCode::Digit8) {
        Some(7)
    } else if keys.just_pressed(KeyCode::Digit9) {
        Some(8)
    } else {
        None
    };

    // Clear selection with 0.
    if keys.just_pressed(KeyCode::Digit0) {
        selection.site_a = None;
        selection.site_b = None;
        return;
    }

    if let Some(idx) = site {
        // Validate the site index against actual lattice size.
        let max_sites = lattice_query
            .iter()
            .next()
            .map(|(_, l)| l.n_sites)
            .unwrap_or(0);

        if idx >= max_sites {
            return;
        }

        if selection.site_a.is_none() {
            selection.site_a = Some(idx);
        } else if selection.site_b.is_none() && selection.site_a != Some(idx) {
            selection.site_b = Some(idx);

            // Create an entangled pair entity.
            let a = selection.site_a.unwrap();
            let b = idx;
            commands.spawn(EntangledPair {
                site_a: a,
                site_b: b,
                entropy: 0.0,
            });
            selection.pairs_created += 1;

            // Reset selection for next pair.
            selection.site_a = None;
            selection.site_b = None;
        }
    }
}

/// Draw entangled pairs as colored lines between lattice sites.
fn entangled_pair_gizmo_system(
    pairs: Query<&EntangledPair>,
    lattice_query: Query<(&SpinLattice, &gororoba_bevy_quantum::CasimirPlate), With<QuantumDomain>>,
    mut gizmos: Gizmos,
) {
    let Some((lattice, plate)) = lattice_query.iter().next() else {
        return;
    };

    let center = Vec3::new(
        plate.position[0] as f32,
        plate.position[1] as f32,
        plate.position[2] as f32,
    );

    for pair in &pairs {
        if pair.site_a >= lattice.n_sites || pair.site_b >= lattice.n_sites {
            continue;
        }

        let radius = 2.0_f32;
        let angle_a = std::f32::consts::TAU * pair.site_a as f32 / lattice.n_sites as f32;
        let angle_b = std::f32::consts::TAU * pair.site_b as f32 / lattice.n_sites as f32;

        let pos_a = center + Vec3::new(radius * angle_a.cos(), 0.0, radius * angle_a.sin());
        let pos_b = center + Vec3::new(radius * angle_b.cos(), 0.0, radius * angle_b.sin());

        // Color entanglement lines by entropy magnitude.
        let t = (pair.entropy as f32).min(1.0);
        let color = Color::srgb(0.8 + t * 0.2, 0.2, 0.8 - t * 0.5);
        gizmos.line(pos_a, pos_b, color);
    }
}

/// Draw MERA tensor network layers as hierarchical arcs above the lattice.
fn mera_layer_gizmo_system(
    engine: Res<QuantumEngine>,
    domain: Query<Entity, With<QuantumDomain>>,
    lattice_query: Query<&gororoba_bevy_quantum::CasimirPlate, With<QuantumDomain>>,
    mut gizmos: Gizmos,
) {
    let Some(entity) = domain.iter().next() else {
        return;
    };

    let Some(inst) = engine.get(entity) else {
        return;
    };

    let Some(plate) = lattice_query.iter().next() else {
        return;
    };

    let center = Vec3::new(
        plate.position[0] as f32,
        plate.position[1] as f32,
        plate.position[2] as f32,
    );

    // Draw MERA layers as concentric circles above the lattice ring.
    for (layer_idx, layer) in inst.mera_layers.iter().enumerate() {
        let height = (layer_idx + 1) as f32 * 1.5;
        let radius = 2.0 - layer_idx as f32 * 0.3;
        let layer_center = center + Vec3::Y * height;

        // Each MERA layer has isometries and disentanglers.
        let n_tensors = layer.n_isometries + layer.n_disentanglers;
        let t = layer_idx as f32 / inst.mera_layers.len().max(1) as f32;
        let color = Color::srgb(0.5 + t * 0.5, 0.8 - t * 0.4, 0.3);

        // Draw a circle for the layer.
        if radius > 0.3 {
            gizmos.circle(
                Isometry3d::new(
                    layer_center,
                    Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                ),
                radius,
                color,
            );
        }

        // Draw tensor nodes on the layer circle.
        for i in 0..n_tensors {
            let angle = std::f32::consts::TAU * i as f32 / n_tensors.max(1) as f32;
            let node_pos =
                layer_center + Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin());
            gizmos.sphere(Isometry3d::from_translation(node_pos), 0.1, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_selection_empty() {
        let sel = LatticeSelection::default();
        assert!(sel.site_a.is_none());
        assert!(sel.site_b.is_none());
        assert_eq!(sel.pairs_created, 0);
    }

    #[test]
    fn selection_pairing_logic() {
        let mut sel = LatticeSelection {
            site_a: Some(0),
            ..Default::default()
        };

        // First site selected.
        assert!(sel.site_a.is_some());
        assert!(sel.site_b.is_none());

        // Select second site (different).
        sel.site_b = Some(3);
        sel.pairs_created += 1;

        assert_eq!(sel.pairs_created, 1);

        // Reset.
        sel.site_a = None;
        sel.site_b = None;
        assert!(sel.site_a.is_none());
    }

    #[test]
    fn same_site_no_pair() {
        let sel = LatticeSelection {
            site_a: Some(2),
            site_b: None,
            pairs_created: 0,
        };
        // Same site should not form a pair.
        assert!(sel.site_a == Some(2));
        assert!(sel.site_a != Some(3));
    }
}

// Game state machine for the Quantum Builder game.
//
// Flow: Menu -> Building -> Measuring -> Results -> Menu.

use bevy::prelude::*;
use bevy_egui::input::EguiWantsInput;

/// Top-level game phase.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum QuantumGamePhase {
    #[default]
    Menu,
    Active,
}

/// Sub-states for active gameplay.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, SubStates)]
#[source(QuantumGamePhase = QuantumGamePhase::Active)]
pub enum QuantumSimState {
    /// Build nanostructures: arrange spin lattices and Casimir plates.
    #[default]
    Building,
    /// Perform quantum measurements and observe entanglement.
    Measuring,
    /// Review experiment results.
    Results,
}

/// Plugin for game state management.
pub struct QuantumStatesPlugin;

impl Plugin for QuantumStatesPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<QuantumGamePhase>()
            .add_sub_state::<QuantumSimState>()
            .add_systems(
                Update,
                menu_start_system.run_if(in_state(QuantumGamePhase::Menu)),
            )
            .add_systems(
                Update,
                advance_state_system.run_if(in_state(QuantumGamePhase::Active)),
            );
    }
}

fn menu_start_system(
    keys: Res<ButtonInput<KeyCode>>,
    egui_input: Res<EguiWantsInput>,
    mut next_phase: ResMut<NextState<QuantumGamePhase>>,
) {
    if egui_input.wants_any_keyboard_input() {
        return;
    }
    if keys.just_pressed(KeyCode::Space) {
        next_phase.set(QuantumGamePhase::Active);
    }
}

fn advance_state_system(
    keys: Res<ButtonInput<KeyCode>>,
    egui_input: Res<EguiWantsInput>,
    state: Res<State<QuantumSimState>>,
    mut next_state: ResMut<NextState<QuantumSimState>>,
    mut next_phase: ResMut<NextState<QuantumGamePhase>>,
) {
    if egui_input.wants_any_keyboard_input() {
        return;
    }
    if keys.just_pressed(KeyCode::Enter) {
        match state.get() {
            QuantumSimState::Building => next_state.set(QuantumSimState::Measuring),
            QuantumSimState::Measuring => next_state.set(QuantumSimState::Results),
            QuantumSimState::Results => next_phase.set(QuantumGamePhase::Menu),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_phase_is_menu() {
        assert_eq!(QuantumGamePhase::default(), QuantumGamePhase::Menu);
    }

    #[test]
    fn default_sim_state_is_building() {
        assert_eq!(QuantumSimState::default(), QuantumSimState::Building);
    }
}

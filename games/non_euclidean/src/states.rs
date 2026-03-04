// Game state machine for the Non-Euclidean puzzle game.
//
// Flow: Menu -> Exploring -> PuzzleSolving -> Results -> Menu.

use bevy::prelude::*;
use bevy_egui::input::EguiWantsInput;

/// Top-level game phase.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum PuzzleGamePhase {
    #[default]
    Menu,
    Active,
}

/// Sub-states for active gameplay.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, SubStates)]
#[source(PuzzleGamePhase = PuzzleGamePhase::Active)]
pub enum PuzzleSimState {
    /// Navigate rooms and discover portals.
    #[default]
    Exploring,
    /// Solve a specific puzzle by manipulating basis elements.
    PuzzleSolving,
    /// Review results after completing a puzzle sequence.
    Results,
}

/// Plugin for game state management.
pub struct PuzzleStatesPlugin;

impl Plugin for PuzzleStatesPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<PuzzleGamePhase>()
            .add_sub_state::<PuzzleSimState>()
            .add_systems(
                Update,
                menu_start_system.run_if(in_state(PuzzleGamePhase::Menu)),
            )
            .add_systems(
                Update,
                advance_state_system.run_if(in_state(PuzzleGamePhase::Active)),
            );
    }
}

fn menu_start_system(
    keys: Res<ButtonInput<KeyCode>>,
    egui_input: Res<EguiWantsInput>,
    mut next_phase: ResMut<NextState<PuzzleGamePhase>>,
) {
    if egui_input.wants_any_keyboard_input() {
        return;
    }
    if keys.just_pressed(KeyCode::Space) {
        next_phase.set(PuzzleGamePhase::Active);
    }
}

fn advance_state_system(
    keys: Res<ButtonInput<KeyCode>>,
    egui_input: Res<EguiWantsInput>,
    state: Res<State<PuzzleSimState>>,
    mut next_state: ResMut<NextState<PuzzleSimState>>,
    mut next_phase: ResMut<NextState<PuzzleGamePhase>>,
) {
    if egui_input.wants_any_keyboard_input() {
        return;
    }
    if keys.just_pressed(KeyCode::Enter) {
        match state.get() {
            PuzzleSimState::Exploring => {
                next_state.set(PuzzleSimState::PuzzleSolving);
            }
            PuzzleSimState::PuzzleSolving => {
                next_state.set(PuzzleSimState::Results);
            }
            PuzzleSimState::Results => {
                next_phase.set(PuzzleGamePhase::Menu);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_phase_is_menu() {
        assert_eq!(PuzzleGamePhase::default(), PuzzleGamePhase::Menu);
    }

    #[test]
    fn default_sim_state_is_exploring() {
        assert_eq!(PuzzleSimState::default(), PuzzleSimState::Exploring);
    }
}

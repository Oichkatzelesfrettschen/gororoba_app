// Fluid dynamics game state machine.
//
// Extends the core GameState with game-specific substates for the
// vehicle design -> wind tunnel -> results flow.

use bevy::prelude::*;
use bevy_egui::input::EguiWantsInput;

/// Game-specific substates for the fluid dynamics game.
///
/// The flow is: Menu -> VehicleDesign -> WindTunnel -> Results -> Menu.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, SubStates)]
#[source(FluidGamePhase = FluidGamePhase::Active)]
pub enum FluidSimState {
    /// Vehicle hull editor: place/remove voxels, select presets.
    #[default]
    VehicleDesign,
    /// Wind tunnel running: LBM simulation active, flow visualization.
    WindTunnel,
    /// Results screen: drag/lift summary, pedagogy review.
    Results,
}

/// Top-level phase for this game. Active means the game is running
/// (not in menu/loading). Substates only exist when Active.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum FluidGamePhase {
    #[default]
    Menu,
    Active,
}

/// Plugin that registers the fluid game states.
pub struct FluidStatesPlugin;

impl Plugin for FluidStatesPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<FluidGamePhase>()
            .add_sub_state::<FluidSimState>()
            .add_systems(
                Update,
                menu_start_system.run_if(in_state(FluidGamePhase::Menu)),
            )
            .add_systems(
                Update,
                advance_state_system.run_if(in_state(FluidGamePhase::Active)),
            );
    }
}

/// Transition from Menu to Active when Space is pressed.
fn menu_start_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    egui_input: Res<EguiWantsInput>,
    mut next_phase: ResMut<NextState<FluidGamePhase>>,
) {
    if egui_input.wants_any_keyboard_input() {
        return;
    }
    if keyboard.just_pressed(KeyCode::Space) {
        next_phase.set(FluidGamePhase::Active);
    }
}

/// Handle state transitions within the active game phase.
fn advance_state_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    egui_input: Res<EguiWantsInput>,
    sim_state: Res<State<FluidSimState>>,
    mut next_sim: ResMut<NextState<FluidSimState>>,
    mut next_phase: ResMut<NextState<FluidGamePhase>>,
) {
    if egui_input.wants_any_keyboard_input() {
        return;
    }
    if keyboard.just_pressed(KeyCode::Enter) {
        match sim_state.get() {
            FluidSimState::VehicleDesign => {
                next_sim.set(FluidSimState::WindTunnel);
            }
            FluidSimState::WindTunnel => {
                next_sim.set(FluidSimState::Results);
            }
            FluidSimState::Results => {
                next_phase.set(FluidGamePhase::Menu);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_phase_is_menu() {
        assert_eq!(FluidGamePhase::default(), FluidGamePhase::Menu);
    }

    #[test]
    fn default_sim_state_is_design() {
        assert_eq!(FluidSimState::default(), FluidSimState::VehicleDesign);
    }
}

// Game state machine for the Interaction Arena.
//
// Flow: Menu -> Playing (ArenaView -> Builder -> Execution -> Results) -> Menu.

use bevy::prelude::*;
use bevy_egui::input::EguiWantsInput;

use gororoba_bevy_game_semantics::{GameSemanticsEngine, LevelPhase};

/// Top-level game phase.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum InteractionArenaPhase {
    #[default]
    Menu,
    Playing,
}

/// Sub-states mirroring the engine's LevelPhase.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, SubStates)]
#[source(InteractionArenaPhase = InteractionArenaPhase::Playing)]
pub enum SimState {
    #[default]
    ArenaView,
    StrategyBuilder,
    Execution,
    Results,
}

pub struct InteractionStatesPlugin;

impl Plugin for InteractionStatesPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<InteractionArenaPhase>()
            .add_sub_state::<SimState>()
            .add_systems(
                Update,
                menu_start_system.run_if(in_state(InteractionArenaPhase::Menu)),
            )
            .add_systems(
                Update,
                (advance_phase_system, sync_sim_state)
                    .chain()
                    .run_if(in_state(InteractionArenaPhase::Playing)),
            );
    }
}

fn menu_start_system(
    keys: Res<ButtonInput<KeyCode>>,
    egui_input: Res<EguiWantsInput>,
    mut next_phase: ResMut<NextState<InteractionArenaPhase>>,
    mut engine: ResMut<GameSemanticsEngine>,
) {
    if egui_input.wants_any_keyboard_input() {
        return;
    }
    if keys.just_pressed(KeyCode::Space) {
        engine.start_level(0);
        next_phase.set(InteractionArenaPhase::Playing);
    }
}

fn advance_phase_system(
    keys: Res<ButtonInput<KeyCode>>,
    egui_input: Res<EguiWantsInput>,
    mut engine: ResMut<GameSemanticsEngine>,
    mut next_top: ResMut<NextState<InteractionArenaPhase>>,
) {
    if egui_input.wants_any_keyboard_input() {
        return;
    }
    if keys.just_pressed(KeyCode::Enter) {
        if engine.phase == LevelPhase::Results && engine.level_complete {
            next_top.set(InteractionArenaPhase::Menu);
        } else {
            engine.advance_phase();
        }
    }
}

fn sync_sim_state(
    engine: Res<GameSemanticsEngine>,
    state: Res<State<SimState>>,
    mut next: ResMut<NextState<SimState>>,
) {
    let target = match engine.phase {
        LevelPhase::ArenaView => SimState::ArenaView,
        LevelPhase::StrategyBuilder => SimState::StrategyBuilder,
        LevelPhase::Execution => SimState::Execution,
        LevelPhase::Results => SimState::Results,
    };
    if *state.get() != target {
        next.set(target);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_phase_is_menu() {
        assert_eq!(
            InteractionArenaPhase::default(),
            InteractionArenaPhase::Menu
        );
    }

    #[test]
    fn default_sim_state_is_arena_view() {
        assert_eq!(SimState::default(), SimState::ArenaView);
    }
}

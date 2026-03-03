// Game state machine for the Relativistic Space game.
//
// Flow: Menu -> Observing -> Navigating -> Results -> Menu.

use bevy::prelude::*;

/// Top-level game phase.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum SpaceGamePhase {
    #[default]
    Menu,
    Active,
}

/// Sub-states for active gameplay.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, SubStates)]
#[source(SpaceGamePhase = SpaceGamePhase::Active)]
pub enum SpaceSimState {
    /// Observe a black hole: shadow, accretion disk, geodesics.
    #[default]
    Observing,
    /// Navigate a spacecraft through curved spacetime.
    Navigating,
    /// Review mission results.
    Results,
}

/// Plugin for game state management.
pub struct SpaceStatesPlugin;

impl Plugin for SpaceStatesPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<SpaceGamePhase>()
            .add_sub_state::<SpaceSimState>()
            .add_systems(
                Update,
                menu_start_system.run_if(in_state(SpaceGamePhase::Menu)),
            )
            .add_systems(
                Update,
                advance_state_system.run_if(in_state(SpaceGamePhase::Active)),
            );
    }
}

fn menu_start_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut next_phase: ResMut<NextState<SpaceGamePhase>>,
) {
    if keys.just_pressed(KeyCode::Space) {
        next_phase.set(SpaceGamePhase::Active);
    }
}

fn advance_state_system(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<SpaceSimState>>,
    mut next_state: ResMut<NextState<SpaceSimState>>,
    mut next_phase: ResMut<NextState<SpaceGamePhase>>,
) {
    if keys.just_pressed(KeyCode::Enter) {
        match state.get() {
            SpaceSimState::Observing => next_state.set(SpaceSimState::Navigating),
            SpaceSimState::Navigating => next_state.set(SpaceSimState::Results),
            SpaceSimState::Results => next_phase.set(SpaceGamePhase::Menu),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_phase_is_menu() {
        assert_eq!(SpaceGamePhase::default(), SpaceGamePhase::Menu);
    }

    #[test]
    fn default_sim_state_is_observing() {
        assert_eq!(SpaceSimState::default(), SpaceSimState::Observing);
    }
}

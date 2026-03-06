// Execution phase: animate strategy step-by-step with payoff accumulation.

use bevy::prelude::*;

use gororoba_bevy_game_semantics::{GameSemanticsEngine, LevelPhase};

use crate::states::SimState;

/// Tracks animation timing for execution steps.
#[derive(Resource)]
pub struct ExecutionTimer {
    pub timer: Timer,
}

impl Default for ExecutionTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.6, TimerMode::Repeating),
        }
    }
}

pub struct ExecutionPlugin;

impl Plugin for ExecutionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ExecutionTimer>()
            .add_systems(OnEnter(SimState::Execution), reset_execution_timer)
            .add_systems(
                Update,
                execution_step_system.run_if(in_state(SimState::Execution)),
            );
    }
}

fn reset_execution_timer(mut timer: ResMut<ExecutionTimer>) {
    timer.timer.reset();
}

fn execution_step_system(
    time: Res<Time>,
    mut timer: ResMut<ExecutionTimer>,
    mut engine: ResMut<GameSemanticsEngine>,
) {
    if engine.phase != LevelPhase::Execution {
        return;
    }

    timer.timer.tick(time.delta());
    if timer.timer.just_finished() {
        engine.step_execution();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_execution_timer() {
        let mut t = ExecutionTimer::default();
        assert!(!t.timer.tick(std::time::Duration::ZERO).just_finished());
    }
}

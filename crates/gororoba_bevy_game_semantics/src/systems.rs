// Bevy systems for game semantics: validation, execution, payoff.
//
// Validation runs in FixedUpdate to check strategy constraints.
// Execution and payoff computation run in Update for smooth animation.

use bevy::prelude::*;

use crate::resources::{GameSemanticsEngine, LevelPhase};

/// Validate the current strategy against conditions each fixed tick.
pub fn validation_system(mut engine: ResMut<GameSemanticsEngine>) {
    if engine.phase == LevelPhase::StrategyBuilder {
        engine.validate_strategy();
    }
}

/// Step the execution forward at a controlled pace.
pub fn execution_step_system(mut engine: ResMut<GameSemanticsEngine>, time: Res<Time>) {
    if engine.phase != LevelPhase::Execution {
        return;
    }

    // Step every 0.5 seconds for visible animation.
    static TIMER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let elapsed_bits = time.elapsed_secs_f64().to_bits();
    let prev_bits = TIMER.swap(elapsed_bits, std::sync::atomic::Ordering::Relaxed);
    let prev = f64::from_bits(prev_bits);
    let curr = time.elapsed_secs_f64();

    // Step once every 0.5 seconds.
    if (curr * 2.0).floor() > (prev * 2.0).floor() {
        engine.step_execution();
    }
}

/// Diagnostics: log current engine state for debugging.
pub fn diagnostics_system(engine: Res<GameSemanticsEngine>) {
    if !engine.is_changed() {
        return;
    }

    if let Some(level) = engine.current_level_def() {
        trace!(
            "GameSemantics: Level {} '{}', Phase: {:?}, Score: {:.1}",
            level.number, level.name, engine.phase, engine.level_score,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn systems_are_fns() {
        // Verify systems have correct signatures (compile-time check).
        let _ = validation_system as fn(ResMut<GameSemanticsEngine>);
    }
}

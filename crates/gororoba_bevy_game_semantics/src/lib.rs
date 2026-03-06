// Game semantics and classical game theory as a Bevy plugin.
//
// Provides arenas, strategies, conditions (well-bracketing, innocence),
// strategy composition, payoff matrices, Nash equilibria, and Pareto
// frontier analysis. Self-contained -- no open_gororoba dependency.
//
// Based on Castellan & Clairambault, "Disentangling Parallelism and
// Interference in Game Semantics" (LMCS 2024).

use bevy::prelude::*;

pub mod arena;
pub mod components;
pub mod composition;
pub mod conditions;
pub mod levels;
pub mod payoff;
pub mod resources;
pub mod strategy;
pub mod systems;

pub use arena::{Arena, Event, MoveKind, Polarity};
pub use components::{ArenaNode, CausalEdge, ConflictMarker, PayoffCell, StrategyMoveMarker};
pub use conditions::{ActiveConditions, ConditionResult};
pub use levels::LevelDef;
pub use payoff::{NashResult, PayoffMatrix};
pub use resources::{GameSemanticsEngine, LevelPhase};
pub use strategy::{Strategy, StrategyError};

pub struct GameSemanticsPlugin;

impl Plugin for GameSemanticsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameSemanticsEngine>()
            .add_systems(FixedUpdate, systems::validation_system)
            .add_systems(
                Update,
                (systems::execution_step_system, systems::diagnostics_system),
            );
    }
}

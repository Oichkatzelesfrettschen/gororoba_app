// ECS components for game semantics visualization.

use bevy::prelude::*;

use crate::arena::Polarity;

/// Marker for entities representing arena nodes in the scene.
#[derive(Component)]
pub struct ArenaNode {
    /// Index into the arena's event list.
    pub event_id: usize,
    /// Polarity determines color (blue for Player, red for Opponent).
    pub polarity: Polarity,
    /// Whether this node has been played in the current strategy.
    pub played: bool,
    /// Whether this node is available to play next.
    pub available: bool,
}

/// Marker for a strategy move visualization.
#[derive(Component)]
pub struct StrategyMoveMarker {
    /// Index in the strategy's move sequence.
    pub sequence_index: usize,
    /// The arena event this move corresponds to.
    pub event_id: usize,
}

/// Marker for a payoff cell in the results matrix display.
#[derive(Component)]
pub struct PayoffCell {
    pub row: usize,
    pub col: usize,
    pub value: f64,
    pub is_equilibrium: bool,
}

/// Marker for causal edge visualization (enabling relation).
#[derive(Component)]
pub struct CausalEdge {
    pub from_event: usize,
    pub to_event: usize,
}

/// Marker for conflict visualization (orange wiggly lines).
#[derive(Component)]
pub struct ConflictMarker {
    pub event_a: usize,
    pub event_b: usize,
}

/// Marker for well-bracketing visualization (nested rectangles).
#[derive(Component)]
pub struct BracketMarker {
    pub question_event: usize,
    pub answer_event: Option<usize>,
    pub depth: usize,
}

/// Marker for constraint satisfaction indicator.
#[derive(Component)]
pub struct ConstraintIndicator {
    pub name: String,
    pub satisfied: bool,
}

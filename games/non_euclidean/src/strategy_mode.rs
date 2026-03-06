// Strategy Mode: adversarial two-player mode over algebra puzzles.
//
// An Opponent AI places blocking basis elements after each Player move.
// Player must find strategies that produce the target regardless of
// Opponent's blocks, creating a two-player zero-sum game.

use bevy::prelude::*;

use gororoba_bevy_algebra::CdAlgebraEngine;

use crate::puzzles::{PuzzleDef, PuzzleState, introductory_puzzles};
use crate::states::PuzzleSimState;

/// Strategy mode state.
#[derive(Resource)]
pub struct StrategyModeState {
    /// Whether strategy mode is active.
    pub active: bool,
    /// Opponent's blocked basis indices.
    pub blocked_bases: Vec<usize>,
    /// Opponent difficulty (0.0 = random, 1.0 = perfect minimax).
    pub difficulty: f32,
    /// Minimax evaluation of the current position.
    pub minimax_value: f64,
    /// Number of Opponent blocks placed so far.
    pub blocks_placed: usize,
    /// Maximum blocks the Opponent can place per puzzle.
    pub max_blocks: usize,
    /// History of (player_move, opponent_block) pairs.
    pub move_history: Vec<(usize, Option<usize>)>,
}

impl Default for StrategyModeState {
    fn default() -> Self {
        Self {
            active: false,
            blocked_bases: Vec::new(),
            difficulty: 0.5,
            minimax_value: 0.0,
            blocks_placed: 0,
            max_blocks: 2,
            move_history: Vec::new(),
        }
    }
}

/// Plugin for strategy mode systems.
pub struct StrategyModePlugin;

impl Plugin for StrategyModePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StrategyModeState>()
            .add_systems(OnEnter(PuzzleSimState::PuzzleSolving), reset_strategy_mode)
            .add_systems(
                Update,
                (opponent_response_system, minimax_eval_system)
                    .chain()
                    .run_if(in_state(PuzzleSimState::PuzzleSolving))
                    .run_if(|sm: Res<StrategyModeState>| sm.active),
            );
    }
}

fn reset_strategy_mode(mut state: ResMut<StrategyModeState>) {
    state.blocked_bases.clear();
    state.blocks_placed = 0;
    state.minimax_value = 0.0;
    state.move_history.clear();
}

/// After the Player selects a basis element, the Opponent places a block.
fn opponent_response_system(
    puzzle_state: Res<PuzzleState>,
    mut strategy_state: ResMut<StrategyModeState>,
) {
    if !puzzle_state.is_changed() {
        return;
    }

    // Only block after a new Player selection.
    let player_count = puzzle_state.selected_elements.len();
    let history_count = strategy_state.move_history.len();

    if player_count <= history_count {
        return;
    }

    // Get the latest Player move.
    let latest_move = puzzle_state.selected_elements[player_count - 1];

    // Opponent chooses a block.
    let block = if strategy_state.blocks_placed < strategy_state.max_blocks {
        let puzzles = introductory_puzzles();
        let current = puzzle_state.current_puzzle;
        if current < puzzles.len() {
            let puzzle = &puzzles[current];
            choose_block(
                puzzle,
                &puzzle_state.selected_elements,
                &strategy_state.blocked_bases,
                strategy_state.difficulty,
            )
        } else {
            None
        }
    } else {
        None
    };

    if let Some(b) = block {
        strategy_state.blocked_bases.push(b);
        strategy_state.blocks_placed += 1;
    }
    strategy_state.move_history.push((latest_move, block));
}

/// Choose which basis to block. Greedy strategy: block the basis that
/// would be most useful to the Player given the current selection.
fn choose_block(
    puzzle: &PuzzleDef,
    selected: &[usize],
    already_blocked: &[usize],
    difficulty: f32,
) -> Option<usize> {
    // Available bases to block (not already selected or blocked).
    let candidates: Vec<usize> = puzzle
        .available_bases
        .iter()
        .copied()
        .filter(|b| !selected.contains(b) && !already_blocked.contains(b))
        .collect();

    if candidates.is_empty() {
        return None;
    }

    if difficulty < 0.3 {
        // Random: just pick the first candidate.
        return candidates.first().copied();
    }

    // Greedy: block the basis that appears in the target with the
    // highest coefficient (most impactful to block).
    let mut best = candidates[0];
    let mut best_score = 0.0_f64;

    for &c in &candidates {
        let score = if c < puzzle.target.len() {
            puzzle.target[c].abs()
        } else {
            0.0
        };
        if score > best_score {
            best_score = score;
            best = c;
        }
    }

    Some(best)
}

/// Evaluate the minimax value of the current position.
fn minimax_eval_system(
    puzzle_state: Res<PuzzleState>,
    mut strategy_state: ResMut<StrategyModeState>,
    _engine: Res<CdAlgebraEngine>,
) {
    let puzzles = introductory_puzzles();
    let current = puzzle_state.current_puzzle;
    if current >= puzzles.len() {
        return;
    }
    let puzzle = &puzzles[current];

    // Evaluate: can the Player still win given blocked bases?
    let available: Vec<usize> = puzzle
        .available_bases
        .iter()
        .copied()
        .filter(|b| !strategy_state.blocked_bases.contains(b))
        .collect();

    // Simple minimax: count how many useful bases remain vs blocked.
    let total = puzzle.available_bases.len() as f64;
    let remaining = available.len() as f64;
    let blocked = strategy_state.blocked_bases.len() as f64;

    // Value is the ratio of remaining useful bases.
    strategy_state.minimax_value = if total > 0.0 {
        (remaining - blocked) / total
    } else {
        0.0
    };
}

/// Check if a basis index is blocked in strategy mode.
pub fn is_blocked(strategy_state: &StrategyModeState, basis: usize) -> bool {
    strategy_state.active && strategy_state.blocked_bases.contains(&basis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_strategy_mode_inactive() {
        let state = StrategyModeState::default();
        assert!(!state.active);
        assert!(state.blocked_bases.is_empty());
    }

    #[test]
    fn choose_block_returns_candidate() {
        let puzzle = PuzzleDef {
            name: "test",
            description: "test",
            target: vec![0.0, 1.0, 0.0, 0.5],
            available_bases: vec![1, 2, 3],
            dimension: 4,
        };
        let block = choose_block(&puzzle, &[1], &[], 0.5);
        assert!(block.is_some());
        let b = block.unwrap();
        assert!(b == 2 || b == 3);
    }

    #[test]
    fn choose_block_greedy_picks_high_target() {
        let puzzle = PuzzleDef {
            name: "test",
            description: "test",
            target: vec![0.0, 5.0, 0.1, 3.0],
            available_bases: vec![1, 2, 3],
            dimension: 4,
        };
        // With high difficulty, should block basis 1 (highest target coeff).
        // But basis 1 is already selected, so block basis 3 (next highest).
        let block = choose_block(&puzzle, &[1], &[], 0.8);
        assert_eq!(block, Some(3));
    }

    #[test]
    fn choose_block_none_when_all_blocked() {
        let puzzle = PuzzleDef {
            name: "test",
            description: "test",
            target: vec![0.0, 1.0],
            available_bases: vec![1],
            dimension: 2,
        };
        let block = choose_block(&puzzle, &[1], &[], 0.5);
        assert!(block.is_none());
    }

    #[test]
    fn is_blocked_when_active() {
        let mut state = StrategyModeState {
            active: true,
            ..Default::default()
        };
        state.blocked_bases.push(3);
        assert!(is_blocked(&state, 3));
        assert!(!is_blocked(&state, 1));
    }

    #[test]
    fn is_blocked_when_inactive() {
        let mut state = StrategyModeState::default();
        state.blocked_bases.push(3);
        // Not active, so nothing is blocked.
        assert!(!is_blocked(&state, 3));
    }
}

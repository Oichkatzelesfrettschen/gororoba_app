// Puzzle logic: manipulate basis elements to open portal paths.
//
// Each puzzle presents a target algebra expression. The player
// must select and combine basis elements to match the target,
// accounting for non-associativity and zero-divisor structure.

use bevy::prelude::*;

use gororoba_bevy_algebra::{CdAlgebraEngine, HypercomplexElement};

use crate::states::PuzzleSimState;

/// A puzzle definition.
#[derive(Clone)]
pub struct PuzzleDef {
    /// Human-readable name.
    pub name: &'static str,
    /// Description of what the player must do.
    pub description: &'static str,
    /// Target element the player must produce via multiplication.
    pub target: Vec<f64>,
    /// Available basis elements the player can combine.
    pub available_bases: Vec<usize>,
    /// Algebra dimension for this puzzle.
    pub dimension: usize,
}

/// The set of predefined puzzles.
pub fn introductory_puzzles() -> Vec<PuzzleDef> {
    vec![
        PuzzleDef {
            name: "Quaternion Product",
            description: "Multiply two quaternion basis elements to produce e3 (k).",
            target: vec![0.0, 0.0, 0.0, 1.0],
            available_bases: vec![1, 2, 3],
            dimension: 4,
        },
        PuzzleDef {
            name: "Octonion Non-Associativity",
            description: "Find a triple (a,b,c) where (a*b)*c differs from a*(b*c).",
            target: vec![0.0; 8], // Any nonzero associator wins.
            available_bases: vec![1, 2, 3, 4, 5, 6, 7],
            dimension: 8,
        },
        PuzzleDef {
            name: "Sedenion Zero-Divisor",
            description: "Find two nonzero elements whose product is zero.",
            target: vec![0.0; 16],
            available_bases: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
            dimension: 16,
        },
        PuzzleDef {
            name: "Portal Key",
            description: "Construct a specific sedenion element to activate a portal.",
            target: {
                let mut t = vec![0.0; 16];
                t[1] = 1.0;
                t[4] = 1.0;
                t
            },
            available_bases: vec![1, 2, 3, 4],
            dimension: 16,
        },
        PuzzleDef {
            name: "Norm Preservation",
            description: "Multiply two unit elements. Does the product have unit norm?",
            target: vec![0.0; 16], // Any unit-norm product wins.
            available_bases: vec![1, 2, 3, 4, 5, 6, 7, 8],
            dimension: 16,
        },
    ]
}

/// Current puzzle state.
#[derive(Resource, Default)]
pub struct PuzzleState {
    /// Index into the puzzle list.
    pub current_puzzle: usize,
    /// Elements the player has selected (basis indices).
    pub selected_elements: Vec<usize>,
    /// Result of the most recent computation.
    pub result: Option<Vec<f64>>,
    /// Whether the current puzzle is solved.
    pub solved: bool,
    /// Total puzzles solved in this session.
    pub total_solved: usize,
}

/// Plugin for puzzle logic.
pub struct PuzzlesPlugin;

impl Plugin for PuzzlesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PuzzleState>()
            .add_systems(OnEnter(PuzzleSimState::PuzzleSolving), setup_puzzle)
            .add_systems(
                Update,
                (puzzle_input_system, puzzle_check_system)
                    .chain()
                    .run_if(in_state(PuzzleSimState::PuzzleSolving)),
            );
    }
}

/// Initialize the puzzle state when entering PuzzleSolving mode.
fn setup_puzzle(mut puzzle_state: ResMut<PuzzleState>) {
    puzzle_state.selected_elements.clear();
    puzzle_state.result = None;
    puzzle_state.solved = false;
}

/// Handle player input for selecting basis elements.
///
/// Number keys 1-9 select basis elements. Backspace removes the last selection.
/// Space triggers computation of the product.
fn puzzle_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut puzzle_state: ResMut<PuzzleState>,
    engine: Res<CdAlgebraEngine>,
    strategy_state: Res<crate::strategy_mode::StrategyModeState>,
) {
    let puzzles = introductory_puzzles();
    let current = puzzle_state.current_puzzle;
    if current >= puzzles.len() {
        return;
    }
    let puzzle = &puzzles[current];

    // Number key selection (1-9 maps to basis index).
    let key_map = [
        (KeyCode::Digit1, 1),
        (KeyCode::Digit2, 2),
        (KeyCode::Digit3, 3),
        (KeyCode::Digit4, 4),
        (KeyCode::Digit5, 5),
        (KeyCode::Digit6, 6),
        (KeyCode::Digit7, 7),
        (KeyCode::Digit8, 8),
        (KeyCode::Digit9, 9),
    ];

    for (key, basis_idx) in key_map {
        if keys.just_pressed(key)
            && puzzle.available_bases.contains(&basis_idx)
            && !crate::strategy_mode::is_blocked(&strategy_state, basis_idx)
        {
            puzzle_state.selected_elements.push(basis_idx);
        }
    }

    // Backspace removes last selection.
    if keys.just_pressed(KeyCode::Backspace) {
        puzzle_state.selected_elements.pop();
    }

    // Space computes the product of selected elements.
    if keys.just_pressed(KeyCode::Space) && puzzle_state.selected_elements.len() >= 2 {
        // Use the first algebra instance available for computation.
        if let Some((_, inst)) = engine.instances.first() {
            let dim = puzzle.dimension;
            let elements: Vec<HypercomplexElement> = puzzle_state
                .selected_elements
                .iter()
                .map(|&i| HypercomplexElement::basis(dim, i))
                .collect();

            let result = if current == 1 && elements.len() >= 3 {
                let ab = inst.multiply(&elements[0].coeffs, &elements[1].coeffs);
                let ab_c = inst.multiply(&ab, &elements[2].coeffs);
                let bc = inst.multiply(&elements[1].coeffs, &elements[2].coeffs);
                let a_bc = inst.multiply(&elements[0].coeffs, &bc);
                ab_c.iter()
                    .zip(a_bc.iter())
                    .map(|(lhs, rhs)| lhs - rhs)
                    .collect()
            } else {
                // Multiply left-to-right: a * b * c * ...
                let mut product = elements[0].coeffs.clone();
                for elem in &elements[1..] {
                    product = inst.multiply(&product, &elem.coeffs);
                }
                product
            };
            puzzle_state.result = Some(result);
        }
    }
}

/// Check if the current puzzle is solved.
fn puzzle_check_system(mut puzzle_state: ResMut<PuzzleState>) {
    if puzzle_state.solved {
        return;
    }

    let puzzles = introductory_puzzles();
    let current = puzzle_state.current_puzzle;
    if current >= puzzles.len() {
        return;
    }
    let puzzle = &puzzles[current];

    if let Some(ref result) = puzzle_state.result {
        let solved = match current {
            // Puzzle 0: exact match to target (quaternion product).
            0 => {
                let diff: f64 = result
                    .iter()
                    .zip(puzzle.target.iter())
                    .map(|(a, b)| (a - b).powi(2))
                    .sum();
                diff.sqrt() < 0.1
            }
            // Puzzle 1: any nonzero associator (we just need nonzero result diff).
            1 => {
                let norm_sq: f64 = result.iter().map(|x| x * x).sum();
                norm_sq > 1e-6
            }
            // Puzzle 2: product is zero (zero-divisor found).
            2 => {
                let norm_sq: f64 = result.iter().map(|x| x * x).sum();
                norm_sq < 1e-10
            }
            // Puzzle 3: match target element.
            3 => {
                let diff: f64 = result
                    .iter()
                    .zip(puzzle.target.iter())
                    .map(|(a, b)| (a - b).powi(2))
                    .sum();
                diff.sqrt() < 0.1
            }
            // Puzzle 4: unit norm product.
            4 => {
                let norm_sq: f64 = result.iter().map(|x| x * x).sum();
                (norm_sq - 1.0).abs() < 0.1
            }
            _ => false,
        };

        if solved {
            puzzle_state.solved = true;
            puzzle_state.total_solved += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_introductory_puzzles() {
        let puzzles = introductory_puzzles();
        assert_eq!(puzzles.len(), 5);
    }

    #[test]
    fn puzzle_dimensions_match_targets() {
        for puzzle in introductory_puzzles() {
            assert_eq!(
                puzzle.target.len(),
                puzzle.dimension,
                "puzzle '{}' target len mismatch",
                puzzle.name
            );
        }
    }

    #[test]
    fn default_puzzle_state() {
        let state = PuzzleState::default();
        assert_eq!(state.current_puzzle, 0);
        assert!(state.selected_elements.is_empty());
        assert!(!state.solved);
    }

    #[test]
    fn puzzle_bases_within_dimension() {
        for puzzle in introductory_puzzles() {
            for &basis in &puzzle.available_bases {
                assert!(
                    basis < puzzle.dimension,
                    "puzzle '{}': basis {basis} >= dim {}",
                    puzzle.name,
                    puzzle.dimension
                );
            }
        }
    }
}

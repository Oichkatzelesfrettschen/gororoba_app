// Level definitions for the Interaction Arena game.
//
// 10 levels progressively introduce game semantics concepts and
// classical game theory elements. Each level specifies an arena,
// active conditions, pre-placed opponent moves, expected solution
// criteria, and payoff parameters.

use crate::arena::{Arena, MoveKind, Polarity, arena_bool, arena_nat, arena_ref, arena_unit};
use crate::conditions::ActiveConditions;
use crate::payoff::PayoffMatrix;

/// A level definition.
#[derive(Debug, Clone)]
pub struct LevelDef {
    /// Level number (1-10).
    pub number: usize,
    /// Human-readable name.
    pub name: &'static str,
    /// Game semantics concept taught.
    pub concept: &'static str,
    /// Classical game theory element.
    pub gt_element: &'static str,
    /// Description shown to the player.
    pub description: &'static str,
    /// The arena for this level.
    pub arena: Arena,
    /// Pre-placed Opponent moves (indices into arena events).
    pub opponent_moves: Vec<usize>,
    /// Which conditions are active.
    pub conditions: ActiveConditions,
    /// Optimal number of Player moves for max score.
    pub optimal_move_count: usize,
    /// Payoff matrix for the results screen.
    pub payoff_matrix: PayoffMatrix,
}

/// Build all 10 level definitions.
pub fn all_levels() -> Vec<LevelDef> {
    vec![
        level_1_hello_bool(),
        level_2_copycat(),
        level_3_branching(),
        level_4_well_bracketed(),
        level_5_composer(),
        level_6_state_of_mind(),
        level_7_race_condition(),
        level_8_innocent_play(),
        level_9_parallel_innocence(),
        level_10_semantic_cube(),
    ]
}

/// Level 1: Hello Bool -- Arenas, moves, polarity.
fn level_1_hello_bool() -> LevelDef {
    let arena = arena_bool();
    let mut pm = PayoffMatrix::zero_sum(2, 1);
    pm.rows = vec!["tt".into(), "ff".into()];
    pm.cols = vec!["q".into()];
    pm.values = vec![vec![10.0], vec![10.0]];

    LevelDef {
        number: 1,
        name: "Hello Bool",
        concept: "Arenas, moves, polarity (+/-)",
        gt_element: "Payoff: choosing tt vs ff",
        description: "Welcome to game semantics! An arena is a game board. \
            Opponent (-red) asks questions; you (+blue) give answers. \
            Opponent has asked 'q'. Choose your answer: tt or ff.",
        arena,
        opponent_moves: vec![0], // O: q
        conditions: ActiveConditions::default(),
        optimal_move_count: 1,
        payoff_matrix: pm,
    }
}

/// Level 2: The Copycat -- Copycat strategy, function types.
fn level_2_copycat() -> LevelDef {
    let u = arena_unit();
    let arena = Arena::function_arena(&u, &u);

    let mut pm = PayoffMatrix::zero_sum(2, 2);
    pm.rows = vec!["Copy".into(), "Ignore".into()];
    pm.cols = vec!["Ask".into(), "Wait".into()];
    pm.values = vec![vec![20.0, 5.0], vec![0.0, 0.0]];

    LevelDef {
        number: 2,
        name: "The Copycat",
        concept: "Copycat strategy, function types (U -> U)",
        gt_element: "Tit-for-tat / mirroring",
        description: "Functions are strategies on A -> B arenas. \
            The copycat strategy mirrors Opponent's moves. When Opponent \
            asks in the codomain, forward the question to the domain. \
            When Opponent answers in the domain, forward the answer back.",
        arena,
        opponent_moves: vec![2, 1], // O: codomain q, then O: domain answer
        conditions: ActiveConditions::default(),
        optimal_move_count: 2,
        payoff_matrix: pm,
    }
}

/// Level 3: Branching -- Branching, let-binding (B -> N).
fn level_3_branching() -> LevelDef {
    let b = arena_bool();
    let n = arena_nat();
    let arena = Arena::function_arena(&b, &n);

    let mut pm = PayoffMatrix::zero_sum(4, 2);
    pm.rows = vec!["0".into(), "1".into(), "2".into(), "3".into()];
    pm.cols = vec!["tt".into(), "ff".into()];
    pm.values = vec![
        vec![5.0, 15.0],
        vec![15.0, 5.0],
        vec![10.0, 10.0],
        vec![0.0, 0.0],
    ];

    LevelDef {
        number: 3,
        name: "Branching",
        concept: "Branching, let-binding (B -> N)",
        gt_element: "Decision trees",
        description: "The arena B -> N represents a function from booleans to naturals. \
            First, read the boolean input (ask in the domain). Then, based on \
            the answer (tt or ff), choose which natural to return. Your response \
            should depend on the input -- that's branching.",
        arena,
        opponent_moves: vec![], // Player initiates by asking domain
        conditions: ActiveConditions::default(),
        optimal_move_count: 3,
        payoff_matrix: pm,
    }
}

/// Level 4: Well-Bracketed -- Well-bracketing constraint.
fn level_4_well_bracketed() -> LevelDef {
    let u = arena_unit();
    let mut arena = Arena::function_arena(&u, &u);
    // Add extra question-answer pair to test bracketing depth.
    let extra_q = arena.add_event(
        Polarity::Opponent,
        MoveKind::Question,
        "q2",
        Some(1), // justified by domain answer slot
    );
    arena.add_event(Polarity::Player, MoveKind::Answer, "a2", Some(extra_q));

    let mut pm = PayoffMatrix::zero_sum(2, 2);
    pm.rows = vec!["Bracketed".into(), "Unbracketed".into()];
    pm.cols = vec!["Simple".into(), "Nested".into()];
    pm.values = vec![vec![20.0, 15.0], vec![5.0, 0.0]];

    LevelDef {
        number: 4,
        name: "Well-Bracketed",
        concept: "Well-bracketing (answer before new question)",
        gt_element: "Sequential rationality",
        description: "Well-bracketing enforces call/return discipline: you must \
            answer the most recent unanswered question before asking a new one. \
            Like a stack: last question in, first answer out. Violating this \
            is like returning from the wrong function call.",
        arena,
        opponent_moves: vec![2, 4], // O: q, then O: q2
        conditions: ActiveConditions {
            well_bracketing: true,
            ..Default::default()
        },
        optimal_move_count: 2,
        payoff_matrix: pm,
    }
}

/// Level 5: The Composer -- Strategy composition.
fn level_5_composer() -> LevelDef {
    let u = arena_unit();
    let b = arena_bool();
    // Compose (U -> B) ; (B -> U).
    let arena1 = Arena::function_arena(&u, &b);
    let arena2 = Arena::function_arena(&b, &u);
    let arena = Arena::product_arena(&arena1, &arena2);

    let mut pm = PayoffMatrix::zero_sum(2, 2);
    pm.rows = vec!["Compose".into(), "Independent".into()];
    pm.cols = vec!["Sync".into(), "Desync".into()];
    pm.values = vec![vec![25.0, 10.0], vec![10.0, 5.0]];

    LevelDef {
        number: 5,
        name: "The Composer",
        concept: "Strategy composition (A->B, B->C => A->C)",
        gt_element: "Subgame perfection",
        description: "Composition chains strategies: given f: A->B and g: B->C, \
            build g . f : A->C. The internal B moves are hidden -- only A and C \
            moves are visible. Synchronize on the shared interface.",
        arena,
        opponent_moves: vec![],
        conditions: ActiveConditions::default(),
        optimal_move_count: 4,
        payoff_matrix: pm,
    }
}

/// Level 6: State of Mind -- References, interference.
fn level_6_state_of_mind() -> LevelDef {
    let r = arena_ref();
    let u = arena_unit();
    let arena = Arena::product_arena(&r, &u);

    let pm = PayoffMatrix::new(
        vec!["Read".into(), "Write".into(), "Both".into()],
        vec!["Clean".into(), "Dirty".into()],
        vec![vec![10.0, 5.0], vec![15.0, 10.0], vec![20.0, 8.0]],
    );

    LevelDef {
        number: 6,
        name: "State of Mind",
        concept: "References, interference (+ref type)",
        gt_element: "Payoff matrix + Nash equilibrium",
        description: "References introduce state: read and write operations. \
            Interference happens when multiple strategies access the same reference. \
            The ref type adds read/write question-answer pairs. Find the Nash \
            equilibrium strategy that optimizes your payoff.",
        arena,
        opponent_moves: vec![0, 2], // O: read, O: write
        conditions: ActiveConditions::default(),
        optimal_move_count: 2,
        payoff_matrix: pm,
    }
}

/// Level 7: Race Condition -- Parallel composition, conflict detection.
fn level_7_race_condition() -> LevelDef {
    let u1 = arena_unit();
    let u2 = arena_unit();
    let arena = Arena::product_arena(&u1, &u2);

    // Prisoner's dilemma structure.
    let pm = PayoffMatrix::new(
        vec!["Cooperate".into(), "Defect".into()],
        vec!["Cooperate".into(), "Defect".into()],
        vec![vec![15.0, 0.0], vec![20.0, 5.0]],
    );

    LevelDef {
        number: 7,
        name: "Race Condition",
        concept: "Parallel composition, conflict detection",
        gt_element: "Prisoner's dilemma structure",
        description: "Parallel composition runs strategies side by side. When they \
            access shared resources, conflicts arise -- race conditions! \
            Two threads both want to answer the same question. The dilemma: \
            cooperate (take turns) or defect (race ahead)?",
        arena,
        opponent_moves: vec![0, 2], // O: q in both components
        conditions: ActiveConditions::default(),
        optimal_move_count: 2,
        payoff_matrix: pm,
    }
}

/// Level 8: Innocent Play -- Innocence condition.
fn level_8_innocent_play() -> LevelDef {
    let b = arena_bool();
    let arena = Arena::function_arena(&b, &b);

    let mut pm = PayoffMatrix::zero_sum(2, 2);
    pm.rows = vec!["Innocent".into(), "Interfering".into()];
    pm.cols = vec!["Reveal".into(), "Hide".into()];
    pm.values = vec![vec![20.0, 15.0], vec![10.0, 5.0]];

    LevelDef {
        number: 8,
        name: "Innocent Play",
        concept: "Innocence (use only visible history)",
        gt_element: "Imperfect information games",
        description: "An innocent strategy depends only on the P-view: the part of \
            history visible to Player. Hidden Opponent moves cannot influence your \
            decisions. This models programs without side effects -- pure functions.",
        arena,
        opponent_moves: vec![],
        conditions: ActiveConditions {
            innocence: true,
            ..Default::default()
        },
        optimal_move_count: 3,
        payoff_matrix: pm,
    }
}

/// Level 9: Parallel Innocence -- Causal, not interleaving.
fn level_9_parallel_innocence() -> LevelDef {
    let u = arena_unit();
    let arena = Arena::product_arena(&u, &u);

    let pm = PayoffMatrix::new(
        vec!["Causal".into(), "Sequential".into()],
        vec!["Fair".into(), "Biased".into()],
        vec![vec![25.0, 15.0], vec![15.0, 10.0]],
    );

    LevelDef {
        number: 9,
        name: "Parallel Innocence",
        concept: "Parallel innocence (causal, not interleaving)",
        gt_element: "Mechanism design",
        description: "Parallel innocence strengthens innocence: your response must \
            depend only on causally related history, not on the interleaving order \
            of independent moves. Two independent threads cannot observe each \
            other's timing. Design a mechanism that is fair regardless of order.",
        arena,
        opponent_moves: vec![0, 2],
        conditions: ActiveConditions {
            parallel_innocence: true,
            ..Default::default()
        },
        optimal_move_count: 2,
        payoff_matrix: pm,
    }
}

/// Level 10: The Semantic Cube -- All constraints toggleable.
fn level_10_semantic_cube() -> LevelDef {
    let b = arena_bool();
    let n = arena_nat();
    let r = arena_ref();
    // Complex arena combining function, product, and ref.
    let f_bn = Arena::function_arena(&b, &n);
    let arena = Arena::product_arena(&f_bn, &r);

    let pm = PayoffMatrix::new(
        vec!["PCF".into(), "IA".into(), "IA//".into(), "Custom".into()],
        vec!["Minimal".into(), "Maximal".into()],
        vec![
            vec![30.0, 20.0],
            vec![25.0, 25.0],
            vec![20.0, 30.0],
            vec![15.0, 15.0],
        ],
    );

    LevelDef {
        number: 10,
        name: "The Semantic Cube",
        concept: "All constraints toggleable",
        gt_element: "Pareto frontier visualization",
        description: "The semantic cube: toggle innocence, well-bracketing, and \
            sequentiality to capture different programming languages. \
            PCF = well-bracketed + innocent + sequential. \
            IA = well-bracketed + sequential (drops innocence). \
            IA// = well-bracketed + innocent (drops sequentiality). \
            Find the Pareto-optimal strategy for your chosen language model.",
        arena,
        opponent_moves: vec![],
        conditions: ActiveConditions {
            well_bracketing: true,
            innocence: true,
            parallel_innocence: false,
            sequentiality: true,
        },
        optimal_move_count: 4,
        payoff_matrix: pm,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_levels_defined() {
        let levels = all_levels();
        assert_eq!(levels.len(), 10);
    }

    #[test]
    fn level_numbers_sequential() {
        let levels = all_levels();
        for (i, level) in levels.iter().enumerate() {
            assert_eq!(level.number, i + 1);
        }
    }

    #[test]
    fn all_levels_have_arenas() {
        for level in all_levels() {
            assert!(
                !level.arena.events.is_empty(),
                "Level {} '{}' has empty arena",
                level.number,
                level.name
            );
        }
    }

    #[test]
    fn all_levels_have_payoff_matrices() {
        for level in all_levels() {
            assert!(
                !level.payoff_matrix.values.is_empty(),
                "Level {} has no payoff matrix",
                level.number
            );
        }
    }

    #[test]
    fn opponent_moves_within_arena() {
        for level in all_levels() {
            for &m in &level.opponent_moves {
                assert!(
                    m < level.arena.events.len(),
                    "Level {}: opponent move {} out of bounds (arena has {} events)",
                    level.number,
                    m,
                    level.arena.events.len()
                );
            }
        }
    }

    #[test]
    fn level_1_is_simple() {
        let l = level_1_hello_bool();
        assert_eq!(l.optimal_move_count, 1);
        assert_eq!(l.opponent_moves.len(), 1);
        // No conditions active on level 1.
        assert!(!l.conditions.well_bracketing);
        assert!(!l.conditions.innocence);
    }

    #[test]
    fn level_10_has_all_conditions() {
        let l = level_10_semantic_cube();
        assert!(l.conditions.well_bracketing);
        assert!(l.conditions.innocence);
        assert!(l.conditions.sequentiality);
    }
}

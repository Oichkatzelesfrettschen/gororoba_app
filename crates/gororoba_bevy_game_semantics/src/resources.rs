// ECS resources for game semantics engine state.

use bevy::prelude::*;

use crate::conditions::ConditionResult;
use crate::levels::LevelDef;
use crate::payoff::NashResult;
use crate::strategy::Strategy;

/// Current phase of a level's gameplay.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LevelPhase {
    /// Viewing the arena structure.
    #[default]
    ArenaView,
    /// Building a strategy by placing moves.
    StrategyBuilder,
    /// Executing the strategy step by step.
    Execution,
    /// Viewing results and payoff analysis.
    Results,
}

/// Primary engine resource managing game semantics state.
#[derive(Resource)]
pub struct GameSemanticsEngine {
    /// All level definitions.
    pub levels: Vec<LevelDef>,
    /// Current level index (0-based).
    pub current_level: usize,
    /// Current phase within the level.
    pub phase: LevelPhase,
    /// The player's strategy for the current level.
    pub strategy: Option<Strategy>,
    /// Condition check results from the last validation.
    pub condition_results: Vec<ConditionResult>,
    /// Current execution step (during Execution phase).
    pub execution_step: usize,
    /// Accumulated payoff during execution.
    pub accumulated_payoff: f64,
    /// Final score for the level.
    pub level_score: f64,
    /// Nash equilibrium results for the current level.
    pub nash_results: Vec<NashResult>,
    /// Whether the current level is complete.
    pub level_complete: bool,
    /// Total score across all levels.
    pub total_score: f64,
    /// Levels completed.
    pub levels_completed: Vec<bool>,
}

impl Default for GameSemanticsEngine {
    fn default() -> Self {
        let levels = crate::levels::all_levels();
        let n = levels.len();
        Self {
            levels,
            current_level: 0,
            phase: LevelPhase::default(),
            strategy: None,
            condition_results: Vec::new(),
            execution_step: 0,
            accumulated_payoff: 0.0,
            level_score: 0.0,
            nash_results: Vec::new(),
            level_complete: false,
            total_score: 0.0,
            levels_completed: vec![false; n],
        }
    }
}

impl GameSemanticsEngine {
    /// Get the current level definition.
    pub fn current_level_def(&self) -> Option<&LevelDef> {
        self.levels.get(self.current_level)
    }

    /// Start a new level: reset strategy and phase.
    pub fn start_level(&mut self, level_index: usize) {
        if level_index >= self.levels.len() {
            return;
        }
        self.current_level = level_index;
        self.phase = LevelPhase::ArenaView;
        self.level_complete = false;
        self.level_score = 0.0;
        self.execution_step = 0;
        self.accumulated_payoff = 0.0;
        self.condition_results.clear();
        self.nash_results.clear();

        // Create a fresh strategy with the level's arena and pre-placed opponent moves.
        let level = &self.levels[level_index];
        let mut strategy = Strategy::new(level.arena.clone());
        for &m in &level.opponent_moves {
            strategy.add_move(m);
        }
        self.strategy = Some(strategy);
    }

    /// Advance to the next phase.
    pub fn advance_phase(&mut self) {
        self.phase = match self.phase {
            LevelPhase::ArenaView => LevelPhase::StrategyBuilder,
            LevelPhase::StrategyBuilder => {
                self.validate_strategy();
                self.execution_step = 0;
                LevelPhase::Execution
            }
            LevelPhase::Execution => {
                self.compute_results();
                LevelPhase::Results
            }
            LevelPhase::Results => {
                // Mark level complete and return to arena view of same level
                // (or advance to next level via select_level).
                self.level_complete = true;
                if self.current_level < self.levels.len() {
                    self.levels_completed[self.current_level] = true;
                }
                self.total_score += self.level_score;
                LevelPhase::ArenaView
            }
        };
    }

    /// Validate the current strategy against active conditions.
    pub fn validate_strategy(&mut self) {
        if let (Some(strategy), Some(level)) = (&self.strategy, self.levels.get(self.current_level))
        {
            self.condition_results =
                crate::conditions::check_conditions(strategy, &level.conditions);
        }
    }

    /// Step the execution forward by one move.
    pub fn step_execution(&mut self) -> bool {
        if let Some(strategy) = &self.strategy
            && self.execution_step < strategy.moves.len()
        {
            self.accumulated_payoff += 10.0;
            self.execution_step += 1;
            return true;
        }
        false
    }

    /// Compute final results: score, Nash equilibria, etc.
    fn compute_results(&mut self) {
        if let Some(level) = self.levels.get(self.current_level) {
            let move_count = self
                .strategy
                .as_ref()
                .map(|s| s.player_moves().len())
                .unwrap_or(0);

            let constraints_satisfied = self
                .condition_results
                .iter()
                .filter(|r| r.satisfied)
                .count();
            let total_constraints = self.condition_results.len();

            self.level_score = crate::payoff::compute_payoff(
                move_count,
                level.optimal_move_count,
                constraints_satisfied,
                total_constraints,
            );

            self.nash_results = crate::payoff::find_nash_equilibria(&level.payoff_matrix);
        }
    }

    /// Add a Player move to the current strategy.
    pub fn add_player_move(&mut self, event_id: usize) -> bool {
        if let Some(strategy) = &mut self.strategy
            && event_id < strategy.arena.events.len()
        {
            strategy.add_move(event_id);
            return true;
        }
        false
    }

    /// Undo the last move.
    pub fn undo_move(&mut self) -> Option<usize> {
        self.strategy.as_mut().and_then(|s| s.undo())
    }

    /// Get available moves (events that can be legally played next).
    pub fn available_moves(&self) -> Vec<usize> {
        let Some(strategy) = &self.strategy else {
            return Vec::new();
        };

        let played: std::collections::HashSet<usize> = strategy.moves.iter().copied().collect();

        strategy
            .arena
            .events
            .iter()
            .filter(|e| {
                // Not already played.
                !played.contains(&e.id)
                    // Justifier already played (or no justifier = initial move).
                    && e.justifier.is_none_or(|j| played.contains(&j))
            })
            .map(|e| e.id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_engine_has_10_levels() {
        let engine = GameSemanticsEngine::default();
        assert_eq!(engine.levels.len(), 10);
        assert_eq!(engine.current_level, 0);
        assert_eq!(engine.phase, LevelPhase::ArenaView);
    }

    #[test]
    fn start_level_initializes_strategy() {
        let mut engine = GameSemanticsEngine::default();
        engine.start_level(0);
        assert!(engine.strategy.is_some());
        // Level 1 has 1 pre-placed opponent move.
        assert_eq!(engine.strategy.as_ref().unwrap().moves.len(), 1);
    }

    #[test]
    fn advance_phase_cycle() {
        let mut engine = GameSemanticsEngine::default();
        engine.start_level(0);
        assert_eq!(engine.phase, LevelPhase::ArenaView);
        engine.advance_phase();
        assert_eq!(engine.phase, LevelPhase::StrategyBuilder);
        engine.advance_phase();
        assert_eq!(engine.phase, LevelPhase::Execution);
        engine.advance_phase();
        assert_eq!(engine.phase, LevelPhase::Results);
    }

    #[test]
    fn add_and_undo_move() {
        let mut engine = GameSemanticsEngine::default();
        engine.start_level(0);
        let initial_len = engine.strategy.as_ref().unwrap().moves.len();
        engine.add_player_move(1); // P: tt
        assert_eq!(
            engine.strategy.as_ref().unwrap().moves.len(),
            initial_len + 1
        );
        engine.undo_move();
        assert_eq!(engine.strategy.as_ref().unwrap().moves.len(), initial_len);
    }

    #[test]
    fn available_moves_respects_causality() {
        let mut engine = GameSemanticsEngine::default();
        engine.start_level(0); // Bool arena, O:q already played
        let available = engine.available_moves();
        // After O plays q (event 0), P can play tt (1) or ff (2).
        assert!(available.contains(&1));
        assert!(available.contains(&2));
        // q (0) already played, should not be available.
        assert!(!available.contains(&0));
    }

    #[test]
    fn step_execution() {
        let mut engine = GameSemanticsEngine::default();
        engine.start_level(0);
        engine.add_player_move(1);
        engine.phase = LevelPhase::Execution;
        engine.execution_step = 0;
        assert!(engine.step_execution());
        assert_eq!(engine.execution_step, 1);
    }

    #[test]
    fn levels_completed_tracking() {
        let mut engine = GameSemanticsEngine::default();
        engine.start_level(0);
        engine.add_player_move(1);
        // Advance through all phases to complete.
        engine.advance_phase(); // ArenaView -> Builder
        engine.advance_phase(); // Builder -> Execution
        engine.advance_phase(); // Execution -> Results
        engine.advance_phase(); // Results -> ArenaView (marks complete)
        assert!(engine.levels_completed[0]);
    }
}

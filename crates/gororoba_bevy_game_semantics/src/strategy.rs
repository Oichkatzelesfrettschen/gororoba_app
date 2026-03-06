// Strategy: a set of moves in an arena satisfying game-semantic constraints.
//
// A strategy is a consistent selection of Player moves in response to
// Opponent moves, respecting causality and alternation. Validation
// checks are progressive: basic constraints first, then conditions
// like well-bracketing and innocence.

use crate::arena::{Arena, Polarity};

/// A strategy over an arena: the selected sequence of moves.
#[derive(Debug, Clone)]
pub struct Strategy {
    /// The arena this strategy operates on.
    pub arena: Arena,
    /// Ordered sequence of event indices chosen by the player.
    /// Includes both Player and Opponent moves (a play).
    pub moves: Vec<usize>,
}

/// Validation error types for strategies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrategyError {
    /// A move references an event that does not exist in the arena.
    InvalidEvent(usize),
    /// A move is played before its justifier (causal dependency violated).
    CausalityViolation { move_id: usize, justifier: usize },
    /// Two consecutive moves have the same polarity (alternation violated).
    AlternationViolation { position: usize },
    /// The first move is not an Opponent move (games start with O).
    FirstMoveNotOpponent,
    /// A duplicate move appears in the sequence.
    DuplicateMove(usize),
}

impl Strategy {
    /// Create a new strategy with an arena and no moves.
    pub fn new(arena: Arena) -> Self {
        Self {
            arena,
            moves: Vec::new(),
        }
    }

    /// Add a move to the strategy. Returns the index in the move sequence.
    pub fn add_move(&mut self, event_id: usize) -> usize {
        let idx = self.moves.len();
        self.moves.push(event_id);
        idx
    }

    /// Remove the last move (undo).
    pub fn undo(&mut self) -> Option<usize> {
        self.moves.pop()
    }

    /// Clear all moves.
    pub fn clear(&mut self) {
        self.moves.clear();
    }

    /// Validate basic structural constraints: event existence, no duplicates.
    pub fn validate_structure(&self) -> Vec<StrategyError> {
        let mut errors = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for &m in &self.moves {
            if m >= self.arena.events.len() {
                errors.push(StrategyError::InvalidEvent(m));
            } else if !seen.insert(m) {
                errors.push(StrategyError::DuplicateMove(m));
            }
        }

        errors
    }

    /// Validate causality: every move's justifier must appear earlier.
    pub fn validate_causality(&self) -> Vec<StrategyError> {
        let mut errors = Vec::new();
        let mut played = std::collections::HashSet::new();

        for &m in &self.moves {
            if m < self.arena.events.len()
                && let Some(j) = self.arena.events[m].justifier
                && !played.contains(&j)
            {
                errors.push(StrategyError::CausalityViolation {
                    move_id: m,
                    justifier: j,
                });
            }
            played.insert(m);
        }

        errors
    }

    /// Validate alternation: Player and Opponent moves must alternate.
    /// The first move of a play is typically an Opponent move.
    pub fn validate_alternation(&self) -> Vec<StrategyError> {
        let mut errors = Vec::new();

        if self.moves.is_empty() {
            return errors;
        }

        // First move should be Opponent.
        let first = self.moves[0];
        if first < self.arena.events.len()
            && self.arena.events[first].polarity != Polarity::Opponent
        {
            errors.push(StrategyError::FirstMoveNotOpponent);
        }

        // Consecutive moves must alternate polarity.
        for i in 1..self.moves.len() {
            let prev = self.moves[i - 1];
            let curr = self.moves[i];
            if prev < self.arena.events.len()
                && curr < self.arena.events.len()
                && self.arena.events[prev].polarity == self.arena.events[curr].polarity
            {
                errors.push(StrategyError::AlternationViolation { position: i });
            }
        }

        errors
    }

    /// Run all basic validations.
    pub fn validate(&self) -> Vec<StrategyError> {
        let mut errors = self.validate_structure();
        errors.extend(self.validate_causality());
        errors.extend(self.validate_alternation());
        errors
    }

    /// Check if the strategy is complete and valid (no errors).
    pub fn is_valid(&self) -> bool {
        self.validate().is_empty()
    }

    /// Get the polarity sequence of the current moves.
    pub fn polarity_sequence(&self) -> Vec<Polarity> {
        self.moves
            .iter()
            .filter_map(|&m| {
                if m < self.arena.events.len() {
                    Some(self.arena.events[m].polarity)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get all Player moves in this strategy.
    pub fn player_moves(&self) -> Vec<usize> {
        self.moves
            .iter()
            .copied()
            .filter(|&m| {
                m < self.arena.events.len() && self.arena.events[m].polarity == Polarity::Player
            })
            .collect()
    }

    /// Get all Opponent moves in this strategy.
    pub fn opponent_moves(&self) -> Vec<usize> {
        self.moves
            .iter()
            .copied()
            .filter(|&m| {
                m < self.arena.events.len() && self.arena.events[m].polarity == Polarity::Opponent
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::{Arena, arena_bool, arena_unit};

    #[test]
    fn empty_strategy_is_valid() {
        let s = Strategy::new(arena_unit());
        assert!(s.is_valid());
    }

    #[test]
    fn valid_unit_strategy() {
        let a = arena_unit();
        let mut s = Strategy::new(a);
        s.add_move(0); // O: q
        s.add_move(1); // P: *
        assert!(s.is_valid());
    }

    #[test]
    fn alternation_violation() {
        let a = arena_bool();
        let mut s = Strategy::new(a);
        s.add_move(0); // O: q
        s.add_move(1); // P: tt
        s.add_move(2); // P: ff (same polarity as previous)
        let errors = s.validate_alternation();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, StrategyError::AlternationViolation { .. }))
        );
    }

    #[test]
    fn causality_violation() {
        let a = arena_unit();
        let mut s = Strategy::new(a);
        // Play answer before question (justifier not yet played).
        s.add_move(1); // P: * (justifier is 0, not yet played)
        let errors = s.validate_causality();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, StrategyError::CausalityViolation { .. }))
        );
    }

    #[test]
    fn duplicate_move() {
        let a = arena_unit();
        let mut s = Strategy::new(a);
        s.add_move(0);
        s.add_move(0);
        let errors = s.validate_structure();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, StrategyError::DuplicateMove(0)))
        );
    }

    #[test]
    fn undo_removes_last() {
        let a = arena_unit();
        let mut s = Strategy::new(a);
        s.add_move(0);
        s.add_move(1);
        assert_eq!(s.undo(), Some(1));
        assert_eq!(s.moves.len(), 1);
    }

    #[test]
    fn function_arena_strategy() {
        let u = arena_unit();
        let f = Arena::function_arena(&u, &u);
        let mut s = Strategy::new(f);
        // Copycat strategy for U -> U:
        // The codomain initial question is justified by domain initial.
        // In function arena: domain events [0,1] with flipped polarity,
        // codomain events [2,3] with original polarity.
        // Event 0: P (was O, flipped) - domain question
        // Event 1: O (was P, flipped) - domain answer
        // Event 2: O - codomain question (justified by 0)
        // Event 3: P - codomain answer
        s.add_move(2); // O: codomain question
        s.add_move(0); // P: domain question (copycat: forward the question)
        s.add_move(1); // O: domain answer
        s.add_move(3); // P: codomain answer (copycat: forward the answer)
        assert!(s.is_valid());
    }
}

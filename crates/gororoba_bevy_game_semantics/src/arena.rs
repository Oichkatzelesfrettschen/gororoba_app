// Arena and event types from game semantics.
//
// An arena is a game board with events (moves) that have polarity
// (Player or Opponent) and kind (Question or Answer). Events are
// connected by enabling relations that define causal dependencies.
//
// Based on Castellan & Clairambault, "Disentangling Parallelism and
// Interference in Game Semantics" (LMCS 2024), Section 2.

/// Who makes a move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Polarity {
    /// Player (proponent, +, blue). Represents the program.
    Player,
    /// Opponent (-, red). Represents the environment.
    Opponent,
}

impl Polarity {
    pub fn flip(self) -> Self {
        match self {
            Self::Player => Self::Opponent,
            Self::Opponent => Self::Player,
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Player => "+",
            Self::Opponent => "-",
        }
    }
}

/// Whether a move asks or answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoveKind {
    /// A question initiates an interaction.
    Question,
    /// An answer completes an interaction.
    Answer,
}

impl MoveKind {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Question => "q",
            Self::Answer => "a",
        }
    }
}

/// A single event (move) in an arena.
#[derive(Debug, Clone)]
pub struct Event {
    /// Unique index within the arena.
    pub id: usize,
    /// Who plays this move.
    pub polarity: Polarity,
    /// Question or answer.
    pub kind: MoveKind,
    /// Human-readable label.
    pub label: String,
    /// Justifier: the enabling event (None for initial moves).
    pub justifier: Option<usize>,
}

/// An arena: a set of events with enabling relations.
///
/// Arenas represent types in game semantics. The product A x B combines
/// two arenas; the function A -> B reverses polarity of A's events.
#[derive(Debug, Clone)]
pub struct Arena {
    pub name: String,
    pub events: Vec<Event>,
    /// Explicit enabling edges (source, target). The target event is
    /// enabled by (causally depends on) the source event.
    pub enabling: Vec<(usize, usize)>,
}

impl Arena {
    /// Create a new empty arena.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            events: Vec::new(),
            enabling: Vec::new(),
        }
    }

    /// Add an event and return its id.
    pub fn add_event(
        &mut self,
        polarity: Polarity,
        kind: MoveKind,
        label: impl Into<String>,
        justifier: Option<usize>,
    ) -> usize {
        let id = self.events.len();
        self.events.push(Event {
            id,
            polarity,
            kind,
            label: label.into(),
            justifier,
        });
        if let Some(j) = justifier {
            self.enabling.push((j, id));
        }
        id
    }

    /// Get all initial moves (events with no justifier).
    pub fn initial_moves(&self) -> Vec<usize> {
        self.events
            .iter()
            .filter(|e| e.justifier.is_none())
            .map(|e| e.id)
            .collect()
    }

    /// Get events directly enabled by the given event.
    pub fn children(&self, event_id: usize) -> Vec<usize> {
        self.enabling
            .iter()
            .filter(|(src, _)| *src == event_id)
            .map(|(_, tgt)| *tgt)
            .collect()
    }

    /// Build a function arena A -> B. This reverses polarity of A's events
    /// and composes both arenas, with B's initial moves justified by A's
    /// initial moves.
    pub fn function_arena(domain: &Arena, codomain: &Arena) -> Self {
        let mut arena = Arena::new(format!("{} -> {}", domain.name, codomain.name));

        // Add domain events with flipped polarity.
        let domain_offset = 0;
        for e in &domain.events {
            arena.events.push(Event {
                id: domain_offset + e.id,
                polarity: e.polarity.flip(),
                kind: e.kind,
                label: e.label.clone(),
                justifier: e.justifier.map(|j| domain_offset + j),
            });
        }
        for &(s, t) in &domain.enabling {
            arena.enabling.push((domain_offset + s, domain_offset + t));
        }

        // Add codomain events with original polarity.
        let codomain_offset = domain.events.len();
        for e in &codomain.events {
            arena.events.push(Event {
                id: codomain_offset + e.id,
                polarity: e.polarity,
                kind: e.kind,
                label: e.label.clone(),
                justifier: e.justifier.map(|j| codomain_offset + j),
            });
        }
        for &(s, t) in &codomain.enabling {
            arena
                .enabling
                .push((codomain_offset + s, codomain_offset + t));
        }

        // Codomain initial questions are justified by domain initial questions.
        let domain_initials: Vec<usize> = domain
            .events
            .iter()
            .filter(|e| e.justifier.is_none())
            .map(|e| domain_offset + e.id)
            .collect();
        let codomain_initials: Vec<usize> = codomain
            .events
            .iter()
            .filter(|e| e.justifier.is_none())
            .map(|e| codomain_offset + e.id)
            .collect();

        // Codomain initial moves are the initial moves of the function game.
        // They have no justifier (play starts with an Opponent move in the
        // codomain). The enabling edges record the arena structure but do
        // not constrain play order for initial moves.
        for &ci in &codomain_initials {
            for &di in &domain_initials {
                arena.enabling.push((di, ci));
            }
        }

        arena
    }

    /// Build a product arena A x B (just concatenate, no cross-enabling).
    pub fn product_arena(a: &Arena, b: &Arena) -> Self {
        let mut arena = Arena::new(format!("{} x {}", a.name, b.name));

        for e in &a.events {
            arena.events.push(Event {
                id: e.id,
                polarity: e.polarity,
                kind: e.kind,
                label: e.label.clone(),
                justifier: e.justifier,
            });
        }
        for &(s, t) in &a.enabling {
            arena.enabling.push((s, t));
        }

        let offset = a.events.len();
        for e in &b.events {
            arena.events.push(Event {
                id: offset + e.id,
                polarity: e.polarity,
                kind: e.kind,
                label: e.label.clone(),
                justifier: e.justifier.map(|j| offset + j),
            });
        }
        for &(s, t) in &b.enabling {
            arena.enabling.push((offset + s, offset + t));
        }

        arena
    }
}

/// Standard arenas for basic types.
pub fn arena_unit() -> Arena {
    // Unit type: single Opponent question, single Player answer.
    let mut a = Arena::new("U");
    let q = a.add_event(Polarity::Opponent, MoveKind::Question, "q", None);
    a.add_event(Polarity::Player, MoveKind::Answer, "*", Some(q));
    a
}

pub fn arena_bool() -> Arena {
    // Bool type: Opponent question, Player answers tt or ff.
    let mut a = Arena::new("B");
    let q = a.add_event(Polarity::Opponent, MoveKind::Question, "q", None);
    a.add_event(Polarity::Player, MoveKind::Answer, "tt", Some(q));
    a.add_event(Polarity::Player, MoveKind::Answer, "ff", Some(q));
    a
}

pub fn arena_nat() -> Arena {
    // Nat type: Opponent question, Player answers with a number 0-3.
    let mut a = Arena::new("N");
    let q = a.add_event(Polarity::Opponent, MoveKind::Question, "q", None);
    a.add_event(Polarity::Player, MoveKind::Answer, "0", Some(q));
    a.add_event(Polarity::Player, MoveKind::Answer, "1", Some(q));
    a.add_event(Polarity::Player, MoveKind::Answer, "2", Some(q));
    a.add_event(Polarity::Player, MoveKind::Answer, "3", Some(q));
    a
}

/// Arena for a mutable reference cell (ref type).
/// Contains read/write operations with question-answer pairs.
pub fn arena_ref() -> Arena {
    let mut a = Arena::new("ref");
    let q_read = a.add_event(Polarity::Opponent, MoveKind::Question, "read", None);
    a.add_event(Polarity::Player, MoveKind::Answer, "val", Some(q_read));
    let q_write = a.add_event(Polarity::Opponent, MoveKind::Question, "write", None);
    a.add_event(Polarity::Player, MoveKind::Answer, "ok", Some(q_write));
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_arena_structure() {
        let a = arena_unit();
        assert_eq!(a.events.len(), 2);
        assert_eq!(a.events[0].polarity, Polarity::Opponent);
        assert_eq!(a.events[1].polarity, Polarity::Player);
        assert_eq!(a.enabling.len(), 1);
    }

    #[test]
    fn bool_arena_has_two_answers() {
        let a = arena_bool();
        assert_eq!(a.events.len(), 3);
        let answers: Vec<_> = a
            .events
            .iter()
            .filter(|e| e.kind == MoveKind::Answer)
            .collect();
        assert_eq!(answers.len(), 2);
    }

    #[test]
    fn function_arena_reverses_domain_polarity() {
        let u = arena_unit();
        let b = arena_bool();
        let f = Arena::function_arena(&u, &b);
        // Domain events have flipped polarity.
        assert_eq!(f.events[0].polarity, Polarity::Player);
        assert_eq!(f.events[1].polarity, Polarity::Opponent);
        // Codomain events keep original polarity.
        assert_eq!(f.events[2].polarity, Polarity::Opponent);
    }

    #[test]
    fn product_arena_combines_events() {
        let u = arena_unit();
        let b = arena_bool();
        let p = Arena::product_arena(&u, &b);
        assert_eq!(p.events.len(), u.events.len() + b.events.len());
    }

    #[test]
    fn polarity_flip() {
        assert_eq!(Polarity::Player.flip(), Polarity::Opponent);
        assert_eq!(Polarity::Opponent.flip(), Polarity::Player);
    }

    #[test]
    fn initial_moves_of_bool() {
        let a = arena_bool();
        let initials = a.initial_moves();
        assert_eq!(initials.len(), 1);
        assert_eq!(a.events[initials[0]].kind, MoveKind::Question);
    }
}

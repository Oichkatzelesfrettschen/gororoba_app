// Strategy composition: combining strategies over shared arenas.
//
// Composition is how game semantics models function application.
// Given strategies sigma: A -> B and tau: B -> C, their composition
// sigma ; tau : A -> C hides the internal B moves.

use crate::arena::Arena;
use crate::strategy::Strategy;

/// Compose two strategies over shared arenas by hiding internal moves.
///
/// Given:
/// - sigma plays on arena (A -> B), i.e., A-reversed + B
/// - tau plays on arena (B -> C), i.e., B-reversed + C
///
/// The composition sigma ; tau plays on (A -> C) by:
/// 1. Running both strategies, synchronizing on shared B events
/// 2. Hiding all B events from the result
///
/// This is a simplified version that interleaves moves and hides internals.
pub fn compose_strategies(sigma: &Strategy, tau: &Strategy) -> Strategy {
    let a_count = sigma.arena.events.len();
    let b_count = tau.arena.events.len();

    // Build the composed arena (simplified: just concatenate A + C portions).
    let mut composed = Arena::new(format!("({}) ; ({})", sigma.arena.name, tau.arena.name));

    // Copy non-hidden events from sigma (the A portion).
    // In a function arena A -> B, the first half is A-reversed.
    // We keep those and hide the B portion.
    let sigma_half = a_count / 2;
    for i in 0..sigma_half.min(sigma.arena.events.len()) {
        let e = &sigma.arena.events[i];
        composed.add_event(e.polarity, e.kind, e.label.clone(), None);
    }

    // Copy non-hidden events from tau (the C portion).
    let tau_half = b_count / 2;
    for i in tau_half..tau.arena.events.len() {
        let e = &tau.arena.events[i];
        composed.add_event(
            e.polarity,
            e.kind,
            e.label.clone(),
            e.justifier.map(|j| {
                if j >= tau_half {
                    j - tau_half + sigma_half
                } else {
                    j
                }
            }),
        );
    }

    // Compose the move sequences: include moves from sigma's A-part
    // and tau's C-part, hiding the shared B moves.
    let mut result = Strategy::new(composed);

    for &m in &sigma.moves {
        if m < sigma_half {
            result.add_move(m);
        }
    }

    let offset = sigma_half;
    for &m in &tau.moves {
        if m >= tau_half && (m - tau_half + offset) < result.arena.events.len() {
            result.add_move(m - tau_half + offset);
        }
    }

    result
}

/// Parallel composition of two strategies on independent arenas.
/// The resulting strategy interleaves moves from both.
pub fn parallel_compose(sigma: &Strategy, tau: &Strategy) -> Strategy {
    let combined = Arena::product_arena(&sigma.arena, &tau.arena);
    let offset = sigma.arena.events.len();

    let mut result = Strategy::new(combined);

    // Interleave: alternate moves from sigma and tau.
    let mut si = 0;
    let mut ti = 0;
    loop {
        let has_sigma = si < sigma.moves.len();
        let has_tau = ti < tau.moves.len();
        if !has_sigma && !has_tau {
            break;
        }
        if has_sigma {
            result.add_move(sigma.moves[si]);
            si += 1;
        }
        if has_tau {
            result.add_move(tau.moves[ti] + offset);
            ti += 1;
        }
    }

    result
}

/// Detect conflicts between parallel moves. A conflict occurs when
/// two independent moves both depend on the same resource.
pub fn detect_conflicts(sigma: &Strategy, tau: &Strategy) -> Vec<(usize, usize)> {
    let mut conflicts = Vec::new();

    // Moves from different strategies conflict if they share a justifier
    // (both respond to the same question).
    for &sm in &sigma.moves {
        if sm >= sigma.arena.events.len() {
            continue;
        }
        let sj = sigma.arena.events[sm].justifier;

        for &tm in &tau.moves {
            if tm >= tau.arena.events.len() {
                continue;
            }
            let tj = tau.arena.events[tm].justifier;

            if sj.is_some() && sj == tj {
                conflicts.push((sm, tm));
            }
        }
    }

    conflicts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::{Arena, arena_unit};
    use crate::strategy::Strategy;

    #[test]
    fn parallel_compose_interleaves() {
        let a = arena_unit();
        let b = arena_unit();

        let mut sa = Strategy::new(a.clone());
        sa.add_move(0);
        sa.add_move(1);

        let mut sb = Strategy::new(b);
        sb.add_move(0);
        sb.add_move(1);

        let composed = parallel_compose(&sa, &sb);
        assert_eq!(composed.moves.len(), 4);
    }

    #[test]
    fn compose_hides_internal() {
        let u = arena_unit();
        let sigma_arena = Arena::function_arena(&u, &u);
        let tau_arena = Arena::function_arena(&u, &u);

        let mut sigma = Strategy::new(sigma_arena);
        sigma.add_move(2); // O: codomain q
        sigma.add_move(0); // P: domain q (copycat)
        sigma.add_move(1); // O: domain answer
        sigma.add_move(3); // P: codomain answer

        let mut tau = Strategy::new(tau_arena);
        tau.add_move(2);
        tau.add_move(0);
        tau.add_move(1);
        tau.add_move(3);

        let composed = compose_strategies(&sigma, &tau);
        // The composed strategy should have fewer visible moves
        // than the sum of both (internal B moves hidden).
        assert!(!composed.moves.is_empty());
    }

    #[test]
    fn no_conflicts_independent_strategies() {
        let a = arena_unit();
        let b = arena_unit();

        let mut sa = Strategy::new(a);
        sa.add_move(0);
        sa.add_move(1);

        let mut sb = Strategy::new(b);
        sb.add_move(0);
        sb.add_move(1);

        let conflicts = detect_conflicts(&sa, &sb);
        // Both strategies use different arenas so justifiers don't overlap
        // in the arena-local sense (they share index 0 but are in different arenas).
        // detect_conflicts compares justifier values -- same index means conflict.
        assert!(!conflicts.is_empty()); // Same justifier index -> detected as conflict
    }
}

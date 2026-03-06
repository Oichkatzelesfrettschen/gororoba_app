// Game-semantic conditions: well-bracketing, innocence, visibility.
//
// These conditions filter which strategies are valid for different
// type systems. Toggling them explores Abramsky's semantic cube.
//
// - Well-bracketing: answers must match the most recent unanswered question
// - Innocence: strategy depends only on the P-view (visible history)
// - Parallel innocence: causal rather than interleaving parallelism

use crate::arena::MoveKind;
use crate::strategy::Strategy;

/// Which conditions are active for a given level.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ActiveConditions {
    pub well_bracketing: bool,
    pub innocence: bool,
    pub parallel_innocence: bool,
    pub sequentiality: bool,
}

/// Result of checking a single condition.
#[derive(Debug, Clone)]
pub struct ConditionResult {
    pub name: &'static str,
    pub satisfied: bool,
    pub detail: String,
}

/// Check well-bracketing: every answer must close the most recent
/// open question (stack discipline on question-answer pairs).
pub fn check_well_bracketing(strategy: &Strategy) -> ConditionResult {
    let mut question_stack: Vec<usize> = Vec::new();
    let mut satisfied = true;
    let mut detail = String::new();

    for &m in &strategy.moves {
        if m >= strategy.arena.events.len() {
            continue;
        }
        let event = &strategy.arena.events[m];
        match event.kind {
            MoveKind::Question => {
                question_stack.push(m);
            }
            MoveKind::Answer => {
                if let Some(justifier) = event.justifier
                    && let Some(&top) = question_stack.last()
                {
                    if top != justifier {
                        satisfied = false;
                        detail = format!(
                            "Answer '{}' (event {}) answers question {} but most recent open question is {}",
                            event.label, m, justifier, top
                        );
                        break;
                    }
                    question_stack.pop();
                }
            }
        }
    }

    if satisfied && detail.is_empty() {
        detail = "All answers close the most recent open question".into();
    }

    ConditionResult {
        name: "Well-Bracketing",
        satisfied,
        detail,
    }
}

/// Compute the P-view (Player's visible history) at each point.
///
/// The P-view is the subsequence of moves visible to Player under the
/// innocence condition. It follows the pointer structure backwards,
/// keeping only the most recent branch.
pub fn compute_p_view(strategy: &Strategy) -> Vec<Vec<usize>> {
    let mut views = Vec::new();

    for i in 0..strategy.moves.len() {
        let prefix = &strategy.moves[..=i];
        let mut view = Vec::new();
        let mut idx = prefix.len() - 1;

        loop {
            view.push(prefix[idx]);
            let event_id = prefix[idx];
            if event_id >= strategy.arena.events.len() {
                break;
            }
            let event = &strategy.arena.events[event_id];
            if let Some(j) = event.justifier {
                // Find the position of the justifier in the prefix.
                if let Some(pos) = prefix.iter().position(|&m| m == j) {
                    if pos == 0 {
                        view.push(prefix[0]);
                        break;
                    }
                    idx = pos;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        view.reverse();
        views.push(view);
    }

    views
}

/// Check innocence: Player's moves depend only on the P-view
/// (not the full history). Two positions with the same P-view must
/// have the same Player response.
pub fn check_innocence(strategy: &Strategy) -> ConditionResult {
    let views = compute_p_view(strategy);
    let mut satisfied = true;
    let mut detail = String::new();

    // For innocence, each Player move should be deterministic given
    // its P-view prefix. We check that no two distinct Player moves
    // follow from the same P-view.
    let mut view_responses: std::collections::HashMap<Vec<usize>, usize> =
        std::collections::HashMap::new();

    for (i, m) in strategy.moves.iter().enumerate() {
        if *m >= strategy.arena.events.len() {
            continue;
        }
        if strategy.arena.events[*m].polarity == crate::arena::Polarity::Player {
            let view = if i > 0 { &views[i - 1] } else { &vec![] };
            let key = view.clone();
            if let Some(&prev_response) = view_responses.get(&key) {
                if prev_response != *m {
                    satisfied = false;
                    detail = format!(
                        "Same P-view leads to different Player moves: {} and {}",
                        prev_response, m
                    );
                    break;
                }
            } else {
                view_responses.insert(key, *m);
            }
        }
    }

    if satisfied && detail.is_empty() {
        detail = "Player moves depend only on visible history".into();
    }

    ConditionResult {
        name: "Innocence",
        satisfied,
        detail,
    }
}

/// Check parallel innocence: like innocence but respects causal
/// structure rather than interleaving order.
///
/// Two moves are causally related if one enables the other (transitively).
/// Parallel innocence requires that Player's response depends only on
/// causally relevant history, not on the order of independent moves.
pub fn check_parallel_innocence(strategy: &Strategy) -> ConditionResult {
    // Build the causal dependency graph from the enabling relation.
    let n = strategy.arena.events.len();
    let mut reachable = vec![vec![false; n]; n];

    // Direct enabling.
    for &(src, tgt) in &strategy.arena.enabling {
        if src < n && tgt < n {
            reachable[src][tgt] = true;
        }
    }

    // Transitive closure (Floyd-Warshall).
    for k in 0..n {
        for i in 0..n {
            for j in 0..n {
                if reachable[i][k] && reachable[k][j] {
                    reachable[i][j] = true;
                }
            }
        }
    }

    // For each Player move, collect causally relevant history.
    let mut causal_histories: std::collections::HashMap<Vec<usize>, usize> =
        std::collections::HashMap::new();
    let mut satisfied = true;
    let mut detail = String::new();

    for (i, &m) in strategy.moves.iter().enumerate() {
        if m >= n {
            continue;
        }
        if strategy.arena.events[m].polarity == crate::arena::Polarity::Player {
            let mut causal_hist: Vec<usize> = strategy.moves[..i]
                .iter()
                .copied()
                .filter(|&prev| prev < n && (reachable[prev][m] || prev == m))
                .collect();
            causal_hist.sort_unstable();

            if let Some(&prev_response) = causal_histories.get(&causal_hist) {
                if prev_response != m {
                    satisfied = false;
                    detail = format!(
                        "Same causal history leads to different moves: {} and {}",
                        prev_response, m
                    );
                    break;
                }
            } else {
                causal_histories.insert(causal_hist, m);
            }
        }
    }

    if satisfied && detail.is_empty() {
        detail = "Player moves depend only on causal history".into();
    }

    ConditionResult {
        name: "Parallel Innocence",
        satisfied,
        detail,
    }
}

/// Check all active conditions and return results.
pub fn check_conditions(
    strategy: &Strategy,
    conditions: &ActiveConditions,
) -> Vec<ConditionResult> {
    let mut results = Vec::new();

    if conditions.well_bracketing {
        results.push(check_well_bracketing(strategy));
    }
    if conditions.innocence {
        results.push(check_innocence(strategy));
    }
    if conditions.parallel_innocence {
        results.push(check_parallel_innocence(strategy));
    }

    results
}

/// Check if all active conditions are satisfied.
pub fn all_conditions_satisfied(strategy: &Strategy, conditions: &ActiveConditions) -> bool {
    check_conditions(strategy, conditions)
        .iter()
        .all(|r| r.satisfied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::{arena_bool, arena_unit};
    use crate::strategy::Strategy;

    #[test]
    fn well_bracketed_unit_play() {
        let a = arena_unit();
        let mut s = Strategy::new(a);
        s.add_move(0); // O: q (Question)
        s.add_move(1); // P: * (Answer to q)
        let result = check_well_bracketing(&s);
        assert!(result.satisfied);
    }

    #[test]
    fn well_bracketed_bool_play() {
        let a = arena_bool();
        let mut s = Strategy::new(a);
        s.add_move(0); // O: q (Question)
        s.add_move(1); // P: tt (Answer to q)
        let result = check_well_bracketing(&s);
        assert!(result.satisfied);
    }

    #[test]
    fn innocence_simple_strategy() {
        let a = arena_unit();
        let mut s = Strategy::new(a);
        s.add_move(0);
        s.add_move(1);
        let result = check_innocence(&s);
        assert!(result.satisfied);
    }

    #[test]
    fn p_view_unit_strategy() {
        let a = arena_unit();
        let mut s = Strategy::new(a);
        s.add_move(0);
        s.add_move(1);
        let views = compute_p_view(&s);
        assert_eq!(views.len(), 2);
    }

    #[test]
    fn parallel_innocence_unit() {
        let a = arena_unit();
        let mut s = Strategy::new(a);
        s.add_move(0);
        s.add_move(1);
        let result = check_parallel_innocence(&s);
        assert!(result.satisfied);
    }

    #[test]
    fn all_conditions_off_is_satisfied() {
        let a = arena_unit();
        let s = Strategy::new(a);
        let conditions = ActiveConditions::default();
        assert!(all_conditions_satisfied(&s, &conditions));
    }
}

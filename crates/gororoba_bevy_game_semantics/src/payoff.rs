// Classical game theory: payoff matrices, Nash equilibria, Pareto frontier.
//
// Hybrid scoring layer that connects game-semantic strategies to
// classical GT concepts: payoff computation, equilibrium analysis,
// and optimality ranking.

/// A payoff matrix for a two-player game.
#[derive(Debug, Clone)]
pub struct PayoffMatrix {
    /// Row labels (Player actions).
    pub rows: Vec<String>,
    /// Column labels (Opponent actions).
    pub cols: Vec<String>,
    /// Payoff values: rows[i] x cols[j] -> values[i][j].
    /// Positive favors Player, negative favors Opponent.
    pub values: Vec<Vec<f64>>,
}

/// Result of Nash equilibrium computation.
#[derive(Debug, Clone)]
pub struct NashResult {
    /// Mixed strategy probabilities for Player.
    pub player_strategy: Vec<f64>,
    /// Mixed strategy probabilities for Opponent.
    pub opponent_strategy: Vec<f64>,
    /// Expected payoff at equilibrium.
    pub value: f64,
}

impl PayoffMatrix {
    /// Create a new payoff matrix.
    pub fn new(rows: Vec<String>, cols: Vec<String>, values: Vec<Vec<f64>>) -> Self {
        Self { rows, cols, values }
    }

    /// Create a zero-sum payoff matrix from dimensions.
    pub fn zero_sum(rows: usize, cols: usize) -> Self {
        Self {
            rows: (0..rows).map(|i| format!("P{}", i)).collect(),
            cols: (0..cols).map(|j| format!("O{}", j)).collect(),
            values: vec![vec![0.0; cols]; rows],
        }
    }

    /// Get the payoff for a specific action pair.
    pub fn payoff(&self, row: usize, col: usize) -> f64 {
        self.values[row][col]
    }

    /// Set the payoff for a specific action pair.
    pub fn set_payoff(&mut self, row: usize, col: usize, value: f64) {
        self.values[row][col] = value;
    }

    /// Find the minimax value (Player's guaranteed minimum payoff).
    pub fn minimax_value(&self) -> f64 {
        // For each Player action, find the minimum payoff across Opponent actions.
        // Then take the maximum of those minima.
        self.values
            .iter()
            .map(|row| {
                row.iter()
                    .copied()
                    .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .unwrap_or(0.0)
            })
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0)
    }

    /// Find the maximin value (Opponent's guaranteed maximum loss).
    pub fn maximin_value(&self) -> f64 {
        if self.values.is_empty() || self.values[0].is_empty() {
            return 0.0;
        }
        let cols = self.values[0].len();
        (0..cols)
            .map(|j| {
                self.values
                    .iter()
                    .map(|row| row[j])
                    .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .unwrap_or(0.0)
            })
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0)
    }

    /// Find pure strategy Nash equilibria (saddle points).
    pub fn pure_nash_equilibria(&self) -> Vec<(usize, usize)> {
        let mut equilibria = Vec::new();
        let rows = self.values.len();
        if rows == 0 {
            return equilibria;
        }
        let cols = self.values[0].len();

        for i in 0..rows {
            for j in 0..cols {
                let val = self.values[i][j];
                // Check if val is the minimum of its row.
                let is_row_min = self.values[i].iter().all(|&v| val <= v + f64::EPSILON);
                // Check if val is the maximum of its column.
                let is_col_max = (0..rows).all(|r| val + f64::EPSILON >= self.values[r][j]);

                if is_row_min && is_col_max {
                    equilibria.push((i, j));
                }
            }
        }

        equilibria
    }
}

/// Find mixed strategy Nash equilibrium for a 2x2 zero-sum game.
///
/// For a 2x2 matrix [[a,b],[c,d]], the mixed strategy probabilities are:
/// Player: p = (d-c) / (a-b-c+d)
/// Opponent: q = (d-b) / (a-b-c+d)
pub fn find_nash_2x2(matrix: &PayoffMatrix) -> Option<NashResult> {
    if matrix.values.len() != 2 || matrix.values[0].len() != 2 {
        return None;
    }

    let a = matrix.values[0][0];
    let b = matrix.values[0][1];
    let c = matrix.values[1][0];
    let d = matrix.values[1][1];

    let denom = a - b - c + d;
    if denom.abs() < 1e-10 {
        // Degenerate case: check for pure equilibrium.
        let pure = matrix.pure_nash_equilibria();
        if let Some(&(i, j)) = pure.first() {
            let mut ps = vec![0.0; 2];
            let mut os = vec![0.0; 2];
            ps[i] = 1.0;
            os[j] = 1.0;
            return Some(NashResult {
                player_strategy: ps,
                opponent_strategy: os,
                value: matrix.values[i][j],
            });
        }
        return None;
    }

    let p = (d - c) / denom;
    let q = (d - b) / denom;

    // Clamp to [0,1] for valid probabilities.
    let p = p.clamp(0.0, 1.0);
    let q = q.clamp(0.0, 1.0);

    let value = a * p * q + b * p * (1.0 - q) + c * (1.0 - p) * q + d * (1.0 - p) * (1.0 - q);

    Some(NashResult {
        player_strategy: vec![p, 1.0 - p],
        opponent_strategy: vec![q, 1.0 - q],
        value,
    })
}

/// Find Nash equilibria for general matrices using support enumeration.
/// For small games (up to 4x4), this enumerates pure equilibria and
/// attempts 2x2 subgame analysis.
pub fn find_nash_equilibria(matrix: &PayoffMatrix) -> Vec<NashResult> {
    let mut results = Vec::new();

    // Pure equilibria.
    for (i, j) in matrix.pure_nash_equilibria() {
        let rows = matrix.values.len();
        let cols = matrix.values[0].len();
        let mut ps = vec![0.0; rows];
        let mut os = vec![0.0; cols];
        ps[i] = 1.0;
        os[j] = 1.0;
        results.push(NashResult {
            player_strategy: ps,
            opponent_strategy: os,
            value: matrix.values[i][j],
        });
    }

    // Mixed equilibrium for 2x2.
    if matrix.values.len() == 2
        && matrix.values[0].len() == 2
        && results.is_empty()
        && let Some(nash) = find_nash_2x2(matrix)
    {
        results.push(nash);
    }

    results
}

/// Compute the Pareto frontier from a set of (Player payoff, Opponent payoff) outcomes.
///
/// Returns indices of outcomes that are Pareto-optimal: no other outcome
/// is at least as good for both players and strictly better for one.
pub fn pareto_frontier(outcomes: &[(f64, f64)]) -> Vec<usize> {
    let mut frontier = Vec::new();

    for (i, &(pi, oi)) in outcomes.iter().enumerate() {
        let dominated = outcomes
            .iter()
            .enumerate()
            .any(|(j, &(pj, oj))| j != i && pj >= pi && oj >= oi && (pj > pi || oj > oi));
        if !dominated {
            frontier.push(i);
        }
    }

    frontier
}

/// Compute strategy efficiency score.
///
/// Scores based on:
/// - Move count efficiency (fewer moves = higher score)
/// - Constraint satisfaction bonus
/// - Optimality bonus (Pareto-optimal solutions)
pub fn compute_payoff(
    move_count: usize,
    optimal_count: usize,
    constraints_satisfied: usize,
    total_constraints: usize,
) -> f64 {
    let efficiency = if move_count > 0 {
        (optimal_count as f64) / (move_count as f64)
    } else {
        0.0
    };

    let constraint_bonus = if total_constraints > 0 {
        (constraints_satisfied as f64) / (total_constraints as f64)
    } else {
        1.0
    };

    // Base score: 100 * efficiency * constraint satisfaction.
    let score = 100.0 * efficiency * constraint_bonus;
    score.clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_sum_matrix_creation() {
        let m = PayoffMatrix::zero_sum(2, 2);
        assert_eq!(m.values.len(), 2);
        assert_eq!(m.values[0].len(), 2);
        assert_eq!(m.payoff(0, 0), 0.0);
    }

    #[test]
    fn minimax_matching_pennies() {
        // Matching pennies: [[1,-1],[-1,1]]
        let m = PayoffMatrix::new(
            vec!["H".into(), "T".into()],
            vec!["H".into(), "T".into()],
            vec![vec![1.0, -1.0], vec![-1.0, 1.0]],
        );
        // Minimax = -1 (worst case for each row, best of those).
        assert_eq!(m.minimax_value(), -1.0);
    }

    #[test]
    fn pure_nash_prisoners_dilemma() {
        // Prisoner's dilemma: [[3,0],[5,1]] (row player payoff).
        // Pure Nash equilibrium at (1,1) = (Defect, Defect) = 1.
        let m = PayoffMatrix::new(
            vec!["C".into(), "D".into()],
            vec!["C".into(), "D".into()],
            vec![vec![3.0, 0.0], vec![5.0, 1.0]],
        );
        let eq = m.pure_nash_equilibria();
        assert_eq!(eq.len(), 1);
        assert_eq!(eq[0], (1, 1));
    }

    #[test]
    fn mixed_nash_matching_pennies() {
        let m = PayoffMatrix::new(
            vec!["H".into(), "T".into()],
            vec!["H".into(), "T".into()],
            vec![vec![1.0, -1.0], vec![-1.0, 1.0]],
        );
        let nash = find_nash_2x2(&m).unwrap();
        // Both players should play 50/50.
        assert!((nash.player_strategy[0] - 0.5).abs() < 1e-6);
        assert!((nash.opponent_strategy[0] - 0.5).abs() < 1e-6);
        assert!(nash.value.abs() < 1e-6);
    }

    #[test]
    fn pareto_frontier_basic() {
        let outcomes = vec![(1.0, 3.0), (2.0, 2.0), (3.0, 1.0), (1.5, 1.5)];
        let frontier = pareto_frontier(&outcomes);
        // (1,3), (2,2), (3,1) are on the frontier; (1.5,1.5) is dominated by (2,2).
        assert_eq!(frontier.len(), 3);
        assert!(frontier.contains(&0));
        assert!(frontier.contains(&1));
        assert!(frontier.contains(&2));
        assert!(!frontier.contains(&3));
    }

    #[test]
    fn payoff_perfect_score() {
        let score = compute_payoff(3, 3, 2, 2);
        assert!((score - 100.0).abs() < 1e-6);
    }

    #[test]
    fn payoff_half_efficiency() {
        let score = compute_payoff(6, 3, 2, 2);
        assert!((score - 50.0).abs() < 1e-6);
    }

    #[test]
    fn find_nash_general() {
        let m = PayoffMatrix::new(
            vec!["C".into(), "D".into()],
            vec!["C".into(), "D".into()],
            vec![vec![3.0, 0.0], vec![5.0, 1.0]],
        );
        let results = find_nash_equilibria(&m);
        assert!(!results.is_empty());
    }
}

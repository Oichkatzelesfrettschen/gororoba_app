// Hypercomplex rotation, zero-divisor portal, and algebra domain components.
//
// These define the ECS data model for Cayley-Dickson algebra in Bevy.

use bevy::prelude::*;

pub use gororoba_kernel_api::algebra::AlgebraDimension;

/// Marker component for entities in a hypercomplex algebra domain.
#[derive(Component, Default)]
pub struct AlgebraDomain;

/// Configuration for the algebra simulation domain.
#[derive(Component)]
pub struct AlgebraParams {
    /// Which algebra dimension to operate in.
    pub dimension: AlgebraDimension,
    /// Tolerance for zero-divisor detection.
    pub zero_tolerance: f64,
    /// Whether to use parallel search algorithms.
    pub parallel_search: bool,
    /// Maximum blade order for zero-divisor search (2 or 3).
    pub max_blade_order: usize,
    /// Random seed for reproducibility in stochastic searches.
    pub seed: u64,
}

impl Default for AlgebraParams {
    fn default() -> Self {
        Self {
            dimension: AlgebraDimension::default(),
            zero_tolerance: 1e-12,
            parallel_search: true,
            max_blade_order: 2,
            seed: 42,
        }
    }
}

/// A hypercomplex element stored as coefficient vector.
///
/// Used to represent rotations, positions, or other algebraic objects
/// in the Cayley-Dickson algebra. The length of `coeffs` determines
/// the algebra dimension.
#[derive(Component, Clone)]
pub struct HypercomplexElement {
    /// Coefficients in the standard basis: a0 + a1*e1 + a2*e2 + ...
    pub coeffs: Vec<f64>,
}

impl HypercomplexElement {
    /// Create a unit element (1, 0, 0, ...) for the given dimension.
    pub fn identity(dim: usize) -> Self {
        let mut coeffs = vec![0.0; dim];
        coeffs[0] = 1.0;
        Self { coeffs }
    }

    /// Create a basis element e_i for the given dimension.
    pub fn basis(dim: usize, i: usize) -> Self {
        assert!(i < dim, "basis index {i} out of range for dim {dim}");
        let mut coeffs = vec![0.0; dim];
        coeffs[i] = 1.0;
        Self { coeffs }
    }

    /// Euclidean norm squared of the element.
    pub fn norm_sq(&self) -> f64 {
        self.coeffs.iter().map(|c| c * c).sum()
    }

    /// Euclidean norm.
    pub fn norm(&self) -> f64 {
        self.norm_sq().sqrt()
    }
}

/// A zero-divisor portal connecting two points in algebra space.
///
/// Zero-divisors are pairs (a, b) where a*b = 0 but a != 0 and b != 0.
/// In the game, these represent portal locations where geometry becomes
/// non-standard.
#[derive(Component)]
pub struct ZeroDivisorPortal {
    /// First element of the zero-divisor pair (basis indices).
    pub a_indices: (usize, usize),
    /// Second element of the zero-divisor pair (basis indices).
    pub b_indices: (usize, usize),
    /// Sign of the second blade's second basis term.
    pub rhs_sign: i8,
    /// Product norm (how close to zero; 0.0 = exact zero-divisor).
    pub product_norm: f64,
    /// Whether this portal is currently active (traversable).
    pub active: bool,
}

/// Diagnostic output from the algebra engine.
#[derive(Component, Default)]
pub struct AlgebraDiagnostics {
    /// Number of 2-blade zero-divisors found.
    pub zd_count_2blade: usize,
    /// Number of 3-blade zero-divisors found (if searched).
    pub zd_count_3blade: usize,
    /// Associator norm for the current triple (a, b, c).
    /// Non-zero means non-associativity detected.
    pub associator_norm: f64,
    /// Algebra dimension in use.
    pub dimension: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn algebra_dimension_values() {
        assert_eq!(AlgebraDimension::Quaternion.dim(), 4);
        assert_eq!(AlgebraDimension::Octonion.dim(), 8);
        assert_eq!(AlgebraDimension::Sedenion.dim(), 16);
        assert_eq!(AlgebraDimension::Dim32.dim(), 32);
    }

    #[test]
    fn hypercomplex_identity() {
        let id = HypercomplexElement::identity(8);
        assert_eq!(id.coeffs.len(), 8);
        assert!((id.coeffs[0] - 1.0).abs() < 1e-15);
        assert!((id.norm() - 1.0).abs() < 1e-15);
    }

    #[test]
    fn hypercomplex_basis() {
        let e3 = HypercomplexElement::basis(16, 3);
        assert_eq!(e3.coeffs.len(), 16);
        assert!((e3.coeffs[3] - 1.0).abs() < 1e-15);
        assert!((e3.norm() - 1.0).abs() < 1e-15);
    }

    #[test]
    fn default_params_sedenion() {
        let params = AlgebraParams::default();
        assert_eq!(params.dimension, AlgebraDimension::Sedenion);
        assert!(params.zero_tolerance > 0.0);
    }
}

// CdAlgebraEngine resource wrapping cd_kernel and algebra_core.
//
// Manages hypercomplex algebra computations and zero-divisor search
// results as a Bevy resource.

use bevy::prelude::*;

use algebra_core::construction::hypercomplex::{
    HypercomplexAlgebra, ZeroDivisorResults, ZeroSearchConfig,
};
use cd_kernel::cayley_dickson::{cd_associator_norm, cd_multiply, cd_norm_sq};

use crate::components::AlgebraDimension;

/// CPU-based Cayley-Dickson algebra engine.
///
/// Wraps algebra_core's HypercomplexAlgebra for zero-divisor searches
/// and cd_kernel for element multiplication and associator computation.
#[derive(Resource, Default)]
pub struct CdAlgebraEngine {
    /// Active algebra instances keyed by entity ID.
    pub instances: Vec<(Entity, AlgebraInstance)>,
}

/// Per-entity algebra state.
pub struct AlgebraInstance {
    /// The algebra wrapper from algebra_core.
    pub algebra: HypercomplexAlgebra,
    /// Cached zero-divisor search results.
    pub zd_results: Option<ZeroDivisorResults>,
    /// Algebra dimension.
    pub dim: usize,
}

/// Configuration for creating a new algebra instance.
pub struct AlgebraConfig {
    pub dimension: AlgebraDimension,
    pub zero_tolerance: f64,
    pub parallel_search: bool,
    pub max_blade_order: usize,
    pub seed: u64,
}

impl CdAlgebraEngine {
    /// Create a new algebra instance for the given entity.
    pub fn create_instance(&mut self, entity: Entity, config: &AlgebraConfig) {
        self.instances.retain(|(e, _)| *e != entity);

        let dim = config.dimension.dim();
        let algebra = HypercomplexAlgebra::new(dim);
        self.instances.push((
            entity,
            AlgebraInstance {
                algebra,
                zd_results: None,
                dim,
            },
        ));
    }

    /// Get mutable reference to an instance by entity.
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut AlgebraInstance> {
        self.instances
            .iter_mut()
            .find(|(e, _)| *e == entity)
            .map(|(_, inst)| inst)
    }

    /// Get reference to an instance by entity.
    pub fn get(&self, entity: Entity) -> Option<&AlgebraInstance> {
        self.instances
            .iter()
            .find(|(e, _)| *e == entity)
            .map(|(_, inst)| inst)
    }

    /// Remove instance for entity.
    pub fn remove(&mut self, entity: Entity) {
        self.instances.retain(|(e, _)| *e != entity);
    }
}

impl AlgebraInstance {
    /// Run zero-divisor search and cache results.
    pub fn search_zero_divisors(&mut self, config: &ZeroSearchConfig) {
        self.zd_results = Some(self.algebra.find_zero_divisors(config));
    }

    /// Multiply two hypercomplex elements.
    pub fn multiply(&self, a: &[f64], b: &[f64]) -> Vec<f64> {
        assert_eq!(a.len(), self.dim);
        assert_eq!(b.len(), self.dim);
        cd_multiply(a, b)
    }

    /// Compute the associator norm |[a, b, c]| = |(ab)c - a(bc)|.
    pub fn associator_norm(&self, a: &[f64], b: &[f64], c: &[f64]) -> f64 {
        assert_eq!(a.len(), self.dim);
        assert_eq!(b.len(), self.dim);
        assert_eq!(c.len(), self.dim);
        cd_associator_norm(a, b, c)
    }

    /// Compute the norm squared of an element.
    pub fn norm_sq(&self, a: &[f64]) -> f64 {
        cd_norm_sq(a)
    }

    /// Count of 2-blade zero-divisors found.
    pub fn zd_count_2blade(&self) -> usize {
        self.zd_results
            .as_ref()
            .map(|r| r.blade2.len())
            .unwrap_or(0)
    }

    /// Count of 3-blade zero-divisors found (if searched).
    pub fn zd_count_3blade(&self) -> usize {
        self.zd_results
            .as_ref()
            .and_then(|r| r.blade3.as_ref())
            .map(|b| b.len())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(dim: AlgebraDimension) -> AlgebraConfig {
        AlgebraConfig {
            dimension: dim,
            zero_tolerance: 1e-12,
            parallel_search: false,
            max_blade_order: 2,
            seed: 42,
        }
    }

    #[test]
    fn engine_create_and_lookup() {
        let mut engine = CdAlgebraEngine::default();
        let entity = Entity::from_bits(1);
        engine.create_instance(entity, &test_config(AlgebraDimension::Sedenion));

        assert!(engine.get(entity).is_some());
        assert!(engine.get(Entity::from_bits(99)).is_none());

        let inst = engine.get(entity).unwrap();
        assert_eq!(inst.dim, 16);
    }

    #[test]
    fn engine_remove() {
        let mut engine = CdAlgebraEngine::default();
        let entity = Entity::from_bits(1);
        engine.create_instance(entity, &test_config(AlgebraDimension::Octonion));
        engine.remove(entity);
        assert!(engine.get(entity).is_none());
    }

    #[test]
    fn quaternion_multiplication() {
        let mut engine = CdAlgebraEngine::default();
        let entity = Entity::from_bits(1);
        engine.create_instance(entity, &test_config(AlgebraDimension::Quaternion));

        let inst = engine.get(entity).unwrap();
        // i * j = k in quaternion algebra (e1 * e2 = e3)
        let i = vec![0.0, 1.0, 0.0, 0.0];
        let j = vec![0.0, 0.0, 1.0, 0.0];
        let product = inst.multiply(&i, &j);
        // Should be +/- e3 depending on convention
        assert!(product[0].abs() < 1e-12, "real part should be ~0");
        assert!(product[3].abs() > 0.5, "e3 component should be nonzero");
    }

    #[test]
    fn octonion_non_associativity() {
        let mut engine = CdAlgebraEngine::default();
        let entity = Entity::from_bits(1);
        engine.create_instance(entity, &test_config(AlgebraDimension::Octonion));

        let inst = engine.get(entity).unwrap();
        // Octonions are non-associative: pick three basis elements
        let e1 = vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let e2 = vec![0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let e4 = vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0];
        let norm = inst.associator_norm(&e1, &e2, &e4);
        // Octonions have nonzero associator for generic triples
        assert!(norm > 1e-12, "octonion associator should be nonzero");
    }

    #[test]
    fn sedenion_zero_divisor_search() {
        let mut engine = CdAlgebraEngine::default();
        let entity = Entity::from_bits(1);
        engine.create_instance(entity, &test_config(AlgebraDimension::Sedenion));

        let inst = engine.get_mut(entity).unwrap();
        let search_config = ZeroSearchConfig {
            tolerance: 1e-12,
            parallel: false,
            max_blade_order: 2,
            n_samples: 0,
            seed: 42,
        };
        inst.search_zero_divisors(&search_config);
        // Sedenions (dim 16) must have zero-divisors
        assert!(
            inst.zd_count_2blade() > 0,
            "sedenions must have zero-divisors"
        );
    }
}

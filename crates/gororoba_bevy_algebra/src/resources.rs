// CdAlgebraEngine resource wrapping gororoba_app's local algebra kernel.
//
// Manages hypercomplex algebra computations and zero-divisor search
// results as a Bevy resource.

use bevy::prelude::*;
use gororoba_kernel_algebra::CayleyDicksonKernel;
use gororoba_kernel_api::algebra::{AlgebraKernel, ZeroDivisorPair, ZeroDivisorSearchConfig};

use crate::components::AlgebraDimension;

/// CPU-based Cayley-Dickson algebra engine.
#[derive(Resource, Default)]
pub struct CdAlgebraEngine {
    /// Active algebra instances keyed by entity ID.
    pub instances: Vec<(Entity, AlgebraInstance)>,
}

/// Per-entity algebra state.
pub struct AlgebraInstance {
    /// Local Cayley-Dickson kernel used by the game engine.
    pub kernel: CayleyDicksonKernel,
    /// Cached zero-divisor search results.
    pub zd_results: Vec<ZeroDivisorPair>,
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
        self.instances.push((
            entity,
            AlgebraInstance {
                kernel: CayleyDicksonKernel::new(config.dimension),
                zd_results: Vec::new(),
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
    pub fn search_zero_divisors(&mut self, config: &ZeroDivisorSearchConfig) {
        self.zd_results = self.kernel.search_zero_divisors(config);
    }

    /// Multiply two hypercomplex elements.
    pub fn multiply(&self, a: &[f64], b: &[f64]) -> Vec<f64> {
        assert_eq!(a.len(), self.dim);
        assert_eq!(b.len(), self.dim);
        self.kernel.multiply(a, b)
    }

    /// Compute the associator norm |[a, b, c]| = |(ab)c - a(bc)|.
    pub fn associator_norm(&self, a: &[f64], b: &[f64], c: &[f64]) -> f64 {
        assert_eq!(a.len(), self.dim);
        assert_eq!(b.len(), self.dim);
        assert_eq!(c.len(), self.dim);
        self.kernel.associator_norm(a, b, c)
    }

    /// Compute the norm squared of an element.
    pub fn norm_sq(&self, a: &[f64]) -> f64 {
        self.kernel.norm_sq(a)
    }

    /// Count of 2-blade zero-divisors found.
    pub fn zd_count_2blade(&self) -> usize {
        self.zd_results.len()
    }

    /// Count of 3-blade zero-divisors found (if searched).
    pub fn zd_count_3blade(&self) -> usize {
        0
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
        let search_config = ZeroDivisorSearchConfig {
            tolerance: 1e-12,
            max_results: 64,
        };
        inst.search_zero_divisors(&search_config);
        // Sedenions (dim 16) must have zero-divisors
        assert!(
            inst.zd_count_2blade() > 0,
            "sedenions must have zero-divisors"
        );
    }
}

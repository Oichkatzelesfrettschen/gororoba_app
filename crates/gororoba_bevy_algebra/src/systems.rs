// Zero-divisor search, associator computation, and room transform systems.
//
// algebra_init_system initializes algebra instances for new domains.
// zd_search_system runs the zero-divisor search (FixedUpdate).
// associator_system computes associator norms (FixedUpdate).
// diagnostics_system updates diagnostic components (Update).

use bevy::prelude::*;
use gororoba_kernel_api::algebra::ZeroDivisorSearchConfig;

use crate::components::{
    AlgebraDiagnostics, AlgebraDomain, AlgebraParams, HypercomplexElement, ZeroDivisorPortal,
};
use crate::resources::{AlgebraConfig, CdAlgebraEngine};

/// Initialize algebra instances for newly-spawned AlgebraDomain entities.
#[allow(clippy::type_complexity)]
pub fn algebra_init_system(
    mut engine: ResMut<CdAlgebraEngine>,
    query: Query<(Entity, &AlgebraParams), (With<AlgebraDomain>, Added<AlgebraDomain>)>,
) {
    for (entity, params) in &query {
        let config = AlgebraConfig {
            dimension: params.dimension,
            zero_tolerance: params.zero_tolerance,
            parallel_search: params.parallel_search,
            max_blade_order: params.max_blade_order,
            seed: params.seed,
        };
        engine.create_instance(entity, &config);
    }
}

/// Run zero-divisor search on newly-initialized algebra instances.
///
/// This is intentionally separate from init because the search can be
/// expensive for large dimensions. Runs once after initialization.
#[allow(clippy::type_complexity)]
pub fn zd_search_system(
    mut engine: ResMut<CdAlgebraEngine>,
    query: Query<(Entity, &AlgebraParams), (With<AlgebraDomain>, Added<AlgebraDomain>)>,
) {
    for (entity, params) in &query {
        if let Some(inst) = engine.get_mut(entity) {
            let search_config = ZeroDivisorSearchConfig {
                tolerance: params.zero_tolerance,
                max_results: params.max_blade_order.max(1) * 32,
            };
            inst.search_zero_divisors(&search_config);
        }
    }
}

/// Spawn portal entities from zero-divisor search results.
///
/// For each 2-blade zero-divisor pair found, spawns a ZeroDivisorPortal
/// component that the game can use for portal placement.
#[allow(clippy::type_complexity)]
pub fn portal_spawn_system(
    engine: Res<CdAlgebraEngine>,
    query: Query<(Entity, &AlgebraParams), (With<AlgebraDomain>, Added<AlgebraDomain>)>,
    mut commands: Commands,
) {
    for (entity, _params) in &query {
        if let Some(inst) = engine.get(entity) {
            for pair in &inst.zd_results {
                commands.spawn((
                    ZeroDivisorPortal {
                        a_indices: pair.lhs_indices,
                        b_indices: pair.rhs_indices,
                        rhs_sign: pair.rhs_sign,
                        product_norm: pair.product_norm,
                        active: true,
                    },
                    ChildOf(entity),
                ));
            }
        }
    }
}

/// Compute associator norms for entities with hypercomplex elements.
///
/// Takes triples of elements and computes |[a, b, c]| to detect
/// non-associativity. Runs in FixedUpdate for consistency.
pub fn associator_system(
    engine: Res<CdAlgebraEngine>,
    query: Query<(Entity, &AlgebraParams), With<AlgebraDomain>>,
    elements: Query<(&HypercomplexElement, &ChildOf)>,
    mut diagnostics: Query<&mut AlgebraDiagnostics, With<AlgebraDomain>>,
) {
    for (entity, _params) in &query {
        if let Some(inst) = engine.get(entity) {
            // Collect child elements for this domain.
            let domain_elements: Vec<&HypercomplexElement> = elements
                .iter()
                .filter(|(_, parent)| parent.0 == entity)
                .map(|(elem, _)| elem)
                .collect();

            // Compute associator for first triple if available.
            let assoc_norm = if domain_elements.len() >= 3 {
                inst.associator_norm(
                    &domain_elements[0].coeffs,
                    &domain_elements[1].coeffs,
                    &domain_elements[2].coeffs,
                )
            } else {
                0.0
            };

            if let Ok(mut diag) = diagnostics.get_mut(entity) {
                diag.associator_norm = assoc_norm;
            }
        }
    }
}

/// Update diagnostic components from algebra engine state.
pub fn diagnostics_system(
    engine: Res<CdAlgebraEngine>,
    mut query: Query<(Entity, &mut AlgebraDiagnostics), With<AlgebraDomain>>,
) {
    for (entity, mut diag) in &mut query {
        if let Some(inst) = engine.get(entity) {
            diag.dimension = inst.dim;
            diag.zd_count_2blade = inst.zd_count_2blade();
            diag.zd_count_3blade = inst.zd_count_3blade();
        }
    }
}

/// Clean up algebra instances when AlgebraDomain entities are despawned.
pub fn algebra_cleanup_system(
    mut engine: ResMut<CdAlgebraEngine>,
    mut removals: RemovedComponents<AlgebraDomain>,
) {
    for entity in removals.read() {
        engine.remove(entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::AlgebraDimension;
    use crate::resources::AlgebraConfig;

    fn test_config() -> AlgebraConfig {
        AlgebraConfig {
            dimension: AlgebraDimension::Sedenion,
            zero_tolerance: 1e-12,
            parallel_search: false,
            max_blade_order: 2,
            seed: 42,
        }
    }

    #[test]
    fn zd_search_finds_sedenion_divisors() {
        let mut engine = CdAlgebraEngine::default();
        let entity = Entity::from_bits(1);
        engine.create_instance(entity, &test_config());

        let inst = engine.get_mut(entity).unwrap();
        let search_config = ZeroDivisorSearchConfig {
            tolerance: 1e-12,
            max_results: 64,
        };
        inst.search_zero_divisors(&search_config);

        assert!(inst.zd_count_2blade() > 0);
    }

    #[test]
    fn associator_norm_quaternion_zero() {
        let mut engine = CdAlgebraEngine::default();
        let entity = Entity::from_bits(1);
        let config = AlgebraConfig {
            dimension: AlgebraDimension::Quaternion,
            zero_tolerance: 1e-12,
            parallel_search: false,
            max_blade_order: 2,
            seed: 42,
        };
        engine.create_instance(entity, &config);

        let inst = engine.get(entity).unwrap();
        // Quaternions are associative: [e1, e2, e3] should be 0
        let e1 = vec![0.0, 1.0, 0.0, 0.0];
        let e2 = vec![0.0, 0.0, 1.0, 0.0];
        let e3 = vec![0.0, 0.0, 0.0, 1.0];
        let norm = inst.associator_norm(&e1, &e2, &e3);
        assert!(
            norm < 1e-12,
            "quaternion associator should be zero, got {norm}"
        );
    }
}

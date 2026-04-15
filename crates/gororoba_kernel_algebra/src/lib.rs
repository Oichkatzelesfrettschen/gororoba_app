use gororoba_kernel_api::algebra::{
    AlgebraDimension, AlgebraKernel, AlgebraSnapshot, KernelBackend, ZeroDivisorPair,
    ZeroDivisorSearchConfig,
};
use gororoba_kernel_api::projection::{ProjectedPoint3, ProjectionSpec};
use rayon::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct CayleyDicksonKernel {
    dimension: AlgebraDimension,
}

impl CayleyDicksonKernel {
    pub fn new(dimension: AlgebraDimension) -> Self {
        Self { dimension }
    }

    pub fn snapshot(
        &self,
        associator_norm: f64,
        zero_divisors: Vec<ZeroDivisorPair>,
    ) -> AlgebraSnapshot {
        AlgebraSnapshot {
            dimension: self.dimension,
            backend: KernelBackend::Cpu,
            associator_norm,
            zero_divisors,
        }
    }
}

impl AlgebraKernel for CayleyDicksonKernel {
    fn dimension(&self) -> AlgebraDimension {
        self.dimension
    }

    fn multiply(&self, lhs: &[f64], rhs: &[f64]) -> Vec<f64> {
        assert_eq!(lhs.len(), self.dimension.dim());
        assert_eq!(rhs.len(), self.dimension.dim());
        cd_multiply(lhs, rhs)
    }

    fn associator_norm(&self, a: &[f64], b: &[f64], c: &[f64]) -> f64 {
        let ab_c = self.multiply(&self.multiply(a, b), c);
        let a_bc = self.multiply(a, &self.multiply(b, c));
        ab_c.iter()
            .zip(a_bc.iter())
            .map(|(lhs, rhs)| (lhs - rhs) * (lhs - rhs))
            .sum::<f64>()
            .sqrt()
    }

    fn norm_sq(&self, value: &[f64]) -> f64 {
        value.iter().map(|entry| entry * entry).sum()
    }

    fn search_zero_divisors(&self, config: &ZeroDivisorSearchConfig) -> Vec<ZeroDivisorPair> {
        let dim = self.dimension.dim();
        if dim < 16 {
            return Vec::new();
        }

        let lhs_pairs: Vec<(usize, usize)> = (1..dim)
            .flat_map(|i| ((i + 1)..dim).map(move |j| (i, j)))
            .collect();

        let mut pairs: Vec<ZeroDivisorPair> = lhs_pairs
            .par_iter()
            .map(|&(i, j)| {
                let lhs = diagonal_two_blade(dim, i, j, 1);
                let mut local_pairs = Vec::new();
                for k in 1..dim {
                    for l in (k + 1)..dim {
                        for rhs_sign in [1_i8, -1_i8] {
                            let rhs = diagonal_two_blade(dim, k, l, rhs_sign);
                            let product = self.multiply(&lhs, &rhs);
                            let product_norm = self.norm_sq(&product).sqrt();
                            if product_norm <= config.tolerance {
                                local_pairs.push(ZeroDivisorPair {
                                    lhs_indices: (i, j),
                                    rhs_indices: (k, l),
                                    rhs_sign,
                                    product_norm,
                                });
                            }
                        }
                    }
                }
                local_pairs
            })
            .reduce(Vec::new, |mut acc, mut local_pairs| {
                acc.append(&mut local_pairs);
                acc
            });

        pairs.sort_by(|lhs, rhs| {
            lhs.lhs_indices
                .cmp(&rhs.lhs_indices)
                .then(lhs.rhs_indices.cmp(&rhs.rhs_indices))
                .then(lhs.rhs_sign.cmp(&rhs.rhs_sign))
                .then_with(|| lhs.product_norm.total_cmp(&rhs.product_norm))
        });
        pairs.truncate(config.max_results);
        pairs
    }
}

pub fn basis_element(dim: usize, index: usize) -> Vec<f64> {
    let mut coeffs = vec![0.0; dim];
    coeffs[index] = 1.0;
    coeffs
}

pub fn diagonal_two_blade(dim: usize, lhs: usize, rhs: usize, rhs_sign: i8) -> Vec<f64> {
    let mut coeffs = vec![0.0; dim];
    coeffs[lhs] = 1.0;
    coeffs[rhs] = f64::from(rhs_sign);
    coeffs
}

pub fn zero_divisor_signature(dim: usize, pair: &ZeroDivisorPair) -> Vec<f64> {
    let mut coeffs = vec![0.0; dim];
    coeffs[pair.lhs_indices.0] += 1.0;
    coeffs[pair.lhs_indices.1] += 1.0;
    coeffs[pair.rhs_indices.0] -= 1.0;
    coeffs[pair.rhs_indices.1] -= f64::from(pair.rhs_sign);
    coeffs
}

pub fn project_coeffs(coeffs: &[f64], spec: &ProjectionSpec) -> ProjectedPoint3 {
    let pick = |axis: usize| coeffs.get(axis).copied().unwrap_or(0.0) as f32 * spec.scale;
    ProjectedPoint3 {
        x: pick(spec.axes[0]),
        y: pick(spec.axes[1]),
        z: pick(spec.axes[2]),
    }
}

fn cd_multiply(lhs: &[f64], rhs: &[f64]) -> Vec<f64> {
    assert_eq!(lhs.len(), rhs.len());
    assert!(lhs.len().is_power_of_two());

    if lhs.len() == 1 {
        return vec![lhs[0] * rhs[0]];
    }

    let half = lhs.len() / 2;
    let (a, b) = lhs.split_at(half);
    let (c, d) = rhs.split_at(half);

    let ac = cd_multiply(a, c);
    let d_conj_b = cd_multiply(&conjugate(d), b);
    let da = cd_multiply(d, a);
    let b_conj_c = cd_multiply(b, &conjugate(c));

    let mut result = vec![0.0; lhs.len()];
    for idx in 0..half {
        result[idx] = ac[idx] - d_conj_b[idx];
        result[half + idx] = da[idx] + b_conj_c[idx];
    }
    result
}

fn conjugate(value: &[f64]) -> Vec<f64> {
    if value.len() == 1 {
        return vec![value[0]];
    }

    let half = value.len() / 2;
    let (real, imag) = value.split_at(half);
    let mut result = Vec::with_capacity(value.len());
    result.extend(conjugate(real));
    result.extend(imag.iter().map(|entry| -entry));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quaternion_basis_multiplication_matches_expected_orientation() {
        let kernel = CayleyDicksonKernel::new(AlgebraDimension::Quaternion);
        let i = basis_element(4, 1);
        let j = basis_element(4, 2);
        let product = kernel.multiply(&i, &j);
        assert!(product[0].abs() < 1e-12);
        assert!(product[3] > 0.5);
    }

    #[test]
    fn octonion_associator_is_nonzero() {
        let kernel = CayleyDicksonKernel::new(AlgebraDimension::Octonion);
        let e1 = basis_element(8, 1);
        let e2 = basis_element(8, 2);
        let e4 = basis_element(8, 4);
        assert!(kernel.associator_norm(&e1, &e2, &e4) > 1e-12);
    }

    #[test]
    fn sedenion_zero_divisor_search_finds_known_pairs() {
        let kernel = CayleyDicksonKernel::new(AlgebraDimension::Sedenion);
        let pairs = kernel.search_zero_divisors(&ZeroDivisorSearchConfig::default());
        assert!(!pairs.is_empty());

        let signature = zero_divisor_signature(16, &pairs[0]);
        let projected = project_coeffs(&signature, &ProjectionSpec::default()).scaled(2.0);
        assert!(projected.x.is_finite());
        assert!(projected.y.is_finite());
        assert!(projected.z.is_finite());
    }
}

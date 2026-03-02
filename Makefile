.PHONY: fmt clippy test check run run-physics run-synthesis \
       run-fluid run-noneuclidean run-relativistic run-quantum check-games \
       package-desktop mobile-contract verify-deps

fmt:
	cargo fmt --all --check

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace --all-targets

check: fmt clippy test

run:
	cargo run -p gororoba_studio_web

run-physics:
	cargo run -p physics_sandbox

run-synthesis:
	cargo run -p synthesis_arena

run-fluid:
	cargo run -p fluid_dynamics

run-noneuclidean:
	cargo run -p non_euclidean

run-relativistic:
	cargo run -p relativistic_space

run-quantum:
	cargo run -p quantum_builder

check-games:
	cargo clippy -p fluid_dynamics -p non_euclidean -p relativistic_space -p quantum_builder -- -D warnings
	cargo test -p fluid_dynamics -p non_euclidean -p relativistic_space -p quantum_builder

package-desktop:
	scripts/package_desktop_matrix.sh

mobile-contract:
	scripts/generate_mobile_contract.sh

verify-deps:
	cargo run -p xtask -- verify-deps

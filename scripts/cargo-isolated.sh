#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

export CARGO_HOME="${CARGO_HOME:-${repo_root}/.cargo-home}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${repo_root}/target-gororoba_app}"

detected_cpus="$(nproc 2>/dev/null || getconf _NPROCESSORS_ONLN || echo 1)"
default_jobs="$((detected_cpus / 2))"
if [[ "${default_jobs}" -lt 1 ]]; then
  default_jobs=1
fi

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-${GOROROBA_CARGO_JOBS:-${default_jobs}}}"
export RUST_TEST_THREADS="${RUST_TEST_THREADS:-${CARGO_BUILD_JOBS}}"
export RAYON_NUM_THREADS="${RAYON_NUM_THREADS:-${CARGO_BUILD_JOBS}}"

if [[ "${GOROROBA_USE_SCCACHE:-0}" == "1" ]] && [[ -z "${RUSTC_WRAPPER:-}" ]] && command -v sccache >/dev/null 2>&1; then
  export RUSTC_WRAPPER="sccache"
fi

if [[ "${RUSTC_WRAPPER:-}" == *sccache* ]]; then
  unset CARGO_INCREMENTAL
fi

if [[ "$(uname -s)" == "Linux" && "$(uname -m)" == "x86_64" ]]; then
  if [[ -z "${CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER:-}" ]] && command -v clang >/dev/null 2>&1; then
    export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="clang"
  fi

  if command -v mold >/dev/null 2>&1; then
    mold_flag="-C link-arg=-fuse-ld=mold"
    current_flags="${CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS:-}"
    if [[ " ${current_flags} " != *" ${mold_flag} "* ]]; then
      if [[ -n "${current_flags}" ]]; then
        export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS="${current_flags} ${mold_flag}"
      else
        export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS="${mold_flag}"
      fi
    fi
  fi
fi

mkdir -p "${CARGO_HOME}" "${CARGO_TARGET_DIR}"

exec cargo "$@"

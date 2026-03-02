#!/usr/bin/env bash
set -euo pipefail

DEFAULT_TARGETS="x86_64-unknown-linux-gnu,x86_64-unknown-freebsd,x86_64-pc-windows-msvc,x86_64-apple-darwin,aarch64-apple-darwin"
DEFAULT_BINS="gororoba_studio_web,physics_sandbox,synthesis_arena"

PROFILE="${GOROROBA_PACKAGE_PROFILE:-release}"
OUT_DIR="${GOROROBA_PACKAGE_OUT_DIR:-dist/desktop}"
BINS_CSV="${GOROROBA_PACKAGE_BINS:-$DEFAULT_BINS}"
REQUESTED_TARGETS="${GOROROBA_PACKAGE_TARGETS:-$DEFAULT_TARGETS}"
ENABLE_CROSS="${GOROROBA_ENABLE_CROSS:-0}"
CROSS_READY="${GOROROBA_CROSS_TOOLCHAINS_READY:-0}"
ALLOW_MISSING="${GOROROBA_PACKAGE_ALLOW_MISSING:-1}"

HOST_TARGET="$(rustc -vV | awk '/^host: / { print $2 }')"
if [[ -z "${HOST_TARGET}" ]]; then
  echo "error: unable to determine host target from rustc -vV" >&2
  exit 1
fi

if [[ "${ENABLE_CROSS}" != "1" ]]; then
  REQUESTED_TARGETS="${HOST_TARGET}"
  echo "info: cross packaging is disabled; packaging host target only (${HOST_TARGET})."
fi

INSTALLED_TARGETS="$(rustup target list --installed || true)"
if [[ -z "${INSTALLED_TARGETS}" ]]; then
  echo "error: rustup returned no installed targets; install at least ${HOST_TARGET}." >&2
  exit 1
fi

declare -a SELECTED_TARGETS=()
declare -a SKIPPED_TARGETS=()

IFS=',' read -r -a TARGET_ARRAY <<< "${REQUESTED_TARGETS}"
for RAW_TARGET in "${TARGET_ARRAY[@]}"; do
  TARGET="$(echo "${RAW_TARGET}" | xargs)"
  if [[ -z "${TARGET}" ]]; then
    continue
  fi

  if ! grep -qx "${TARGET}" <<< "${INSTALLED_TARGETS}"; then
    SKIPPED_TARGETS+=("${TARGET}: missing rust target (run: rustup target add ${TARGET})")
    continue
  fi

  if [[ "${TARGET}" != "${HOST_TARGET}" && "${ENABLE_CROSS}" == "1" && "${CROSS_READY}" != "1" ]]; then
    SKIPPED_TARGETS+=("${TARGET}: cross toolchains not confirmed (set GOROROBA_CROSS_TOOLCHAINS_READY=1 after configuring linkers)")
    continue
  fi

  SELECTED_TARGETS+=("${TARGET}")
done

if [[ ${#SELECTED_TARGETS[@]} -eq 0 ]]; then
  echo "error: no targets selected after preflight checks." >&2
  if [[ ${#SKIPPED_TARGETS[@]} -gt 0 ]]; then
    printf 'error: %s\n' "${SKIPPED_TARGETS[@]}" >&2
  fi
  exit 1
fi

TARGETS_CSV="$(IFS=','; echo "${SELECTED_TARGETS[*]}")"

echo "info: desktop packaging targets: ${TARGETS_CSV}"
if [[ ${#SKIPPED_TARGETS[@]} -gt 0 ]]; then
  echo "info: skipped targets:"
  printf '  - %s\n' "${SKIPPED_TARGETS[@]}"
fi

XTASK_ARGS=(
  -p xtask -- desktop-package
  --targets "${TARGETS_CSV}"
  --bins "${BINS_CSV}"
  --profile "${PROFILE}"
  --out-dir "${OUT_DIR}"
)

if [[ "${ALLOW_MISSING}" == "1" ]]; then
  XTASK_ARGS+=(--allow-missing)
fi

cargo run "${XTASK_ARGS[@]}"

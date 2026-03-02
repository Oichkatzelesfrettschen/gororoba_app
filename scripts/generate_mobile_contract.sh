#!/usr/bin/env bash
set -euo pipefail

cargo run -p xtask -- mobile-contract \
  --output apps/mobile_spike/contracts/shared_core_contract.json

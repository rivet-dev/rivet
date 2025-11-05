#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

cd "${REPO_ROOT}"

RUST_LOG="${RUST_LOG:-debug}" \
RIVET__PEGBOARD__RETRY_RESET_DURATION="100" \
RIVET__PEGBOARD__BASE_RETRY_TIMEOUT="100" \
RIVET__PEGBOARD__RESCHEDULE_BACKOFF_MAX_EXPONENT="1" \
RIVET__PEGBOARD__RUNNER_ELIGIBLE_THRESHOLD="5000" \
RIVET__PEGBOARD__RUNNER_LOST_THRESHOLD="7000" \
cargo run --bin rivet-engine -- start "$@" | tee /tmp/rivet-engine.log


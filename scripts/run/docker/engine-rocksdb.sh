#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

cd "${REPO_ROOT}"

RUST_BACKTRACE=full \
RUST_LOG="${RUST_LOG:-"opentelemetry_sdk=off,opentelemetry-otlp=info,tower::buffer::worker=info,debug"}" \
RUST_LOG_TARGET=1 \
RIVET__PEGBOARD__RETRY_RESET_DURATION="100" \
RIVET__PEGBOARD__BASE_RETRY_TIMEOUT="100" \
RIVET__PEGBOARD__RESCHEDULE_BACKOFF_MAX_EXPONENT="1" \
RIVET__PEGBOARD__RUNNER_ELIGIBLE_THRESHOLD="5000" \
RIVET__PEGBOARD__RUNNER_LOST_THRESHOLD="7000" \
cargo run --bin rivet-engine -- start "$@" 2>&1 | tee -i /tmp/rivet-engine.log


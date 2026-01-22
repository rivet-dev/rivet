#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"

cd "${REPO_ROOT}"

FILTERED_ARGS=()

for arg in "$@"; do
	FILTERED_ARGS+=("$arg")
done

RUST_BACKTRACE=full \
RUST_LOG="${RUST_LOG:-"opentelemetry_sdk=off,opentelemetry-otlp=info,tower::buffer::worker=info,debug"}" \
RUST_LOG_TARGET=1 \
cargo run --bin rivet-engine ${FILTERED_ARGS[@]+"${FILTERED_ARGS[@]}"} -- start 2>&1 | tee -i /tmp/rivet-engine.log

#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

cd "${REPO_ROOT}"

RUST_BACKTRACE=full \
RIVET__AUTH__ADMIN_TOKEN="${RIVET__AUTH__ADMIN_TOKEN:-default}" \
RUST_LOG="${RUST_LOG:-"opentelemetry_sdk=off,opentelemetry-otlp=info,tower::buffer::worker=info,debug"}" \
RUST_LOG_TARGET=1 \
cargo run -p rivet-engine --bin rivet-engine -- start 2>&1 | tee -i /tmp/rivet-engine.log

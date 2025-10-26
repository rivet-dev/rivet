#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

cd "${REPO_ROOT}"

RUST_LOG="${RUST_LOG:-debug}" \
cargo run --bin rivet-engine -- start "$@" | tee /tmp/rivet-engine.log


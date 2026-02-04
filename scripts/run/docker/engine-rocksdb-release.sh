#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"

cd "${REPO_ROOT}"

RUST_LOG=warn \
cargo run --release --bin rivet-engine -- start 2>&1 | tee -i /tmp/rivet-engine.log

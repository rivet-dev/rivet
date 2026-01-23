#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"

cd "${REPO_ROOT}"

export RIVET__POSTGRES__URL=postgres://postgres:postgres@localhost:5432/postgres
export RUST_LOG=debug
export PATH="${REPO_ROOT}/target/debug:${REPO_ROOT}/target/release:${PATH}"

echo "Opening subshell with Rivet engine environment variables..."
echo "RIVET__POSTGRES__URL=${RIVET__POSTGRES__URL}"
echo "RUST_LOG=${RUST_LOG}"
echo "PATH=${PATH}"
echo ""
echo "Type 'exit' to leave the subshell."
echo ""

# Open subshell (use user's preferred shell or default to bash)
exec bash

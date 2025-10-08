#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

if ! command -v nc >/dev/null 2>&1; then
  echo "error: required command 'nc' not found."
  exit 1
fi

if ! nc -z localhost 5432 >/dev/null 2>&1; then
  echo "Postgres is not reachable at localhost:5432."
  echo "Hint: run scripts/dev/run-postgres.sh to start the local Postgres container."
  exit 1
fi

cd "${REPO_ROOT}"

RIVET__POSTGRES__URL=postgres://postgres:postgres@localhost:5432/postgres \
RUST_LOG=debug \
cargo run --bin rivet-engine -- start "$@"

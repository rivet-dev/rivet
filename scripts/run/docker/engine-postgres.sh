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
  echo "Starting postgres container..."
  "${SCRIPT_DIR}/postgres.sh"

  echo "Waiting for postgres to be ready..."
  for i in {1..30}; do
    if nc -z localhost 5432 >/dev/null 2>&1; then
      echo "Postgres is ready!"
      break
    fi
    if [ $i -eq 30 ]; then
      echo "error: postgres did not become ready in time"
      exit 1
    fi
    sleep 1
  done
fi

cd "${REPO_ROOT}"

RUST_BACKTRACE=full \
RIVET__POSTGRES__URL=postgres://postgres:postgres@localhost:5432/postgres \
RUST_LOG=debug \
RUST_LOG_TARGET=1 \
cargo run --bin rivet-engine -- start "$@" 2>&1 | tee /tmp/rivet-engine.log

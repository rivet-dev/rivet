#!/usr/bin/env bash
set -euf -o pipefail

cd lib/bolt

# Keep rust flags the same as when building services
export RUSTFLAGS="--cfg tokio_unstable"

cargo fmt
cargo build --bin bolt

#!/usr/bin/env bash
set -euf -o pipefail

cd lib/bolt

cargo fmt
cargo build --bin bolt

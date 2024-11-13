#!/bin/bash

# This will log errors for tests that have not been fixed. The --keep-going flag
# will ignore them.

cargo clippy --fix --keep-going -- -W warnings
cargo fmt


#!/usr/bin/env bash
# Ensure the user-scoped agent working directory exists.
# Agent working files (notes, specs, research, todo, benchmarks, scratch) live
# here instead of inside the repo. Override the location with AGENTS_DIR.
set -euo pipefail

AGENTS_DIR="${AGENTS_DIR:-$HOME/.agents}"
mkdir -p "$AGENTS_DIR"/{notes,specs,research,todo,benchmarks,scratch}
echo "agents dir ready at $AGENTS_DIR"

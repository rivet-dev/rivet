#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"

cd "${REPO_ROOT}"

deleted=false

# macOS path
if [[ -d "$HOME/Library/Application Support/rivet-engine/" ]]; then
    rm -rf "$HOME/Library/Application Support/rivet-engine/"
    echo "Deleted: $HOME/Library/Application Support/rivet-engine/"
    deleted=true
fi

# Linux path
if [[ -d "$HOME/.local/share/rivet-engine" ]]; then
    rm -rf "$HOME/.local/share/rivet-engine"
    echo "Deleted: $HOME/.local/share/rivet-engine"
    deleted=true
fi

if [[ "$deleted" == "false" ]]; then
    echo "Error: No RocksDB data directories found" >&2
    exit 1
fi

echo "RocksDB data successfully nuked"

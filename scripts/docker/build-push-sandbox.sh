#!/usr/bin/env bash
set -euo pipefail
exec "$(dirname "$0")/build-sandbox.sh" --push "$@"

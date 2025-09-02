#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

: "${VERSION:?Set VERSION env var, e.g. VERSION=1.2.3}"

# Release to rivetkit/engine with :$VERSION and :latest
IMAGE_REPO=${IMAGE_REPO:-rivetkit/engine}

"${SCRIPT_DIR}/build-push.sh" "${IMAGE_REPO}" "${VERSION}" latest


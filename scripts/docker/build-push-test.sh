#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

DATE=$(date +%Y%m%d-%H%M%S)
IMAGE_REPO=${IMAGE_REPO:-ghcr.io/rivet-gg/engine}
IMAGE_TAG=${IMAGE_TAG:-local-${DATE}}

"${SCRIPT_DIR}/build-push.sh" "${IMAGE_REPO}" "${IMAGE_TAG}"

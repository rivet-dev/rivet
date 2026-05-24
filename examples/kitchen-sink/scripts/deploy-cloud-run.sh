#!/usr/bin/env bash
# Manually deploy the kitchen-sink (built from the local workspace) to the
# `rivet-kitchen-sink` Cloud Run service in dev-projects-491221 / us-east4.
#
# By default this only deploys to `rivet-kitchen-sink` (prod). Pass
# `--also-staging` to additionally deploy to `kitchen-sink-staging`.
#
# This is the "current workspace" path. For deploying a *published* rivetkit
# preview build instead (temp-copy + pinned versions), see the second flow in
# examples/kitchen-sink/CLAUDE.md.
#
# Prereqs:
#   - docker
#   - gcloud (authenticated to nathan@rivet.gg or any account with
#     run.developer on dev-projects-491221 and artifactregistry.writer on the
#     us-east4 cloud-run-source-deploy repo)
#   - jj (for tag derivation; falls back to `git rev-parse --short HEAD` if
#     jj is not installed)
#   - rivetkit-typescript/packages/rivetkit-napi/rivetkit-napi.linux-x64-gnu.node
#     already built. Build it with:
#       cd rivetkit-typescript/packages/rivetkit-napi && pnpm build:release
#
# Usage:
#   examples/kitchen-sink/scripts/deploy-cloud-run.sh [--also-staging]

set -euo pipefail

REPO_ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$REPO_ROOT"

ALSO_STAGING=0
if [[ "${1:-}" == "--also-staging" ]]; then
    ALSO_STAGING=1
fi

PROJECT=dev-projects-491221
REGION=us-east4
REGISTRY="${REGION}-docker.pkg.dev/${PROJECT}/cloud-run-source-deploy/rivet-dev-rivet/rivet-kitchen-sink"

NAPI_SRC="rivetkit-typescript/packages/rivetkit-napi/rivetkit-napi.linux-x64-gnu.node"
NAPI_DST="examples/kitchen-sink/rivetkit-napi.linux-x64-gnu.node"

if [[ ! -f "$NAPI_SRC" ]]; then
    echo "error: $NAPI_SRC is missing." >&2
    echo "Build it first: cd rivetkit-typescript/packages/rivetkit-napi && pnpm build:release" >&2
    exit 1
fi

if command -v jj >/dev/null 2>&1; then
    SHA=$(jj log -r @ --no-graph -T 'commit_id.short()')
else
    SHA=$(git rev-parse --short HEAD)
fi
TAG="manual-${SHA}"
IMG="${REGISTRY}:${TAG}"

echo "[deploy] staging napi binary"
cp "$NAPI_SRC" "$NAPI_DST"

echo "[deploy] building $IMG"
docker build --progress=plain -t "$IMG" -f examples/kitchen-sink/Dockerfile .

echo "[deploy] configuring docker auth for ${REGION}-docker.pkg.dev"
gcloud auth configure-docker "${REGION}-docker.pkg.dev" --quiet >/dev/null

echo "[deploy] pushing $IMG"
docker push "$IMG"

deploy_one() {
    local svc="$1"
    echo "[deploy] updating Cloud Run service $svc -> $IMG"
    gcloud run services update "$svc" \
        --image="$IMG" \
        --project="$PROJECT" \
        --region="$REGION" \
        --quiet
    local url
    url=$(gcloud run services describe "$svc" \
        --project="$PROJECT" --region="$REGION" \
        --format='value(status.url)')
    echo "[deploy] verifying $svc /api/rivet/health"
    if ! curl --max-time 20 -fsS "$url/api/rivet/health"; then
        echo
        echo "[deploy] WARNING: health check failed for $svc" >&2
        return 1
    fi
    echo
}

deploy_one rivet-kitchen-sink
if (( ALSO_STAGING )); then
    deploy_one kitchen-sink-staging
fi

echo "[deploy] done."

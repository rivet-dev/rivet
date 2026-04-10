#!/bin/bash
set -e

# Build and push builder base images to ghcr.io.
# Usage: ./build-push.sh <base-name|all> [--push]
#
# Examples:
#   ./build-push.sh osxcross --push     # Build and push osxcross base
#   ./build-push.sh linux-musl          # Build only (no push)
#   ./build-push.sh all --push          # Build and push all bases (parallel)
#
# Available bases: osxcross, linux-musl, linux-gnu, windows-mingw
#
# Images are tagged with the git commit SHA that built them:
#   ghcr.io/rivet-dev/rivet/builder-base-osxcross:<sha>
#
# After pushing, update BASE_TAG in .github/workflows/preview-publish.yaml
# and .github/workflows/release.yaml to reference the new tag.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DOCKERFILE_DIR="$REPO_ROOT/docker/builder-base"
REGISTRY="ghcr.io/rivet-dev/rivet"
TAG="$(git -C "$REPO_ROOT" rev-parse --short HEAD)"

BASES="osxcross linux-musl linux-gnu windows-mingw"

build_one() {
    local name="$1"
    local dockerfile="$DOCKERFILE_DIR/${name}.Dockerfile"
    local image="$REGISTRY/builder-base-${name}:${TAG}"

    if [ ! -f "$dockerfile" ]; then
        echo "ERROR: Dockerfile not found: $dockerfile"
        return 1
    fi

    echo "==> Building $image"
    DOCKER_BUILDKIT=1 docker build -f "$dockerfile" -t "$image" "$REPO_ROOT"
    echo "==> Built: $image"
}

push_one() {
    local name="$1"
    local image="$REGISTRY/builder-base-${name}:${TAG}"
    echo "==> Pushing $image"
    docker push "$image"
    echo "==> Pushed: $image"
}

# Parse args
BASE_NAME="${1:-}"
PUSH="false"
for arg in "$@"; do
    if [ "$arg" = "--push" ]; then
        PUSH="true"
    fi
done

if [ -z "$BASE_NAME" ]; then
    echo "Usage: $0 <base-name|all> [--push]"
    echo "Available bases: $BASES"
    exit 1
fi

if [ "$PUSH" = "true" ]; then
    echo "Ensure you are logged in: docker login ghcr.io -u <github-username>"
    echo ""
fi

if [ "$BASE_NAME" = "all" ]; then
    # Build all in parallel
    echo "==> Building all base images in parallel (tag: $TAG)"
    PIDS=()
    NAMES=()
    for base in $BASES; do
        build_one "$base" &
        PIDS+=($!)
        NAMES+=("$base")
    done

    # Wait for all builds
    FAILED=()
    for i in "${!PIDS[@]}"; do
        if ! wait "${PIDS[$i]}"; then
            FAILED+=("${NAMES[$i]}")
        fi
    done

    if [ ${#FAILED[@]} -gt 0 ]; then
        echo ""
        echo "ERROR: Failed to build: ${FAILED[*]}"
        exit 1
    fi

    echo ""
    echo "==> All images built successfully"

    # Push all after all builds succeed
    if [ "$PUSH" = "true" ]; then
        echo "==> Pushing all images"
        for base in $BASES; do
            push_one "$base"
        done
    fi
else
    build_one "$BASE_NAME"
    if [ "$PUSH" = "true" ]; then
        push_one "$BASE_NAME"
    fi
fi

echo ""
echo "Done. Tag: $TAG"
echo ""
echo "Update FROM lines in Dockerfiles to use tag: $TAG"

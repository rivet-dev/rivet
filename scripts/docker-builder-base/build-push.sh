#!/bin/bash
set -e

# Build and push a builder base image to ghcr.io.
# Usage: ./build-push.sh <base-name> [--push]
#
# Examples:
#   ./build-push.sh osxcross --push     # Build and push osxcross base
#   ./build-push.sh linux-musl          # Build only (no push)
#   ./build-push.sh all --push          # Build and push all bases
#
# Available bases: osxcross, linux-musl, linux-gnu, windows-mingw, windows-msvc
#
# Images are tagged with the git commit SHA that built them:
#   ghcr.io/rivet-dev/rivet/builder-base-osxcross:<sha>
#
# After pushing, update the FROM lines in the consuming Dockerfiles
# (engine/docker/engine/*.Dockerfile, rivetkit-typescript/packages/rivetkit-native/docker/*.Dockerfile)
# to reference the new tag.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DOCKERFILE_DIR="$REPO_ROOT/engine/docker/builder-base"
REGISTRY="ghcr.io/rivet-dev/rivet"
TAG="$(git -C "$REPO_ROOT" rev-parse --short HEAD)"

BASES="osxcross linux-musl linux-gnu windows-mingw windows-msvc"

build_base() {
    local name="$1"
    local dockerfile="$DOCKERFILE_DIR/${name}.Dockerfile"
    local image="$REGISTRY/builder-base-${name}:${TAG}"

    if [ ! -f "$dockerfile" ]; then
        echo "ERROR: Dockerfile not found: $dockerfile"
        exit 1
    fi

    echo "==> Building $image"
    DOCKER_BUILDKIT=1 docker build -f "$dockerfile" -t "$image" "$REPO_ROOT"
    echo "==> Built: $image"

    if [ "$PUSH" = "true" ]; then
        echo "==> Pushing $image"
        docker push "$image"
        echo "==> Pushed: $image"
    fi

    echo ""
    echo "Update Dockerfiles with:"
    echo "  FROM $image"
    echo ""
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
    echo "Ensuring ghcr.io login..."
    echo "If not logged in, run: docker login ghcr.io -u <github-username>"
    echo ""
fi

if [ "$BASE_NAME" = "all" ]; then
    for base in $BASES; do
        build_base "$base"
    done
else
    build_base "$BASE_NAME"
fi

echo "Done. Tag: $TAG"

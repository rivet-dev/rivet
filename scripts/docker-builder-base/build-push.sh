#!/bin/bash
set -e

# Build and push builder base images to ghcr.io.
# Usage: ./build-push.sh <base-name|all> [--push]
#
# Examples:
#   ./build-push.sh osxcross --push     # Build and push osxcross base
#   ./build-push.sh linux-musl          # Build only (no push)
#   ./build-push.sh all --push          # Build and push all bases (parallel)
#   TAG_OVERRIDE=engine-base-v1 ./build-push.sh engine-builder --push
#
# Available bases: osxcross, linux-musl, linux-gnu, windows-mingw,
#                  engine-builder, engine-runtime-full, engine-runtime-slim
#
# Images default to the current git commit SHA unless TAG_OVERRIDE is set:
#   ghcr.io/rivet-dev/rivet/builder-base-osxcross:<sha>
#   ghcr.io/rivet-dev/rivet/engine-base-builder:<sha>
#
# After pushing all bases, this script updates the pinned GHCR tags in the
# consumer Dockerfiles.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DOCKERFILE_DIR="$REPO_ROOT/docker/builder-base"
REGISTRY="ghcr.io/rivet-dev/rivet"
TAG="${TAG_OVERRIDE:-$(git -C "$REPO_ROOT" rev-parse --short HEAD)}"

BASES="osxcross linux-musl linux-gnu windows-mingw engine-builder engine-runtime-full engine-runtime-slim"
MULTIARCH_BUILDER="rivet-multiarch"

image_for_base() {
    local name="$1"
    case "$name" in
        engine-builder)
            echo "$REGISTRY/engine-base-builder:$TAG"
            ;;
        engine-runtime-full)
            echo "$REGISTRY/engine-base-runtime-full:$TAG"
            ;;
        engine-runtime-slim)
            echo "$REGISTRY/engine-base-runtime-slim:$TAG"
            ;;
        *)
            echo "$REGISTRY/builder-base-$name:$TAG"
            ;;
    esac
}

is_multiarch_base() {
    local name="$1"
    case "$name" in
        engine-builder|engine-runtime-full|engine-runtime-slim)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

ensure_multiarch_builder() {
    if docker buildx inspect "$MULTIARCH_BUILDER" >/dev/null 2>&1; then
        docker buildx use "$MULTIARCH_BUILDER" >/dev/null
    else
        docker buildx create --name "$MULTIARCH_BUILDER" --driver docker-container --use >/dev/null
    fi
    docker buildx inspect "$MULTIARCH_BUILDER" --bootstrap >/dev/null
}

update_from_line() {
    local file="$1"
    local pattern="$2"
    local replacement="$3"

    if ! grep -Eq "^FROM ${pattern}\$" "$file"; then
        echo "ERROR: Failed to find pinned base image reference in $file"
        return 1
    fi

    perl -0pi -e "s#^FROM ${pattern}\$#FROM ${replacement}#m" "$file"

    if ! grep -Fqx "FROM ${replacement}" "$file"; then
        echo "ERROR: Failed to update pinned base image reference in $file"
        return 1
    fi
}

pin_consumer_dockerfiles() {
    echo "==> Updating pinned base image references to tag: $TAG"

    update_from_line \
        "$REPO_ROOT/docker/build/linux-x64-gnu.Dockerfile" \
        'ghcr\\.io/rivet-dev/rivet/builder-base-linux-gnu:[^[:space:]]+' \
        "ghcr.io/rivet-dev/rivet/builder-base-linux-gnu:${TAG}"
    update_from_line \
        "$REPO_ROOT/docker/build/linux-arm64-gnu.Dockerfile" \
        'ghcr\\.io/rivet-dev/rivet/builder-base-linux-gnu:[^[:space:]]+' \
        "ghcr.io/rivet-dev/rivet/builder-base-linux-gnu:${TAG}"
    update_from_line \
        "$REPO_ROOT/docker/build/linux-x64-musl.Dockerfile" \
        'ghcr\\.io/rivet-dev/rivet/builder-base-linux-musl:[^[:space:]]+' \
        "ghcr.io/rivet-dev/rivet/builder-base-linux-musl:${TAG}"
    update_from_line \
        "$REPO_ROOT/docker/build/linux-arm64-musl.Dockerfile" \
        'ghcr\\.io/rivet-dev/rivet/builder-base-linux-musl:[^[:space:]]+' \
        "ghcr.io/rivet-dev/rivet/builder-base-linux-musl:${TAG}"
    update_from_line \
        "$REPO_ROOT/docker/build/darwin-x64.Dockerfile" \
        'ghcr\\.io/rivet-dev/rivet/builder-base-osxcross:[^[:space:]]+' \
        "ghcr.io/rivet-dev/rivet/builder-base-osxcross:${TAG}"
    update_from_line \
        "$REPO_ROOT/docker/build/darwin-arm64.Dockerfile" \
        'ghcr\\.io/rivet-dev/rivet/builder-base-osxcross:[^[:space:]]+' \
        "ghcr.io/rivet-dev/rivet/builder-base-osxcross:${TAG}"
    update_from_line \
        "$REPO_ROOT/docker/build/windows-x64.Dockerfile" \
        'ghcr\\.io/rivet-dev/rivet/builder-base-windows-mingw:[^[:space:]]+' \
        "ghcr.io/rivet-dev/rivet/builder-base-windows-mingw:${TAG}"
    update_from_line \
        "$REPO_ROOT/docker/engine/Dockerfile" \
        'ghcr\\.io/rivet-dev/rivet/engine-base-builder:[^[:space:]]+ AS builder' \
        "ghcr.io/rivet-dev/rivet/engine-base-builder:${TAG} AS builder"
    update_from_line \
        "$REPO_ROOT/docker/engine/Dockerfile" \
        'ghcr\\.io/rivet-dev/rivet/engine-base-runtime-full:[^[:space:]]+ AS engine-full-base' \
        "ghcr.io/rivet-dev/rivet/engine-base-runtime-full:${TAG} AS engine-full-base"
    update_from_line \
        "$REPO_ROOT/docker/engine/Dockerfile" \
        'ghcr\\.io/rivet-dev/rivet/engine-base-runtime-slim:[^[:space:]]+' \
        "ghcr.io/rivet-dev/rivet/engine-base-runtime-slim:${TAG}"
}

build_one() {
    local name="$1"
    local dockerfile="$DOCKERFILE_DIR/${name}.Dockerfile"
    local image
    image="$(image_for_base "$name")"

    if [ ! -f "$dockerfile" ]; then
        echo "ERROR: Dockerfile not found: $dockerfile"
        return 1
    fi

    echo "==> Building $image"
    if is_multiarch_base "$name"; then
        ensure_multiarch_builder
        DOCKER_BUILDKIT=1 docker buildx build --builder "$MULTIARCH_BUILDER" --platform linux/amd64 -f "$dockerfile" -t "$image" --load "$REPO_ROOT"
    else
        DOCKER_BUILDKIT=1 docker build -f "$dockerfile" -t "$image" "$REPO_ROOT"
    fi
    echo "==> Built: $image"
}

push_one() {
    local name="$1"
    local dockerfile="$DOCKERFILE_DIR/${name}.Dockerfile"
    local image
    image="$(image_for_base "$name")"
    echo "==> Pushing $image"
    if is_multiarch_base "$name"; then
        ensure_multiarch_builder
        DOCKER_BUILDKIT=1 docker buildx build --builder "$MULTIARCH_BUILDER" --platform linux/amd64,linux/arm64 -f "$dockerfile" -t "$image" --push "$REPO_ROOT"
    else
        docker push "$image"
    fi
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
        pin_consumer_dockerfiles
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
if [ "$BASE_NAME" = "all" ] && [ "$PUSH" = "true" ]; then
    echo "Pinned consumer Dockerfiles to tag: $TAG"
else
    echo "Consumer Dockerfiles were not updated. Run ./scripts/docker-builder-base/build-push.sh all --push to pin them to this tag."
fi

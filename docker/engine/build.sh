#!/bin/bash
set -e

# Default to x86_64-unknown-linux-gnu if no target specified
TARGET=${1:-x86_64-unknown-linux-musl}

case $TARGET in
  x86_64-unknown-linux-musl)
    echo "Building for Linux x86_64 platform"
    DOCKERFILE="linux-x86_64.Dockerfile"
    TARGET_STAGE="x86_64-builder"
    BINARY="rivet-engine-$TARGET"
    ;;
  aarch64-unknown-linux-musl)
    echo "Building for Linux ARM64 platform"
    DOCKERFILE="linux-aarch64.Dockerfile"
    TARGET_STAGE="aarch64-builder"
    BINARY="rivet-engine-$TARGET"
    ;;
  aarch64-apple-darwin)
    echo "Building for macOS ARM64 platform"
    DOCKERFILE="macos-aarch64.Dockerfile"
    TARGET_STAGE="aarch64-builder"
    BINARY="rivet-engine-$TARGET"
    ;;
  x86_64-apple-darwin)
    echo "Building for macOS x86_64 platform"
    DOCKERFILE="macos-x86_64.Dockerfile" 
    TARGET_STAGE="x86_64-builder"
    BINARY="rivet-engine-$TARGET"
    ;;
  x86_64-pc-windows-gnu)
    echo "Building for Windows platform"
    DOCKERFILE="windows.Dockerfile"
    TARGET_STAGE=""  # No target stage for Windows
    BINARY="rivet-engine-$TARGET.exe"
    ;;
  *)
    echo "Unsupported target: $TARGET"
    exit 1
    ;;
esac

# Build docker image with target stage (if specified)
if [ -n "$TARGET_STAGE" ]; then
  DOCKER_BUILDKIT=1 docker build --target $TARGET_STAGE -f docker/engine/$DOCKERFILE -t rivet-engine-builder-$TARGET .
else
  DOCKER_BUILDKIT=1 docker build -f docker/engine/$DOCKERFILE -t rivet-engine-builder-$TARGET .
fi

# Extract binary
CONTAINER_ID=$(docker create rivet-engine-builder-$TARGET)
mkdir -p dist
docker cp "$CONTAINER_ID:/artifacts/$BINARY" dist/
docker rm "$CONTAINER_ID"

# Make binary executable (skip for Windows .exe files)
if [[ ! "$BINARY" == *.exe ]]; then
  chmod +x dist/$BINARY
fi

echo "Binary saved to: dist/$BINARY"
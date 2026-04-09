#!/bin/bash
set -e

# Build rivetkit-native for a specific target using Docker cross-compilation.
# Usage: ./build.sh <target>
# Targets: x86_64-unknown-linux-gnu, aarch64-apple-darwin, x86_64-apple-darwin, x86_64-pc-windows-msvc

TARGET=${1:-x86_64-unknown-linux-gnu}
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

case $TARGET in
  x86_64-unknown-linux-gnu)
    echo "Building rivetkit-native for Linux x86_64"
    DOCKERFILE="linux-x64.Dockerfile"
    ARTIFACT="rivetkit-native.linux-x64-gnu.node"
    ;;
  aarch64-apple-darwin)
    echo "Building rivetkit-native for macOS ARM64"
    DOCKERFILE="macos-arm64.Dockerfile"
    ARTIFACT="rivetkit-native.darwin-arm64.node"
    ;;
  x86_64-apple-darwin)
    echo "Building rivetkit-native for macOS x86_64"
    DOCKERFILE="macos-x64.Dockerfile"
    ARTIFACT="rivetkit-native.darwin-x64.node"
    ;;
  x86_64-pc-windows-msvc)
    echo "Building rivetkit-native for Windows x64 MSVC"
    DOCKERFILE="windows-x64.Dockerfile"
    ARTIFACT="rivetkit-native.win32-x64-msvc.node"
    ;;
  *)
    echo "Unsupported target: $TARGET"
    echo "Supported targets: x86_64-unknown-linux-gnu, aarch64-apple-darwin, x86_64-apple-darwin, x86_64-pc-windows-msvc"
    exit 1
    ;;
esac

# Build using Docker from the repo root context
DOCKER_BUILDKIT=1 docker build \
  -f "$SCRIPT_DIR/$DOCKERFILE" \
  -t rivetkit-native-builder-$TARGET \
  "$REPO_ROOT"

# Extract artifact
CONTAINER_ID=$(docker create rivetkit-native-builder-$TARGET)
mkdir -p "$SCRIPT_DIR/../npm"
docker cp "$CONTAINER_ID:/artifacts/$ARTIFACT" "$SCRIPT_DIR/../$ARTIFACT"
docker rm "$CONTAINER_ID"

echo "Built: $ARTIFACT"

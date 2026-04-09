#!/usr/bin/env bash
#
# Build the sandbox Docker image using pnpm deploy for a flat, symlink-free node_modules.
# rivetkit-native is built on the host and injected into the deploy bundle.
#
# Usage:
#   ./scripts/docker/build-sandbox.sh          # build image only
#   ./scripts/docker/build-sandbox.sh --push   # build + push to Artifact Registry
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DEPLOY_DIR="$REPO_ROOT/.sandbox-deploy"

AR_HOSTNAME="${AR_HOSTNAME:-us-east4-docker.pkg.dev}"
AR_PROJECT_ID="${AR_PROJECT_ID:-dev-projects-491221}"
AR_REPOSITORY="${AR_REPOSITORY:-cloud-run-source-deploy}"
IMAGE_NAMESPACE="${IMAGE_NAMESPACE:-rivet-dev-rivet}"
IMAGE_NAME="${IMAGE_NAME:-rivet-kitchen-sink}"
IMAGE_REPO="${AR_HOSTNAME}/${AR_PROJECT_ID}/${AR_REPOSITORY}/${IMAGE_NAMESPACE}/${IMAGE_NAME}"
COMMIT_SHA="${COMMIT_SHA:-$(git -C "$REPO_ROOT" rev-parse HEAD)}"

PUSH=false
if [[ "${1:-}" == "--push" ]]; then
  PUSH=true
fi

cd "$REPO_ROOT"

# --- Host build ---

echo "==> pnpm install"
pnpm install

echo "==> Building rivetkit-native"
(cd rivetkit-typescript/packages/rivetkit-native && pnpm build)

echo "==> Building sandbox (turbo resolves workspace dep graph)"
pnpm build --filter=sandbox

# --- Create flat deploy bundle ---

echo "==> pnpm deploy --prod"
rm -rf "$DEPLOY_DIR"
pnpm --filter=sandbox deploy --prod "$DEPLOY_DIR"

# pnpm deploy respects .gitignore, so build artifacts may be missing.
if [ ! -d "$DEPLOY_DIR/dist" ]; then
  echo "==> Copying dist/ into deploy dir"
  cp -r examples/sandbox/dist "$DEPLOY_DIR/dist"
fi
# srvx looks for public/ relative to the server entry (dist/public/).
if [ -d "examples/sandbox/public" ]; then
  echo "==> Copying public/ (frontend assets) into deploy dir"
  cp -r examples/sandbox/public "$DEPLOY_DIR/dist/public"
fi

# Remove .dockerignore from deploy dir since it excludes node_modules/dist
rm -f "$DEPLOY_DIR/.dockerignore"

# Inject @rivetkit/rivetkit-native (JS + .node binary) so the engine driver
# uses native SQLite instead of falling back to WASM.
NATIVE_PKG="rivetkit-typescript/packages/rivetkit-native"
NATIVE_DST="$DEPLOY_DIR/node_modules/@rivetkit/rivetkit-native"
echo "==> Injecting @rivetkit/rivetkit-native package"
mkdir -p "$NATIVE_DST"
cp "$NATIVE_PKG/package.json" "$NATIVE_DST/"
cp "$NATIVE_PKG/index.js" "$NATIVE_DST/"
cp "$NATIVE_PKG/index.d.ts" "$NATIVE_DST/"
cp "$NATIVE_PKG/wrapper.js" "$NATIVE_DST/"
cp "$NATIVE_PKG/wrapper.d.ts" "$NATIVE_DST/"
# Copy the .node binary for the Docker target platform.
NATIVE_NODE="$NATIVE_PKG/rivetkit-native.linux-x64-gnu.node"
if [ -f "$NATIVE_NODE" ]; then
  cp "$NATIVE_NODE" "$NATIVE_DST/"
else
  echo "WARNING: $NATIVE_NODE not found, native SQLite will not work"
fi

# Also inject @rivetkit/engine-envoy-protocol (dependency of rivetkit-native/wrapper)
ENVOY_PROTO_PKG="engine/sdks/typescript/envoy-protocol"
ENVOY_PROTO_DST="$DEPLOY_DIR/node_modules/@rivetkit/engine-envoy-protocol"
if [ -d "$ENVOY_PROTO_PKG/dist" ]; then
  echo "==> Injecting @rivetkit/engine-envoy-protocol"
  mkdir -p "$ENVOY_PROTO_DST"
  cp "$ENVOY_PROTO_PKG/package.json" "$ENVOY_PROTO_DST/"
  cp -r "$ENVOY_PROTO_PKG/dist" "$ENVOY_PROTO_DST/dist"
fi

# Inject @rivetkit/sqlite-wasm (the workspace rivetkit dynamically imports this name).
# The source package dir is sqlite-vfs but the import specifier is @rivetkit/sqlite-wasm.
SQLITE_VFS_PKG="rivetkit-typescript/packages/sqlite-vfs"
SQLITE_WASM_DST="$DEPLOY_DIR/node_modules/@rivetkit/sqlite-wasm"
if [ -d "$SQLITE_VFS_PKG/dist" ]; then
  echo "==> Injecting @rivetkit/sqlite-wasm"
  mkdir -p "$SQLITE_WASM_DST"
  cp "$SQLITE_VFS_PKG/package.json" "$SQLITE_WASM_DST/"
  cp -r "$SQLITE_VFS_PKG/dist" "$SQLITE_WASM_DST/dist"
fi

# --- Docker build ---

echo "==> docker build"
docker build \
  -f examples/sandbox/Dockerfile \
  -t "${IMAGE_REPO}:${COMMIT_SHA}" \
  -t "${IMAGE_REPO}:latest" \
  "$DEPLOY_DIR"

echo "Built ${IMAGE_REPO}:${COMMIT_SHA}"

if $PUSH; then
  echo "==> Pushing ${IMAGE_REPO}:${COMMIT_SHA}"
  docker push "${IMAGE_REPO}:${COMMIT_SHA}"
  echo "==> Pushing ${IMAGE_REPO}:latest"
  docker push "${IMAGE_REPO}:latest"
fi

echo "Done"

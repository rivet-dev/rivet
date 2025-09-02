#!/usr/bin/env bash
set -euo pipefail

# Base builder/pusher for engine images.
# Usage: build-push.sh <image_repo> <tag1> [tag2 ...]

if [[ $# -lt 2 ]]; then
  echo "Usage: $(basename "$0") <image_repo> <tag1> [tag2 ...]" >&2
  exit 1
fi

IMAGE_REPO=$1
shift
TAGS=("$@")

DOCKERFILE=${DOCKERFILE:-docker/universal/Dockerfile}
TARGET=${TARGET:-engine-full}
CONTEXT=${CONTEXT:-.}

echo "Building frontend..."
pnpm install
VITE_APP_API_URL=${VITE_APP_API_URL:-/} pnpm turbo build:engine -F @rivetkit/engine-frontend --force

echo "Building ${IMAGE_REPO} with tags: ${TAGS[*]} ..."
BUILD_TAG_ARGS=()
for tag in "${TAGS[@]}"; do
  BUILD_TAG_ARGS+=("-t" "${IMAGE_REPO}:${tag}")
done

docker build -f "${DOCKERFILE}" --target "${TARGET}" "${BUILD_TAG_ARGS[@]}" "${CONTEXT}"

echo "Pushing images..."
for tag in "${TAGS[@]}"; do
  echo "Pushing ${IMAGE_REPO}:${tag}..."
  docker push "${IMAGE_REPO}:${tag}"
done

echo "Done! Images built and pushed: ${TAGS[*]}"


#!/usr/bin/env bash
set -euo pipefail

AR_HOSTNAME=${AR_HOSTNAME:-us-east4-docker.pkg.dev}
AR_PROJECT_ID=${AR_PROJECT_ID:-dev-projects-491221}
AR_REPOSITORY=${AR_REPOSITORY:-cloud-run-source-deploy}
IMAGE_NAMESPACE=${IMAGE_NAMESPACE:-rivet-dev-rivet}
IMAGE_NAME=${IMAGE_NAME:-rivet-kitchen-sink}
IMAGE_REPO="${AR_HOSTNAME}/${AR_PROJECT_ID}/${AR_REPOSITORY}/${IMAGE_NAMESPACE}/${IMAGE_NAME}"

COMMIT_SHA=${COMMIT_SHA:-$(git rev-parse HEAD)}
DOCKERFILE=${DOCKERFILE:-examples/kitchen-sink/Dockerfile}
CONTEXT=${CONTEXT:-.}

echo "Building ${IMAGE_REPO}:${COMMIT_SHA} and ${IMAGE_REPO}:latest"
docker build \
  -f "${DOCKERFILE}" \
  -t "${IMAGE_REPO}:${COMMIT_SHA}" \
  -t "${IMAGE_REPO}:latest" \
  "${CONTEXT}"

echo "Pushing ${IMAGE_REPO}:${COMMIT_SHA}"
docker push "${IMAGE_REPO}:${COMMIT_SHA}"

echo "Pushing ${IMAGE_REPO}:latest"
docker push "${IMAGE_REPO}:latest"

echo "Done"

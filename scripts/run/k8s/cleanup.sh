#!/usr/bin/env bash
set -euo pipefail

CLUSTER_NAME="rivet"

if ! command -v k3d >/dev/null 2>&1; then
  echo "error: required command 'k3d' not found."
  exit 1
fi

if k3d cluster list | grep -qw "^${CLUSTER_NAME} "; then
  echo "Deleting k3d cluster '${CLUSTER_NAME}'..."
  k3d cluster delete "${CLUSTER_NAME}"
  echo "Cluster deleted successfully."
else
  echo "Cluster '${CLUSTER_NAME}' not found."
fi

#!/usr/bin/env bash
set -euf -o pipefail

# Run container
CONTAINER_ID=$(cat "$NOMAD_ALLOC_DIR/container-id")
runc run $CONTAINER_ID -b "$NOMAD_ALLOC_DIR/oci-bundle"


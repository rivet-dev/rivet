#!/usr/bin/env bash
set -euf -o pipefail

# Setup
"$NOMAD_TASK_DIR/setup.sh" > "$NOMAD_ALLOC_DIR/logs/setup.stdout" 2> "$NOMAD_ALLOC_DIR/logs/setup.stderr"

# Run container
CONTAINER_ID=$(cat "$NOMAD_TASK_DIR/container-id")
runc run $CONTAINER_ID -b "$NOMAD_TASK_DIR/oci-bundle"

# TODO: Move this to poststop
# MARK: Cleanup
# Clean up: remove network and namespace (you may want to do this later or on failure)
# cnitool del $NETWORK_NAME $NETNS_PATH
# ip netns del "$CONTAINER_ID"


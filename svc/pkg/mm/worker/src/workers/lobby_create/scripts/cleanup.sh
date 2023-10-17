#!/usr/bin/env bash
set -euf -o pipefail

# Every step in this script gracefully fails so everything gets cleaned up no matter what.

if [ -f "$NOMAD_ALLOC_DIR/container-id" ]; then
	CONTAINER_ID=$(cat "$NOMAD_ALLOC_DIR/container-id")
	NETWORK_NAME="rivet-job"
	NETNS_PATH="/var/run/netns/$CONTAINER_ID"

	echo "Deleting container $CONTAINER_ID"
	runc delete --force "$CONTAINER_ID" || echo 'Failed to delete container' >&2

	echo "Deleting network $NETWORK_NAME from namespace $NETNS_PATH"
	cnitool del $NETWORK_NAME $NETNS_PATH || echo 'Failed to delete network' >&2

	echo "Deleting network $CONTAINER_ID" || echo 'Failed to delete network' >&2
	ip netns del "$CONTAINER_ID"
else
	echo "No container ID found. Network may have leaked." >&2
fi


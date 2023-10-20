#!/usr/bin/env bash
set -euf -o pipefail

log() {
    local timestamp=$(date +"%Y-%m-%d %H:%M:%S.%3N")
    echo "[$timestamp] $@"
}

# Every step in this script gracefully fails so everything gets cleaned up no matter what.

if [ -f "$NOMAD_ALLOC_DIR/container-id" ]; then
	CONTAINER_ID=$(cat "$NOMAD_ALLOC_DIR/container-id")
	NETWORK_NAME="rivet-job"
	NETNS_PATH="/var/run/netns/$CONTAINER_ID"

	log "Deleting container $CONTAINER_ID"
	runc delete --force "$CONTAINER_ID" || log 'Failed to delete container' >&2

	log "Deleting network $NETWORK_NAME from namespace $NETNS_PATH"
	cnitool del $NETWORK_NAME $NETNS_PATH || log 'Failed to delete network' >&2

	log "Deleting network $CONTAINER_ID"
	ip netns del "$CONTAINER_ID" || log 'Failed to delete network' >&2
else
	log "No container ID found. Network may have leaked." >&2
fi


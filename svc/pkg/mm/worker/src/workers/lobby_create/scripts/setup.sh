#!/usr/bin/env bash
set -euf -o pipefail

log() {
    local timestamp=$(date +"%Y-%m-%d %H:%M:%S.%3N")
    echo "[$timestamp] [setup] $@"
}

log "Starting setup"

log 'Env:'
env
echo

# Need to prefix with "rivet-" in order to not interfere with any
# auto-generated resources that Nomad creates for the given alloc ID
export CONTAINER_ID="rivet-$NOMAD_ALLOC_ID"
log "CONTAINER_ID: $CONTAINER_ID"
echo -n "$CONTAINER_ID" > "$NOMAD_ALLOC_DIR/container-id"

# Path to the created namespace
if __HOST_NETWORK__; then
	# Host network
	export NETNS_PATH="/proc/1/ns/net"
else
	# CNI network that will be created
	export NETNS_PATH="/var/run/netns/$CONTAINER_ID"
fi

# Run setup scripts
"$NOMAD_TASK_DIR/setup_oci_bundle.sh" &
if __HOST_NETWORK__; then
	"$NOMAD_TASK_DIR/setup_cni_network.sh" &
fi
wait

log "Setup finished"


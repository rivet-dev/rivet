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
export NETNS_PATH="/var/run/netns/$CONTAINER_ID"

# Run setup scripts
"$NOMAD_TASK_DIR/setup_oci_bundle.sh" &
"$NOMAD_TASK_DIR/setup_cni_network.sh" &
wait

log "Setup finished"


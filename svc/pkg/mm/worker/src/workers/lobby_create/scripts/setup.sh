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

# Run OCI setup script
"$NOMAD_TASK_DIR/setup_oci_bundle.sh" &
pid_oci=$!

# Run CNI setup script
if ! __HOST_NETWORK__; then
	"$NOMAD_TASK_DIR/setup_cni_network.sh" &
	pid_cni=$!
fi

# Wait for OCI setup scripts to finish
wait $pid_oci
exit_status_oci=$?
if [ $exit_status_oci -ne 0 ]; then
	log "OCI setup failed with exit code $exit_status_oci"
	exit $exit_status_oci
fi

# Wait for CNI setup script to finish
if ! __HOST_NETWORK__; then
    wait $pid_cni
    exit_status_cni=$?
	if [ $exit_status_cni -ne 0 ]; then
		log "CNI setup failed with exit code $exit_status_cni"
		exit $exit_status_cni
	fi
fi

log "Setup finished"


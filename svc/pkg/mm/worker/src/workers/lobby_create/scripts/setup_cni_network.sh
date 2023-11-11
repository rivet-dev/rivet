#!/usr/bin/env bash
set -euf -o pipefail

log() {
    local timestamp=$(date +"%Y-%m-%d %H:%M:%S.%3N")
    echo "[$timestamp] [setup_cni_network] $@"
}

export CNI_PATH="/opt/cni/bin"
export NETCONFPATH="/opt/cni/config"

# MARK: Create network
#
# See Nomad network creation:
# https://github.com/hashicorp/nomad/blob/a8f0f2612ef9d283ed903721f8453a0c0c3f51c5/client/allocrunner/network_manager_linux.go#L119

# Name of the network in /opt/cni/config/$NETWORK_NAME.conflist
NETWORK_NAME="rivet-job"

# See Nomad capabilities equivalent:
# https://github.com/hashicorp/nomad/blob/a8f0f2612ef9d283ed903721f8453a0c0c3f51c5/client/allocrunner/networking_cni.go#L105C46-L105C46
#
# See supported args:
# https://github.com/containerd/go-cni/blob/6603d5bd8941d7f2026bb5627f6aa4ff434f859a/namespace_opts.go#L22
export CAP_ARGS=$(jq -c <<EOF
{
	"portMappings": $(cat "$NOMAD_ALLOC_DIR/cni-port-mappings.json")
}
EOF
)
export CNI_IFNAME="eth0"
log "CAP_ARGS: $CAP_ARGS"

log "Creating network $CONTAINER_ID"
ip netns add "$CONTAINER_ID"

log "Adding network $NETWORK_NAME to namespace $NETNS_PATH"
cnitool add "$NETWORK_NAME" "$NETNS_PATH" > $NOMAD_ALLOC_DIR/cni.json

log "Finished setting up CNI network"


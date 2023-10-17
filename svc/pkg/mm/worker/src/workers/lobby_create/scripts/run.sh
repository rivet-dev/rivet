#!/usr/bin/env bash
set -euf -o pipefail

echo "$(pwd)"

# TODO: Update NETCONF_PATH to /opt/cni/net.d
export CNI_PATH="/opt/cni/bin"
export NETCONFPATH="/opt/cni/config"

DOCKER_IMAGE_PATH="$NOMAD_TASK_DIR/docker-image.tar"
OCI_IMAGE_PATH="$NOMAD_TASK_DIR/oci-image"
OCI_BUNDLE_PATH="$NOMAD_TASK_DIR/oci-bundle"

# Need to prefix with "rivet-" in order to not interfere with any
# auto-generated resources that Nomad creates for the given alloc ID
CONTAINER_ID="rivet-$NOMAD_ALLOC_ID"
echo "CONTAINER_ID: $CONTAINER_ID"


# MARK: Load container
echo "Converting Docker image -> OCI image"
time skopeo copy "docker-archive:$DOCKER_IMAGE_PATH" "oci:$OCI_IMAGE_PATH:default"

# TODO: Remov hjke
# Install umoci
curl -Lf -o umoci 'https://github.com/opencontainers/umoci/releases/download/v0.4.7/umoci.amd64'
chmod +x umoci

# This allows us to run the bundle natively with runc
echo "Converting OCI image -> OCI bundle"
time ./umoci unpack --image "$OCI_IMAGE_PATH:default" "$OCI_BUNDLE_PATH"


# MARK: Create network
#
# Network manager:
# https://github.com/hashicorp/nomad/blob/6dcc4021882fcaecb7ee73655bd46eb84e4671d4/client/allocrunner/network_manager_linux.go#L120
#
# CNI:
# https://github.com/hashicorp/nomad/blob/6dcc4021882fcaecb7ee73655bd46eb84e4671d4/client/allocrunner/networking_cni.go
#
# Allco hook:
# https://github.com/hashicorp/nomad/blob/6dcc4021882fcaecb7ee73655bd46eb84e4671d4/client/allocrunner/network_hook.go#L107
#
# Go CNI (implements args):
# https://github.com/containerd/go-cni/blob/6603d5bd8941d7f2026bb5627f6aa4ff434f859a/namespace_opts.go#L22
#
# CNI:
# https://github.com/containernetworking/cni
# Name of the network in /opt/cni/config/$NETWORK_NAME.conflist
NETWORK_NAME="rivet-job"
# Path to the created namespace
NETNS_PATH="/var/run/netns/$CONTAINER_ID"

# export CNI_IFNAME="eth"


# https://github.com/hashicorp/nomad/blob/6dcc4021882fcaecb7ee73655bd46eb84e4671d4/client/allocrunner/networking_cni.go#L347
# export CAP_ARGS=$(cat <<EOF
# {
# 	"portMappings": [
# 		"HostPort": $NOMAD_HOST_PORT_http,
# 		"ContainerPort": 8080,
# 		"Protocol": "tcp"
# 	]
# }
# EOF
# )
export CNI_IFNAME="eth0"

echo "Creating network $NETWORK_NAME"
ip netns add "$CONTAINER_ID"

echo "Adding network $NETWORK_NAME to namespace $NETNS_PATH"
cnitool add "$NETWORK_NAME" "$NETNS_PATH" > $NOMAD_TASK_DIR/cni.json

cat <<EOF > $NOMAD_TASK_DIR/resolv.conf
nameserver 8.8.8.8
nameserver 8.8.4.4
nameserver 2001:4860:4860::8888
nameserver 2001:4860:4860::8844
options rotate
options edns0
options attempts:2
EOF

# MARK: Config
# Copy the Docker-specific values from the OCI bundle config.json to the base config
#
# This way, we enforce our own capabilities on the container instead of trusting the
# provided config.json
echo "Templating config.json"
OVERRIDE_CONFIG="$NOMAD_TASK_DIR/oci-bundle-config.overrides.json"
mv "$OCI_BUNDLE_PATH/config.json" "$OVERRIDE_CONFIG"
jq "
.process.args = $(jq '.process.args' $OVERRIDE_CONFIG) |
.process.env = $(jq '.process.env' $OVERRIDE_CONFIG) |
.process.user = $(jq '.process.user' $OVERRIDE_CONFIG) |
.process.cwd = $(jq '.process.cwd' $OVERRIDE_CONFIG) |
.linux.namespaces += [{\"type\": \"network\", \"path\": \"$NETNS_PATH\"}] |
.mounts += [{
	\"destination\": \"/etc/resolv.conf\",
	\"type\": \"bind\",
	\"source\": \"$NOMAD_TASK_DIR/resolv.conf\",
	\"options\": [\"bind\", \"ro\"]
}]
" "$NOMAD_TASK_DIR/oci-bundle-config.base.json" > "$OCI_BUNDLE_PATH/config.json"


# MARK: Run container
echo "Running container"
runc run $CONTAINER_ID -b $OCI_BUNDLE_PATH


# TODO: Move this to poststop
# MARK: Cleanup
# Clean up: remove network and namespace (you may want to do this later or on failure)
cnitool del $NETWORK_NAME $NETNS_PATH
ip netns del "$CONTAINER_ID"


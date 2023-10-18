#!/usr/bin/env bash
set -euf -o pipefail

echo 'Env:'
env
echo

# Need to prefix with "rivet-" in order to not interfere with any
# auto-generated resources that Nomad creates for the given alloc ID
CONTAINER_ID="rivet-$NOMAD_ALLOC_ID"
echo "CONTAINER_ID: $CONTAINER_ID"
echo -n "$CONTAINER_ID" > "$NOMAD_ALLOC_DIR/container-id"

export CNI_PATH="/opt/cni/bin"
export NETCONFPATH="/opt/cni/config"

DOCKER_IMAGE_PATH="$NOMAD_ALLOC_DIR/docker-image.tar"
OCI_IMAGE_PATH="$NOMAD_ALLOC_DIR/oci-image"
OCI_BUNDLE_PATH="$NOMAD_ALLOC_DIR/oci-bundle"

# MARK: Genreate OCI bundle
case "__BUILD_KIND__" in
	"docker-image")
		# We need to conver the Docker image to an OCI bundle in order to run it.

		echo "Downloading Docker image"
		time (__DOWNLOAD_CMD__ > "$DOCKER_IMAGE_PATH")

		# Allows us to work with the build with umoci
		echo "Converting Docker image -> OCI image"
		time skopeo copy "docker-archive:$DOCKER_IMAGE_PATH" "oci:$OCI_IMAGE_PATH:default"

		# Allows us to run the bundle natively with runc
		echo "Converting OCI image -> OCI bundle"
		time umoci unpack --image "$OCI_IMAGE_PATH:default" "$OCI_BUNDLE_PATH"

		;;
	"oci-bundle")
		echo "Downloading OCI bundle"
		time (__DOWNLOAD_CMD__ | tar -x -C "$OCI_BUNDLE_PATH")

		;;
	*)
		echo "Unknown build kind"
		exit 1
		;;
esac


# MARK: Create network
#
# See Nomad network creation:
# https://github.com/hashicorp/nomad/blob/a8f0f2612ef9d283ed903721f8453a0c0c3f51c5/client/allocrunner/network_manager_linux.go#L119

# Name of the network in /opt/cni/config/$NETWORK_NAME.conflist
NETWORK_NAME="rivet-job"
# Path to the created namespace
NETNS_PATH="/var/run/netns/$CONTAINER_ID"

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
echo "CAP_ARGS: $CAP_ARGS"

echo "Creating network $CONTAINER_ID"
ip netns add "$CONTAINER_ID"

echo "Adding network $NETWORK_NAME to namespace $NETNS_PATH"
cnitool add "$NETWORK_NAME" "$NETNS_PATH" > $NOMAD_ALLOC_DIR/cni.json

# resolv.conf
cat <<EOF > $NOMAD_ALLOC_DIR/resolv.conf
nameserver 8.8.8.8
nameserver 8.8.4.4
nameserver 2001:4860:4860::8888
nameserver 2001:4860:4860::8844
options rotate
options edns0
options attempts:2
EOF

# MARK: Config
#
# Sanitize the config.json by copying safe properties from the provided bundle in to our base config.
#
# This way, we enforce our own capabilities on the container instead of trusting the
# provided config.json
echo "Templating config.json"
OVERRIDE_CONFIG="$NOMAD_ALLOC_DIR/oci-bundle-config.overrides.json"
mv "$OCI_BUNDLE_PATH/config.json" "$OVERRIDE_CONFIG"
jq "
.process.args = $(jq '.process.args' $OVERRIDE_CONFIG) |
.process.env = $(jq '.process.env' $OVERRIDE_CONFIG) + .process.env |
.process.user = $(jq '.process.user' $OVERRIDE_CONFIG) |
.process.cwd = $(jq '.process.cwd' $OVERRIDE_CONFIG) |
.linux.namespaces += [{\"type\": \"network\", \"path\": \"$NETNS_PATH\"}] |
.mounts += [{
	\"destination\": \"/etc/resolv.conf\",
	\"type\": \"bind\",
	\"source\": \"$NOMAD_ALLOC_DIR/resolv.conf\",
	\"options\": [\"bind\", \"ro\"]
}]
" "$NOMAD_ALLOC_DIR/oci-bundle-config.base.json" > "$OCI_BUNDLE_PATH/config.json"


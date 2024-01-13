#!/usr/bin/env bash
set -euf -o pipefail

log() {
    local timestamp=$(date +"%Y-%m-%d %H:%M:%S.%3N")
    echo "[$timestamp] [setup_oci_bundle] $@"
}

DOCKER_IMAGE_PATH="$NOMAD_ALLOC_DIR/docker-image.tar"
OCI_IMAGE_PATH="$NOMAD_ALLOC_DIR/oci-image"
OCI_BUNDLE_PATH="$NOMAD_ALLOC_DIR/oci-bundle"

# MARK: Generate OCI bundle
case "__BUILD_KIND__" in
	"docker-image")
		# We need to convert the Docker image to an OCI bundle in order to run it.

		log "Downloading Docker image"
		__DOWNLOAD_CMD__ > "$DOCKER_IMAGE_PATH"

		# Allows us to work with the build with umoci
		log "Converting Docker image -> OCI image"
		skopeo copy "docker-archive:$DOCKER_IMAGE_PATH" "oci:$OCI_IMAGE_PATH:default"

		# Allows us to run the bundle natively with runc
		log "Converting OCI image -> OCI bundle"

		umoci unpack --image "$OCI_IMAGE_PATH:default" "$OCI_BUNDLE_PATH"
		;;
	"oci-bundle")
		log "Downloading OCI bundle"
		mkdir "$OCI_BUNDLE_PATH"
		__DOWNLOAD_CMD__ | tar -x -C "$OCI_BUNDLE_PATH"

		;;
	*)
		log "Unknown build kind"
		exit 1
		;;
esac

# resolv.conf
#
# See also rivet-job.conflist in lib/bolt/core/src/dep/terraform/install_scripts/files/nomad.sh
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
log "Templating config.json"
OVERRIDE_CONFIG="$NOMAD_ALLOC_DIR/oci-bundle-config.overrides.json"
mv "$OCI_BUNDLE_PATH/config.json" "$OVERRIDE_CONFIG"


# Template new config
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
	\"options\": [\"rbind\", \"rprivate\"]
}]
" "$NOMAD_ALLOC_DIR/oci-bundle-config.base.json" > "$OCI_BUNDLE_PATH/config.json"

# Validate config
if [ "$(jq '.process.user.uid' "$OVERRIDE_CONFIG")" == "0" ]; then
	log "Container is attempting to run as root user"
	exit 1
fi
if [ "$(jq '.process.user.gid' "$OVERRIDE_CONFIG")" == "0" ]; then
	log "Container is attempting to run as root group"
	exit 1
fi

log "Finished setting up OCI bundle"


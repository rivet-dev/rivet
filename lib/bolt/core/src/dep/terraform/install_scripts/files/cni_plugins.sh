# Define version
version="1.3.0"

# Download and extract CNI plugins
mkdir -p /opt/cni-plugins-$version
curl -L -o /tmp/cni-plugins.tgz https://github.com/containernetworking/plugins/releases/download/v${version}/cni-plugins-linux-amd64-v${version}.tgz
tar -xz -C /opt/cni-plugins-$version -f /tmp/cni-plugins.tgz
#
# TODO: Verify hash

# Copy plugins to /opt/cni/bin
mkdir -p /opt/cni/bin
cp -r /opt/cni-plugins-$version/* /opt/cni/bin/


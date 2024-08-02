# !!!!!!!!!!!!!!!!!!!!!
#
# DO NOT UPGRADE
#
# This is the last MPL
# licensed version of
# Nomad.
#
# !!!!!!!!!!!!!!!!!!!!!!
version="1.6.0"

# Allow container traffic to be routed through IP tables
#
# See https://developer.hashicorp.com/nomad/docs/install#post-installation-steps
cat << 'EOF' > /etc/sysctl.d/20-nomad.conf
net.bridge.bridge-nf-call-arptables = 1
net.bridge.bridge-nf-call-ip6tables = 1
net.bridge.bridge-nf-call-iptables = 1
EOF

sysctl --system

# Download and unzip nomad
mkdir -p /opt/nomad-$version
curl -L -o /tmp/nomad.zip https://releases.hashicorp.com/nomad/$version/nomad_${version}_linux_amd64.zip
unzip -o /tmp/nomad.zip -d /opt/nomad-$version/

# TODO: Verify hash

# Create symlink in /usr/local/bin
ln -sf /opt/nomad-$version/nomad /usr/local/bin/nomad

# Test nomad installation
if ! nomad version | grep -q "Nomad v$version"; then
  echo "Nomad version mismatch"
  exit 1
fi

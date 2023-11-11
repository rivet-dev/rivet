# Add Docker GPG key
curl -fsSL https://download.docker.com/linux/debian/gpg | apt-key add -

# Add Docker repository
echo 'deb [arch=amd64] https://download.docker.com/linux/debian bullseye stable' > /etc/apt/sources.list.d/docker.list

# Install Docker
apt-get update -y
apt-get install -y docker-ce docker-ce-cli containerd.io

# Add daemon.json
# 
# Enable live restore in order to ensure that container stay alive
# if we update Docker.
#
# Enable IPv6 on the default bridge network.
cat << 'EOF' > /etc/docker/daemon.json
{
	"experimental": true,

	"live-restore": true,

	"ipv6": true,
	"fixed-cidr-v6": "2001:db8:1::/64",
	"ip6tables": true
}
EOF

# Test Docker installation
docker run hello-world


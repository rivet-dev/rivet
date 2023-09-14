# Add Docker GPG key
curl -fsSL https://download.docker.com/linux/debian/gpg | apt-key add -

# Add Docker repository
echo 'deb [arch=amd64] https://download.docker.com/linux/debian bullseye stable' > /etc/apt/sources.list.d/docker.list

# Install Docker
apt-get update -y
apt-get install -y docker-ce docker-ce-cli containerd.io

# Add daemon.json (Replace with actual path)
cat << 'EOF' > /etc/docker/daemon.json
{
	"live-restore": true
}
EOF
chmod 440 /etc/docker/daemon.json

# Test Docker installation
docker run hello-world


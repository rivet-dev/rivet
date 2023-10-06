# Use Traefik v3 beta for TLS servers transport support
version="3.0.0-beta3"

# Create traefik user
if ! id -u "traefik" &>/dev/null; then
	useradd -r -s /bin/false traefik
fi

# Install traefik
mkdir -p "/opt/traefik-${version}"
curl -L "https://github.com/traefik/traefik/releases/download/v${version}/traefik_v${version}_linux_amd64.tar.gz" -o "/tmp/traefik_v${version}.tar.gz"
tar zxvf "/tmp/traefik_v${version}.tar.gz" -C "/opt/traefik-${version}"
install -o traefik -g traefik "/opt/traefik-${version}/traefik" /usr/bin/

# Check traefik version
if [ "$(traefik version | grep -oP 'Version:\s*\K[^\s]+')" = "${version}" ]; then
	echo "Successfully installed Traefik ${version}"
else
	echo "Traefik version mismatch"
	exit 1
fi


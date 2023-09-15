version="2.10.4"

# Create traefik user
if ! id -u "traefik" &>/dev/null; then
	useradd -r -s /bin/false traefik
fi

# Install traefik
mkdir -p "/opt/traefik-${version}"
curl -L "https://github.com/traefik/traefik/releases/download/v${version}/traefik_v${version}_linux_amd64.tar.gz" -o "/tmp/traefik_v${version}.tar.gz"
tar zxvf "/tmp/traefik_v${version}.tar.gz" -C "/opt/traefik-${version}"
cp "/opt/traefik-${version}/traefik" /usr/bin/
chown traefik:traefik /usr/bin/traefik
chmod 755 /usr/bin/traefik

# Check traefik version
if [ "$(traefik version | grep -oP 'Version:\s*\K[^\s]+')" = "${version}" ]; then
	echo "Successfully installed Traefik ${version}"
else
	echo "Traefik version mismatch"
	exit 1
fi


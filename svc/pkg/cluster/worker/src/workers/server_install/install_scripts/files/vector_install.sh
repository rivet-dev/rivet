version="0.34.1"

# Create vector user
if ! id -u "vector" &>/dev/null; then
	useradd -r -s /bin/false vector
fi

# Install vector
mkdir -p "/opt/vector-${version}"
curl -L "https://github.com/vectordotdev/vector/releases/download/v${version}/vector-${version}-x86_64-unknown-linux-gnu.tar.gz" -o "/tmp/vector_${version}.tar.gz"
tar zxvf "/tmp/vector_${version}.tar.gz" -C "/opt/vector-${version}"
install -o vector -g vector "/opt/vector-${version}/vector-x86_64-unknown-linux-gnu/bin/vector" /usr/bin/

# Check vector version
if vector --version | grep "vector ${version}"; then
	echo "Successfully installed Vector ${version}"
else
	echo "Vector version mismatch"
	exit 1
fi

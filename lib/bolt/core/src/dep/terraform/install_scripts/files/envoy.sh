version="1.27.1"

# Install Envoy
mkdir -p "/opt/envoy-${version}"
curl -L "https://github.com/envoyproxy/envoy/releases/download/v${version}/envoy-${version}-linux-x86_64" -o "/tmp/envoy_${version}"
install "/tmp/envoy_${version}" /usr/bin/envoy


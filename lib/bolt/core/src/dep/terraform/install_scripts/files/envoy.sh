version="1.27.1"

# Install Envoy
mkdir -p "/opt/envoy-${version}"
curl -L "https://github.com/envoyproxy/envoy/releases/download/v${version}/envoy-${version}-linux-x86_64" -o "/tmp/envoy_${version}"
install "/tmp/envoy_${version}" /usr/bin/envoy

# Install hot-restarter.py
#
# See https://www.envoyproxy.io/docs/envoy/latest/operations/hot_restarter.html
curl -L "https://raw.githubusercontent.com/envoyproxy/envoy/v${version}/restarter/hot-restarter.py" -o "/tmp/envoy_hot_restarter_${version}.py"
install "/tmp/envoy_hot_restarter_${version}.py" /usr/bin/envoy_hot_restarter.py


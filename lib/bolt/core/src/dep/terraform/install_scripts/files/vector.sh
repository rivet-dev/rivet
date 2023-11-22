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

# Write config
mkdir -p /etc/vector /var/lib/vector

cat << 'EOF' > /etc/vector/vector.toml
__VECTOR_CONFIG__
EOF

chown -R vector:vector /etc/vector /var/lib/vector

# Systemd service
#
# See https://github.com/vectordotdev/vector/blob/c9804f0c9e5a0931bbaaffe1270021d9c960fcb8/distribution/systemd/vector.service
cat << 'EOF' > /etc/systemd/system/vector.service
[Unit]
Description=Vector
Documentation=https://vector.dev
After=network-online.target
Requires=network-online.target

[Service]
User=vector
Group=vector
ExecStartPre=/usr/bin/vector validate
ExecStart=/usr/bin/vector
ExecReload=/usr/bin/vector validate
ExecReload=/bin/kill -HUP $MAINPID
Restart=always
AmbientCapabilities=CAP_NET_BIND_SERVICE
# Since systemd 229, should be in [Unit] but in order to support systemd <229,
# it is also supported to have it here.
StartLimitInterval=10
StartLimitBurst=5

[Install]
WantedBy=multi-user.target
EOF

# Start and enable the service
systemctl daemon-reload
systemctl enable vector
systemctl start vector


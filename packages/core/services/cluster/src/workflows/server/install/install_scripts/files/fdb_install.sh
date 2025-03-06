sysctl --system

# For clarity
FDB_VERSION="__FDB_VERSION__"

mkdir -p /etc/foundationdb

# Custom cluster description and ID (https://apple.github.io/foundationdb/administration.html#cluster-file-format)
cat << 'EOF' > /etc/foundationdb/fdb.cluster
fdb:fdb@127.0.0.1:4500
EOF

echo 'Downloading fdb client'
curl -Lf -o "/tmp/foundationdb-clients_${FDB_VERSION}-1_amd64.deb" "https://github.com/apple/foundationdb/releases/download/${FDB_VERSION}/foundationdb-clients_${FDB_VERSION}-1_amd64.deb"
dpkg -i "/tmp/foundationdb-clients_${FDB_VERSION}-1_amd64.deb"

# Verify installation
fdbcli --version

echo 'Downloading fdb server'
curl -Lf -o "/tmp/foundationdb-server_${FDB_VERSION}-1_amd64.deb" "https://github.com/apple/foundationdb/releases/download/${FDB_VERSION}/foundationdb-server_${FDB_VERSION}-1_amd64.deb"
dpkg -i "/tmp/foundationdb-server_${FDB_VERSION}-1_amd64.deb"

# Verify installation
fdbserver --version

# Required for 7.3
# fdbcli --exec "configure perpetual_storage_wiggle=1 storage_migration_type=gradual"

# https://apple.github.io/foundationdb/administration.html#administration-running-foundationdb
# Configure redundancy and storage engine
fdbcli --exec "configure new single ssd"

pip install wheel "foundationdb==${FDB_VERSION}" prometheus_client

cat << 'EOF' > /usr/local/bin/fdb_prometheus_proxy.py
__PROMETHEUS_PROXY_SCRIPT__
EOF

# Systemd service
cat << 'EOF' > /etc/systemd/system/fdb_prometheus_proxy.service
[Unit]
Description=FDB Prometheus Proxy
After=network-online.target
Requires=network-online.target

[Service]
ExecStart=/usr/bin/python3 /usr/local/bin/fdb_prometheus_proxy.py --fdb-cluster-file /etc/foundationdb/fdb.cluster
Restart=always
RestartSec=2

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable fdb_prometheus_proxy

# NOTE: systemd service for foundationdb automatically created from dpkg:
# https://apple.github.io/foundationdb/administration.html#administration-running-foundationdb

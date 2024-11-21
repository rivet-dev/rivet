sysctl --system

mkdir -p /etc/foundationdb

curl -Lf -o /tmp/foundationdb-clients_OFF-1_amd64.deb "https://github.com/apple/foundationdb/releases/download/7.3.43/foundationdb-clients_7.3.43-1_amd64.deb"
dpkg -i /tmp/foundationdb-clients_OFF-1_amd64.deb

# Verify installation
fdbcli --version

curl -Lf -o /tmp/foundationdb-server_OFF-1_amd64.deb "https://github.com/apple/foundationdb/releases/download/7.3.43/foundationdb-server_7.3.43-1_amd64.deb"
dpkg -i /tmp/foundationdb-server_OFF-1_amd64.deb

# Verify installation
fdbserver --version

# https://apple.github.io/foundationdb/administration.html#administration-running-foundationdb
# Configure redundancy and storage engine
fdbcli --exec "configure perpetual_storage_wiggle=1 storage_migration_type=gradual"
fdbcli --exec "configure single ssd"
service foundationdb stop


pip install wheel foundationdb prometheus_client

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

# NOTE: we dont have a systemd service for fdbserver because it uses `service`:
# https://apple.github.io/foundationdb/administration.html#administration-running-foundationdb

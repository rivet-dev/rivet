PUBLIC_IP=$(ip -4 route get 1.0.0.0 | awk '{print $7; exit}')

# Write config
mkdir -p /etc/vector /var/lib/vector

cat << EOF > /etc/vector/vector.toml
__VECTOR_CONFIG__
EOF

# Vector user created in vector_install.sh script
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
ExecStartPre=/usr/bin/vector validate --config-toml /etc/vector/vector.toml
ExecStart=/usr/bin/vector --config-toml /etc/vector/vector.toml
ExecReload=/usr/bin/vector validate --config-toml /etc/vector/vector.toml
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


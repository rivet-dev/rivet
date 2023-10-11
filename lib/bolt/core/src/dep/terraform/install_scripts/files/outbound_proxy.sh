# Create user
if ! id -u "outbound_proxy" &>/dev/null; then
        useradd -r -s /bin/false outbound_proxy
fi

# Write config
mkdir -p /etc/outbound_proxy

cat << 'EOF' > /etc/outbound_proxy/envoy.yaml
__ENVOY_CONFIG__
EOF

chown -R outbound_proxy:outbound_proxy /etc/outbound_proxy

# Systemd service
cat << 'EOF' > /etc/systemd/system/outbound_proxy.service
[Unit]
Description=Outbound Proxy
After=network.target

[Service]
User=outbound_proxy
Group=outbound_proxy
Restart=always
ExecStart=/usr/bin/envoy -c /etc/outbound_proxy/envoy.yaml
LimitNOFILE=640000

[Install]
WantedBy=multi-user.target
EOF

# Start and enable the service
systemctl daemon-reload
systemctl enable outbound_proxy
systemctl start outbound_proxy


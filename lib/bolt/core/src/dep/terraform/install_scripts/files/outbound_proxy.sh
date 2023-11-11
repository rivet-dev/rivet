# Create user
if ! id -u "outbound_proxy" &>/dev/null; then
        useradd -r -s /bin/false outbound_proxy
fi

# Config
mkdir -p /etc/outbound_proxy

cat << 'EOF' > /etc/outbound_proxy/envoy.yaml
__ENVOY_CONFIG__
EOF

chown -R outbound_proxy:outbound_proxy /etc/outbound_proxy

# Startup scripts
#
# See https://www.envoyproxy.io/docs/envoy/latest/operations/hot_restarter.html
cat << 'EOF' > /etc/outbound_proxy/start_envoy.sh
#!/bin/bash
set -e
exec /usr/bin/envoy -c /etc/outbound_proxy/envoy.yaml --restart-epoch $RESTART_EPOCH
EOF

cat << 'EOF' > /etc/outbound_proxy/check_envoy.sh
#!/bin/sh
set -e
/usr/bin/envoy -c /etc/outbound_proxy/envoy.yaml --mode validate
EOF

cat << 'EOF' > /etc/outbound_proxy/reload_envoy.sh
#!/bin/sh
set -e
/usr/bin/envoy -c /etc/outbound_proxy/envoy.yaml --mode validate
kill -1 $1
EOF

chmod +x /etc/outbound_proxy/start_envoy.sh /etc/outbound_proxy/check_envoy.sh /etc/outbound_proxy/reload_envoy.sh

# Systemd service
cat << 'EOF' > /etc/systemd/system/outbound_proxy.service
[Unit]
Description=Outbound Proxy
After=network.target

[Service]
User=outbound_proxy
Group=outbound_proxy
Restart=always
ExecStart=/usr/bin/envoy_hot_restarter.py /etc/outbound_proxy/start_envoy.sh
ExecStartPre=/etc/outbound_proxy/check_envoy.sh
ExecReload=/etc/outbound_proxy/reload_envoy.sh $MAINPID
LimitNOFILE=102400
TimeoutStopSec=10
KillMode=process

[Install]
WantedBy=multi-user.target
EOF

# Start and enable the service
systemctl daemon-reload
systemctl enable outbound_proxy
systemctl start outbound_proxy

# Reload config if already running
systemctl reload outbound_proxy


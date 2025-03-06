if ! id -u "__TRAEFIK_INSTANCE_NAME__" &>/dev/null; then
	useradd -r -s /bin/false __TRAEFIK_INSTANCE_NAME__
fi

mkdir -p /etc/__TRAEFIK_INSTANCE_NAME__ /etc/__TRAEFIK_INSTANCE_NAME__/dynamic /etc/__TRAEFIK_INSTANCE_NAME__/dynamic/tls /etc/__TRAEFIK_INSTANCE_NAME__/tls /opt/__TRAEFIK_INSTANCE_NAME__

# Static config
cat << 'EOF' > /etc/__TRAEFIK_INSTANCE_NAME__/traefik.toml
__STATIC_CONFIG__
EOF

# Dynamic config
cat << 'EOF' > /etc/__TRAEFIK_INSTANCE_NAME__/dynamic/common.toml
__DYNAMIC_CONFIG__
EOF

chown -R __TRAEFIK_INSTANCE_NAME__:__TRAEFIK_INSTANCE_NAME__ /etc/__TRAEFIK_INSTANCE_NAME__ /etc/__TRAEFIK_INSTANCE_NAME__/dynamic /etc/__TRAEFIK_INSTANCE_NAME__/dynamic/tls /etc/__TRAEFIK_INSTANCE_NAME__/tls /opt/__TRAEFIK_INSTANCE_NAME__

# Systemd service
#
# See https://doc.traefik.io/traefik-enterprise/installing/on-premise/#systemd-linux-only
cat << 'EOF' > /etc/systemd/system/__TRAEFIK_INSTANCE_NAME__.service
[Unit]
Description=__TRAEFIK_INSTANCE_NAME__
After=network-online.target
Wants=network-online.target systemd-networkd-wait-online.service

[Service]
User=__TRAEFIK_INSTANCE_NAME__
Group=__TRAEFIK_INSTANCE_NAME__
ExecStart=/usr/bin/traefik --configFile=/etc/__TRAEFIK_INSTANCE_NAME__/traefik.toml
PrivateTmp=true
PrivateDevices=false
ProtectHome=true
ProtectSystem=full
PermissionsStartOnly=true
NoNewPrivileges=true
LimitNOFILE=32768
AmbientCapabilities=CAP_NET_BIND_SERVICE
Restart=always
RestartSec=2

# Real time service
CPUSchedulingPolicy=fifo
# High CPU priority
CPUSchedulingPriority=85
# Prevent killing from system OOM
OOMScoreAdjust=-900

[Install]
WantedBy=multi-user.target
EOF


# Start and enable the service
systemctl daemon-reload
systemctl enable __TRAEFIK_INSTANCE_NAME__
systemctl start __TRAEFIK_INSTANCE_NAME__


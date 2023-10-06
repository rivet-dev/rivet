if ! id -u "__NAME__" &>/dev/null; then
	useradd -r -s /bin/false __NAME__
fi

for x in /etc/__NAME__ /etc/__NAME__/dynamic /etc/__NAME__/dynamic/tls /etc/__NAME__/tls /opt/__NAME__; do
	mkdir -p $x
	chmod 550 $x
	chown -R __NAME__:__NAME__ $x
done

# Static config
cat << 'EOF' > /etc/__NAME__/traefik.toml
__STATIC_CONFIG__
EOF

# Dynamic config
cat << 'EOF' > /etc/__NAME__/dynamic/common.toml
__DYNAMIC_CONFIG__
EOF

# Systemd service
#
# See https://doc.traefik.io/traefik-enterprise/installing/on-premise/#systemd-linux-only
cat << 'EOF' > /etc/systemd/system/__NAME__.service
[Unit]
Description=__NAME__
After=network-online.target
Wants=network-online.target systemd-networkd-wait-online.service

[Service]
User=__NAME__
Group=__NAME__
ExecStart=/usr/bin/traefik --configFile=/etc/__NAME__/traefik.toml
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

[Install]
WantedBy=multi-user.target
EOF


# Start and enable the service
systemctl enable __NAME__
systemctl start __NAME__


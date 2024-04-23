# Create systemd service file
cat << 'EOF' > /etc/systemd/system/rivet_hook.service
[Unit]
Description=Rivet Hook
Requires=network-online.target __TUNNEL_NAME__.service
After=network-online.target __TUNNEL_NAME__.service
ConditionPathExists=!/var/tmp/rivet_hook.completed

[Service]
User=root
Group=root
Type=oneshot
RemainAfterExit=true
Restart=on-failure
RestartSec=1s
ExecStart=/usr/bin/rivet_hook.sh
ExecStartPost=/bin/touch /var/tmp/rivet_hook.completed

[Install]
WantedBy=multi-user.target
EOF

# Enable initialze script to run on reboot
systemctl daemon-reload
systemctl enable rivet_hook

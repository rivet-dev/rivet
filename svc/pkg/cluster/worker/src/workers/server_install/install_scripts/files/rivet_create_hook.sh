# Create systemd service file
cat << 'EOF' > /etc/systemd/system/rivet_hook.service
[Unit]
Description=Rivet Hook
Requires=network-online.target
After=network-online.target
ConditionPathExists=!/var/tmp/rivet_hook.completed

[Service]
User=root
Group=root
Type=oneshot
ExecStart=/usr/bin/rivet_hook.sh
ExecStartPost=/bin/touch /var/tmp/rivet_hook.completed

[Install]
WantedBy=multi-user.target
EOF

# Enable initialze script to run on reboot
systemctl daemon-reload
systemctl enable rivet_hook

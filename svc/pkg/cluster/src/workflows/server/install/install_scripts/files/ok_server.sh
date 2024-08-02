# Write script
cat << 'EOF' > /usr/bin/ok_server.sh
#!/bin/bash
set -e

trap "exit" INT
while true; do
  { echo -e 'HTTP/1.1 200 OK\r\n\r\n'; } | nc -l -p __OK_SERVER_PORT__ -q 0;
done
EOF

chmod +x /usr/bin/ok_server.sh

# Create systemd service file
cat << 'EOF' > /etc/systemd/system/ok_server.service
[Unit]
Description=Rivet Ok Server
Requires=network-online.target
After=network-online.target

[Service]
User=root
Group=root
Type=oneshot
ExecStart=/usr/bin/ok_server.sh
Type=simple

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable ok_server.service

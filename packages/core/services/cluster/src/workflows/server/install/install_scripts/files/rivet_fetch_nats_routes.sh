# Write script
cat << 'EOF' > /usr/bin/rivet_fetch_nats_routes.sh
#!/usr/bin/env bash
set -eu -o pipefail

# Retry script every 5 seconds until success
echo 'Fetching NATS routes'
while true; do
  response=$(
    curl -f \
      -H "Authorization: Bearer __SERVER_TOKEN__" \
      "__TUNNEL_API_EDGE_API__/provision/datacenters/___DATACENTER_ID___/servers?pools=nats"
  ) && break || sleep 5
done

# Parse the response to get all server LAN IPs
lan_ips=$(echo "$response" | jq -r '[.servers[] | select(.lan_ip != null)] | .[].lan_ip')

# Check if we found any LAN IPs
if [ -z "$lan_ips" ]; then
  echo "Error: No valid NATS server LAN IPs found in the response" >&2
  exit 1
fi

echo "Found NATS server LAN IPs: $lan_ips"

# Generate routes array for NATS config
routes=""
for ip in $lan_ips; do
  # Skip if it's this server's LAN IP
  if [ "$ip" != "___VLAN_IP___" ]; then
    routes="${routes}    nats://__USERNAME__:__PASSWORD__@${ip}:4248\n"
  fi
done

if [ -z "$routes" ]; then
  routes="\n"
fi

# Replace the routes section in the NATS server config
sed -i '/routes = \[/,/\]/c\  routes = [\n'"$routes"'  ]' /etc/nats/server.conf
echo "Updated NATS routes in config"
EOF

chmod +x /usr/bin/rivet_fetch_nats_routes.sh

# Create systemd service file
cat << 'EOF' > /etc/systemd/system/rivet_fetch_nats_routes.service
[Unit]
Description=Fetch NATS Routes
Requires=network-online.target
After=network-online.target

[Service]
User=root
Group=root
Type=oneshot
ExecStart=/usr/bin/rivet_fetch_nats_routes.sh
# Start NATS service after this completes successfully
ExecStartPost=/bin/systemctl start nats.service

# Real time service
CPUSchedulingPolicy=fifo
# High CPU priority
CPUSchedulingPriority=90
# Prevent killing from system OOM
OOMScoreAdjust=-800

[Install]
WantedBy=multi-user.target
EOF

# Create systemd timer file for periodic updates
cat << 'EOF' > /etc/systemd/system/rivet_fetch_nats_routes.timer
[Unit]
Description=Runs NATS routes fetch periodically
Requires=network-online.target
After=network-online.target

[Timer]
# Run immediately on startup
OnBootSec=0
# Trigger every 5 minutes
OnCalendar=*:0/5
# Prevent stampeding herd
RandomizedDelaySec=30
Unit=rivet_fetch_nats_routes.service

[Install]
WantedBy=timers.target
EOF

# Enable and start fetch script
systemctl daemon-reload
systemctl enable rivet_fetch_nats_routes.service
systemctl enable rivet_fetch_nats_routes.timer
systemctl start rivet_fetch_nats_routes.timer
systemctl start --no-block rivet_fetch_nats_routes.service
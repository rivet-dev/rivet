# Write script
cat << 'EOF' > /usr/bin/rivet_fetch_api_route.sh
#!/usr/bin/env bash
set -eu -o pipefail

API_HOSTNAME=rivet-api

# Retry script every 5 seconds until success
echo 'Fetching rivet tls'
while true; do
  response=$(
    curl -f \
      -H "Authorization: Bearer __SERVER_TOKEN__" \
      "__TUNNEL_API_EDGE_API__/provision/datacenters/___DATACENTER_ID___/servers?pools=worker"
  ) && break || sleep 5
done

# Parse the response to get a random server's LAN IP
lan_ip=$(echo "$response" | jq -r '[.servers[] | select(.lan_ip != null)] | .[].lan_ip' | shuf -n 1)

# Check if lan_ip is null
if [ -z "$lan_ip" ]; then
  echo "Error: No valid LAN IP found in the response" >&2
  exit 1
fi

echo "Selected server LAN IP: $lan_ip"

# Remove the old section if it exists
sed -i '/=======/,/=======/d' /etc/hosts

# Update /etc/hosts with the new entry
echo -e "=======\n$lan_ip $API_HOSTNAME\n=======" >> /etc/hosts
EOF

chmod +x /usr/bin/rivet_fetch_api_route.sh

# Create systemd service file
cat << 'EOF' > /etc/systemd/system/rivet_fetch_api_route.service
[Unit]
Description=Rivet API Fetch
Requires=network-online.target
After=network-online.target

[Service]
User=root
Group=root
Type=oneshot
ExecStart=/usr/bin/rivet_fetch_api_route.sh

# Real time service
CPUSchedulingPolicy=fifo
# High CPU priority
CPUSchedulingPriority=90
# Prevent killing from system OOM
OOMScoreAdjust=-800

[Install]
WantedBy=multi-user.target
EOF

# Create systemd timer file
cat << 'EOF' > /etc/systemd/system/rivet_fetch_api_route.timer
[Unit]
Description=Runs API fetch every hour
Requires=network-online.target
After=network-online.target

[Timer]
# Run immediately on startup
OnBootSec=0
# Trigger every hour
OnCalendar=*:0/2
# Prevent stampeding herd
RandomizedDelaySec=60
Unit=rivet_fetch_api_route.service

[Install]
WantedBy=timers.target
EOF

# Enable fetch script
systemctl daemon-reload
systemctl enable rivet_fetch_api_route.timer
systemctl enable rivet_fetch_api_route.service
systemctl start rivet_fetch_api_route.timer
systemctl start --no-block rivet_fetch_api_route.service

# Create dir to hold TLS certs
#
# The guard install script also creates these directories (and chown them),
# but we need the dirs to exist for the rivet_fetch_rg_tls.sh script to run before
# Traefik is installed when using initialize_immediately.
mkdir -p /etc/rivet-server/tls /etc/rivet-server/dynamic/tls

# Write script
cat << 'EOF' > /usr/bin/rivet_fetch_rg_tls.sh
#!/usr/bin/env bash
set -eu -o pipefail

# Retry script every 5 seconds until success
echo 'Fetching rivet tls'
while true; do
  response=$(
    curl -f \
      -H "Authorization: Bearer __SERVER_TOKEN__" \
      "__TUNNEL_API_EDGE_API__/provision/datacenters/___DATACENTER_ID___/tls"
  ) && break || sleep 5
done

echo "TLS received"

# Write tls certs (rename job -> actor)
echo $response | jq -r .job_cert_pem > "/etc/rivet-server/tls/actor_cert.pem"
echo $response | jq -r .job_private_key_pem > "/etc/rivet-server/tls/actor_key.pem"
echo $response | jq -r .api_cert_pem > "/etc/rivet-server/tls/api_cert.pem"
echo $response | jq -r .api_private_key_pem > "/etc/rivet-server/tls/api_key.pem"
EOF

chmod +x /usr/bin/rivet_fetch_rg_tls.sh

# Create systemd service file
cat << 'EOF' > /etc/systemd/system/rivet_fetch_rg_tls.service
[Unit]
Description=Rivet TLS Fetch
Requires=network-online.target
After=network-online.target

[Service]
User=root
Group=root
Type=oneshot
ExecStart=/usr/bin/rivet_fetch_rg_tls.sh

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
cat << 'EOF' > /etc/systemd/system/rivet_fetch_rg_tls.timer
[Unit]
Description=Runs TLS fetch every hour
Requires=network-online.target
After=network-online.target

[Timer]
# Run immediately on startup
OnBootSec=0
# Trigger every hour
OnCalendar=*:0
# Prevent stampeding herd
RandomizedDelaySec=60
Unit=rivet_fetch_rg_tls.service

[Install]
WantedBy=timers.target
EOF

# Enable tls fetch script
systemctl daemon-reload
systemctl enable rivet_fetch_rg_tls.timer
systemctl enable rivet_fetch_rg_tls.service
systemctl start rivet_fetch_rg_tls.timer
systemctl start --no-block rivet_fetch_rg_tls.service

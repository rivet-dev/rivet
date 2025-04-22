PUBLIC_IP=$(ip -4 route get 1.0.0.0 | awk '{print $7; exit}')

# MARK: Rivet server config
cat << 'EOF' > /etc/rivet-server/config.json
__RIVET_EDGE_CONFIG__
EOF

# Systemd service
cat << 'EOF' > /etc/systemd/system/rivet-edge-server.service

[Unit]
Description=Rivet Edge Server
Wants=network-online.target
After=network-online.target
ConditionPathExists=/etc/rivet-server/

[Service]
Environment="RIVET_OTEL_ENABLED=1"
Environment="RIVET_OTEL_ENDPOINT=http://127.0.0.1:__OTEL_PORT__"
Environment="RIVET_SERVICE_NAME=rivet-edge"
Environment="RIVET_CLUSTER_ID=___CLUSTER_ID___"
Environment="RIVET_DATACENTER_ID=___DATACENTER_ID___"
Environment="RIVET_SERVER_ID=___SERVER_ID___"
ExecStart=/usr/local/bin/rivet-edge-server start --skip-provision
Restart=always
RestartSec=2

# Real time service
CPUSchedulingPolicy=fifo
# High CPU priority
CPUSchedulingPriority=90
# Prevent killing from system OOM
OOMScoreAdjust=-1000
# Kill main process, not children
KillMode=process
# Increase limit of file watches
LimitNOFILE=65536
# Increase max process limits
LimitNPROC=infinity
TasksMax=infinity

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable rivet-edge-server
systemctl start rivet-edge-server

PUBLIC_IP=$(ip -4 route get 1.0.0.0 | awk '{print $7; exit}')

# MARK: Rivet server config
cat << 'EOF' > /etc/rivet-server/config.json
__RIVET_GUARD_CONFIG__
EOF

# Systemd service
cat << 'EOF' > /etc/systemd/system/rivet-guard.service

[Unit]
Description=Rivet Guard
Wants=network-online.target
After=network-online.target
ConditionPathExists=/etc/rivet-server/

[Service]
Environment="RIVET_OTEL_ENABLED=1"
Environment="RIVET_OTEL_SAMPLER_RATIO=1"
Environment="RIVET_SERVICE_NAME=guard"
Environment="RIVET_NAMESPACE=__NAMESPACE__"
Environment="RIVET_CLUSTER_ID=___CLUSTER_ID___"
Environment="RIVET_DATACENTER_ID=___DATACENTER_ID___"
Environment="RIVET_SERVER_ID=___SERVER_ID___"
ExecStart=/usr/local/bin/rivet-guard
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
systemctl enable rivet-guard
systemctl start rivet-guard

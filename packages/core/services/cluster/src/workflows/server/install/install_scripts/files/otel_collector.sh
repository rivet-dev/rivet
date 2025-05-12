# Create otelcol user
if ! id -u "otelcol" &>/dev/null; then
	useradd -r -s /bin/false otelcol
fi

# Create required dirs
mkdir -p /etc/otelcol /usr/local/bin /opt/otelcol-__VERSION__

# Download and install otel collector binary
curl -L "https://github.com/open-telemetry/opentelemetry-collector-releases/releases/download/v__VERSION__/otelcol-contrib___VERSION___linux_amd64.tar.gz" -o "/tmp/otelcol-contrib___VERSION___linux_amd64.tar.gz"
tar zxvf "/tmp/otelcol-contrib___VERSION___linux_amd64.tar.gz" -C "/opt/otelcol-__VERSION__"
mv /opt/otelcol-__VERSION__/otelcol-contrib /opt/otelcol-__VERSION__/otelcol
install -o otelcol -g otelcol "/opt/otelcol-__VERSION__/otelcol" /usr/bin/

# Write config
cat << 'EOF' > /etc/otelcol/config.yaml
__CONFIG__
EOF

# Change owner
chown -R otelcol:otelcol /etc/otelcol /etc/otelcol/config.yaml

cat << EOF > /etc/systemd/system/otelcol.service
[Unit]
Description=OTeL Collector
After=network.target

[Service]
User=otelcol
Group=otelcol
ExecStart=/usr/bin/otelcol --config=/etc/otelcol/config.yaml
Restart=always
RestartSec=5

# Medium CPU priority
Nice=-10
# Standard service
CPUSchedulingPolicy=other

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable otelcol
systemctl start otelcol

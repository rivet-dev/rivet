# https://github.com/ncabatoff/process-exporter/releases
version="0.8.7"

if ! id -u "process-exporter" &>/dev/null; then
	useradd -r -s /bin/false process-exporter
fi

# Download and install process-exporter
mkdir -p /opt/process-exporter-$version/ /etc/process-exporter
wget -O /tmp/process-exporter.tar.gz https://github.com/ncabatoff/process-exporter/releases/download/v$version/process-exporter-$version.linux-amd64.tar.gz
tar -zxvf /tmp/process-exporter.tar.gz -C /opt/process-exporter-$version/ --strip-components=1
install -o process-exporter -g process-exporter /opt/process-exporter-$version/process-exporter /usr/bin/

# TODO: Verify hash

# Check version
if [[ "$(process-exporter --version)" != *"$version"* ]]; then
	echo "Version check failed."
	exit 1
fi

# Create config
cat << 'EOF' > /etc/process-exporter/config.yaml
process_names:
  - name: "{{.Comm}}"
    cmdline:
    - '.+'
EOF

# Create systemd service file
cat << 'EOF' > /etc/systemd/system/process-exporter.service
[Unit]
Description=Process Exporter
Requires=network-online.target
After=network-online.target

[Service]
User=process-exporter
Group=process-exporter
Type=simple
ExecStart=/usr/bin/process-exporter --config.path /etc/process-exporter/config.yaml
Restart=always
RestartSec=2

# Medium CPU priority
Nice=-10
# Standard service
CPUSchedulingPolicy=other

[Install]
WantedBy=multi-user.target
EOF

# Start and enable process-exporter service
systemctl daemon-reload
systemctl enable process-exporter
systemctl start process-exporter


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
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317
      http:
        endpoint: 0.0.0.0:4318

processors:
  batch:
    timeout: 5s
    send_batch_size: 10000
  tail_sampling:
    decision_wait: 60s
    num_traces: 50000
    expected_new_traces_per_sec: 10
    decision_cache:
      sampled_cache_size: 1000
      non_sampled_cache_size: 1000
    policies:
      - name: policy-1
        type: status_code
        status_code:
          status_codes:
            - ERROR
      # Guard-only policy
      - name: policy-2
        type: and
        and:
          and_sub_policy:
            - name: latency-policy-1
              type: latency
              latency:
                threshold_ms: 15000
            - name: span-name-policy-1
              type: ottl_condition
              ottl_condition:
                span:
                  - 'name == "routing_fn"'
exporters:
  otlp:
    endpoint: 127.0.0.1:__TUNNEL_OTEL_PORT__
    tls:
      insecure: true

service:
  pipelines:
    logs:
      receivers: [otlp]
      processors: [batch]
      exporters: [otlp]
    traces:
      receivers: [otlp]
      processors: [tail_sampling, batch]
      exporters: [otlp]
    metrics:
      receivers: [otlp]
      processors: [batch]
      exporters: [otlp]
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

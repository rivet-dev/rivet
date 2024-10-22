# Allow container traffic to be routed through IP tables
cat << 'EOF' > /etc/sysctl.d/20-pegboard.conf
net.bridge.bridge-nf-call-arptables = 1
net.bridge.bridge-nf-call-ip6tables = 1
net.bridge.bridge-nf-call-iptables = 1
EOF

sysctl --system

mkdir -p /etc/pegboard
mkdir -p /etc/pegboard/bin

curl -Lf -o /usr/bin/pegboard "__BINARY_URL__"
chmod +x /usr/bin/pegboard

curl -Lf -o /usr/bin/pegboard "__CONTAINER_RUNNER_BINARY_URL__"
chmod +x /usr/bin/pegboard/bin/container-runner

curl -Lf -o /usr/bin/pegboard "__V8_ISOLATE_BINARY_URL__"
chmod +x /usr/bin/pegboard/bin/v8-isolate-runner

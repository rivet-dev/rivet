# Allow container traffic to be routed through IP tables
cat << 'EOF' > /etc/sysctl.d/20-pegboard.conf
net.bridge.bridge-nf-call-arptables = 1
net.bridge.bridge-nf-call-ip6tables = 1
net.bridge.bridge-nf-call-iptables = 1
EOF

sysctl --system

mkdir -p /etc/pegboard
mkdir -p /etc/pegboard/bin

curl -Lf -o /usr/bin/pegboard "__PEGBOARD_MANAGER_BINARY_URL__"
chmod +x /usr/bin/pegboard

curl -Lf -o /etc/pegboard/bin/container-runner "__CONTAINER_RUNNER_BINARY_URL__"
chmod +x /etc/pegboard/bin/container-runner

curl -Lf -o /etc/pegboard/bin/v8-isolate-runner "__V8_ISOLATE_BINARY_URL__"
chmod +x /etc/pegboard/bin/v8-isolate-runner

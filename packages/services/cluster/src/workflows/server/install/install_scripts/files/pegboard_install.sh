# Allow container traffic to be routed through IP tables
cat << 'EOF' > /etc/sysctl.d/20-pegboard.conf
net.bridge.bridge-nf-call-arptables = 1
net.bridge.bridge-nf-call-ip6tables = 1
net.bridge.bridge-nf-call-iptables = 1
EOF

sysctl --system

mkdir -p /etc/rivet-client

curl -Lf -o /usr/local/bin/rivet-client "__PEGBOARD_MANAGER_BINARY_URL__"
chmod +x /usr/local/bin/rivet-client

if [ "__FLAVOR__" = "container" ]; then
	curl -Lf -o /usr/local/bin/rivet-container-runner "__CONTAINER_RUNNER_BINARY_URL__"
	chmod +x /usr/local/bin/rivet-container-runner
fi

if [ "__FLAVOR__" = "isolate" ]; then
	curl -Lf -o /usr/local/bin/rivet-isolate-v8-runner "__V8_ISOLATE_BINARY_URL__"
	chmod +x /usr/local/bin/rivet-isolate-v8-runner
fi

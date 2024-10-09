# Allow container traffic to be routed through IP tables
cat << 'EOF' > /etc/sysctl.d/20-pegboard.conf
net.bridge.bridge-nf-call-arptables = 1
net.bridge.bridge-nf-call-ip6tables = 1
net.bridge.bridge-nf-call-iptables = 1
EOF

sysctl --system

curl -Lf -o /usr/bin/pegboard "__BINARY_URL__"
chmod +x /usr/bin/pegboard

mkdir -p /etc/pegboard

# !!!!!!!!!!!!!!!!!!!!!
#
# DO NOT UPGRADE
#
# This is the last MPL
# licensed version of
# Nomad.
#
# !!!!!!!!!!!!!!!!!!!!!!
version="1.6.0"

# Allow container traffic to be routed through IP tables
#
# See https://developer.hashicorp.com/nomad/docs/install#post-installation-steps
cat << 'EOF' > /etc/sysctl.d/20-nomad.conf
net.bridge.bridge-nf-call-arptables = 1
net.bridge.bridge-nf-call-ip6tables = 1
net.bridge.bridge-nf-call-iptables = 1
EOF

sysctl --system

# Download and unzip nomad
mkdir -p /opt/nomad-$version
curl -L -o /tmp/nomad.zip https://releases.hashicorp.com/nomad/$version/nomad_${version}_linux_amd64.zip
unzip -o /tmp/nomad.zip -d /opt/nomad-$version/

# TODO: Verify hash

# Create symlink in /usr/local/bin
ln -sf /opt/nomad-$version/nomad /usr/local/bin/nomad

# Test nomad installation
if ! nomad version | grep -q "Nomad v$version"; then
  echo "Nomad version mismatch"
  exit 1
fi

# Create admin chain that only accepts traffic from the GG subnet
#
# See Nomad equivalent: https://github.com/hashicorp/nomad/blob/a8f0f2612ef9d283ed903721f8453a0c0c3f51c5/client/allocrunner/networking_bridge_linux.go#L73
ADMIN_CHAIN="RIVET-ADMIN"
SUBNET="172.26.64.0/20"
BRIDGE="nomad"

cat << EOF > /usr/local/bin/setup_nomad_networking.sh
#!/bin/bash
set -euf

if ! iptables -t filter -L $ADMIN_CHAIN &>/dev/null; then
    iptables -t filter -N $ADMIN_CHAIN
    echo "Created iptables chain: $ADMIN_CHAIN"
else
    echo "Chain already exists: $ADMIN_CHAIN"
fi

RULE="-o nomad -d $SUBNET -s __GG_VLAN_SUBNET__ -j ACCEPT"

if ! iptables -C $ADMIN_CHAIN \$RULE &>/dev/null; then
    iptables -A $ADMIN_CHAIN \$RULE
    echo "Added iptables rule: \$RULE"
else
    echo "Rule already exists: \$RULE"
fi
EOF

chmod +x /usr/local/bin/setup_nomad_networking.sh

cat << 'EOF' > /etc/systemd/system/setup_nomad_networking.service
[Unit]
Description=Setup Nomad Networking
After=network.target

[Service]
Type=oneshot
ExecStart=/usr/local/bin/setup_nomad_networking.sh

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable setup_nomad_networking
systemctl start setup_nomad_networking

# Dual-stack CNI config
#
# Modified from default Nomad configuration: https://github.com/hashicorp/nomad/blob/a8f0f2612ef9d283ed903721f8453a0c0c3f51c5/client/allocrunner/networking_bridge_linux.go#L152
cat << EOF > /opt/cni/config/rivet-job.conflist
{
	"cniVersion": "0.4.0",
	"name": "rivet-job",
	"plugins": [
		{
			"type": "loopback"
		},
		{
			"type": "bridge",
			"bridge": "$BRIDGE",
			"ipMasq": true,
			"isGateway": true,
			"forceAddress": true,
			"hairpinMode": false,
			"ipam": {
				"type": "host-local",
				"ranges": [
					[
						{ "subnet": "$SUBNET" }
					],
					[
						{ "subnet": "fd00:db8:2::/64" }
					]
				],
				"routes": [
					{ "dst": "0.0.0.0/0" },
					{ "dst": "::/0" }
				]
			}
		},
		{
			"type": "firewall",
			"backend": "iptables",
			"iptablesAdminChainName": "$ADMIN_CHAIN"
		},
		{
			"type": "portmap",
			"capabilities": { "portMappings": true },
			"snat": true
		}
	]
}
EOF

# Create directories
mkdir -p /opt/nomad/data

mkdir -p /etc/nomad.d

# Copy HCL files
cat << EOF > /etc/nomad.d/common.hcl
region = "global"
datacenter = "__REGION_ID__"
data_dir = "/opt/nomad/data"
name = "__NODE_NAME__"

bind_addr = "__VLAN_IP__"

addresses {
	http = "__VLAN_IP__ 127.0.0.1"
}

telemetry {
	collection_interval = "1s"
	disable_hostname = true
	prometheus_metrics = true
	publish_allocation_metrics = true
	publish_node_metrics = true
}

# Needed for Prometheus rate limiting
limits {
	http_max_conns_per_client = 4096
	rpc_max_conns_per_client = 4096
}
EOF

cat << EOF > /etc/nomad.d/client.hcl
client {
	enabled = true

	node_class = "job"

	# Expose services running on job nodes internally to GG
	network_interface = "__VLAN_IFACE__"

	# See tf/infra/firewall_rules.tf
	min_dynamic_port = 20000
	max_dynamic_port = 25999

	server_join {
		retry_join = [
			__SERVER_JOIN__
		]
		retry_interval = "60s"
		retry_max = 0
	}

	meta {
		# See https://github.com/hashicorp/nomad/issues/9887
		"connect.sidecar_image" = "envoyproxy/envoy:v1.18.3"

		"pool-id" = "job"
	}

	# TODO: This is disabled on job nodes for now because this prevents
	# scheduling full cores at max capacity
	reserved {
		# See tier_list::RESERVE_SYSTEM_CPU
		# cpu = 500
		# See tier_list::RESERVE_SYSTEM_MEMORY
		# memory = 512
		disk = 10000
	}
}

plugin "docker" {
	config {
		extra_labels = ["job_name", "task_group_name", "task_name", "node_name"]
	}
}
EOF

# Systemd service
cat << 'EOF' > /etc/systemd/system/nomad.service
# See https://developer.hashicorp.com/nomad/tutorials/enterprise/production-deployment-guide-vm-with-consul#configure-systemd

[Unit]
Description=Nomad
Wants=network-online.target setup_nomad_networking.service
After=network-online.target setup_nomad_networking.service
ConditionDirectoryNotEmpty=/etc/nomad.d/

[Service]
ExecReload=/bin/kill -HUP $MAINPID
ExecStart=/usr/local/bin/nomad agent -config /etc/nomad.d
KillMode=process
KillSignal=SIGINT
LimitNOFILE=65536
LimitNPROC=infinity
Restart=always
RestartSec=2
TasksMax=infinity
OOMScoreAdjust=-1000

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable nomad
systemctl start nomad


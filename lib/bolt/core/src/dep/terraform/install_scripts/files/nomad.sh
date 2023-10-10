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
chown root:root /etc/sysctl.d/10-rivet.conf
chmod 644 /etc/sysctl.d/10-rivet.conf

sysctl --system

# Download and unzip nomad
mkdir -p /opt/nomad-$version
curl -L -o /tmp/nomad.zip https://releases.hashicorp.com/nomad/$version/nomad_${version}_linux_amd64.zip
unzip -o /tmp/nomad.zip -d /opt/nomad-$version/

# TODO: Verify hash

# Create symlink in /usr/local/bin
ln -sf /opt/nomad-$version/nomad /usr/local/bin/nomad
chown root:root /usr/local/bin/nomad
chmod 755 /usr/local/bin/nomad

# Test nomad installation
if ! nomad version | grep -q "Nomad v$version"; then
  echo "Nomad version mismatch"
  exit 1
fi

# Create directories
mkdir -p /opt/nomad/data
chmod 700 /opt/nomad/data

mkdir -p /opt/vector/data
chmod 700 /opt/vector/data

mkdir -p /etc/nomad.d
chmod 700 /etc/nomad.d

# Copy HCL files
cat << EOF > /etc/nomad.d/common.hcl
region = "global"
datacenter = "__REGION_ID__"
data_dir = "/opt/nomad/data"
name = "__NODE_NAME__"

bind_addr = "__VLAN_ADDR__"

addresses {
	http = "__VLAN_ADDR__ 127.0.0.1"
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

	# Expose services running on job nodes publicly
	network_interface = "__PUBLIC_IFACE__"

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

chmod 640 /etc/nomad.d/*.hcl

# Dual-stack CNI config
#
# Modified from default Nomad configuration: https://developer.hashicorp.com/nomad/docs/networking/cni#nomad-s-bridge-configuration
cat << 'EOF' > /opt/cni/config/rivet-job.conflist
{
	"cniVersion": "0.4.0",
	"name": "rivet-job",
	"plugins": [
		{
			"type": "loopback"
		},
		{
			"type": "bridge",
			"bridge": "nomad",
			"ipMasq": true,
			"isGateway": true,
			"forceAddress": true,
			"hairpinMode": false,
			"ipam": {
				"type": "host-local",
				"ranges": [
					[
						{ "subnet": "172.26.64.0/20" }
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
			"iptablesAdminChainName": "RIVET-ADMIN"
		},
		{
			"type": "portmap",
			"capabilities": {"portMappings": true},
			"snat": true
		}
	]
}
EOF

# Systemd service
cat << 'EOF' > /etc/systemd/system/nomad.service
# See https://developer.hashicorp.com/nomad/tutorials/enterprise/production-deployment-guide-vm-with-consul#configure-systemd

[Unit]
Description=Nomad
Wants=network-online.target
After=network-online.target
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

# Start and enable the service
systemctl enable nomad
systemctl start nomad


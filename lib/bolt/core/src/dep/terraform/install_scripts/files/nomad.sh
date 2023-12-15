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

PUBLIC_IP=$(ip -4 route get 1.0.0.0 | awk '{print $7; exit}')

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
SUBNET_IPV4="172.26.64.0/20"
SUBNET_IPV6="fd00:db8:2::/64"

# !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
#
# DO NOT CHANGE NETWORKING CONFIGS WITHOUT MANUALLY RE-TESTING RULES
#
# There are no automated tests to validate that iptables
# and tc correctly marks traffic priorities. Manually
# check the following if you change this file.
#
# 1. Restart the game node(s) to make sure there are
#    fresh iptables & tc rules.
# 2. Run `tc -s class show dev eth1` and start a lobby.
#    Make sure packets are passing through 1:10 (Game
#    Guard traffic) and 1:20 (ATS traffic).
# 3. Run `iptables -L -v` and validate that packets are
#    flowing through *ALL* rules in the RIVET-ADMIN chain
#    (for game traffic) and the RIVET-INPUT chain (for ATS
#    traffic).
# 4. Run `iptables -L -v -t mangle` and validate that
#    packets are flowing through *ALL* the rules in 
#    RIVET-ADMIN and RIVET-INPUT.
# 5. Obviously, make sure both bridge and host networking
#    works. The lobby connectivity tests cover this.
#
# !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!

# MARK: iptables
cat << EOF > /usr/local/bin/setup_nomad_networking.sh
#!/bin/bash
set -euf

# MARK: Linux Traffic Control
for iface in __PUBLIC_IFACE__ __VLAN_IFACE__; do
    # Check if the HTB qdisc already exists
    if ! tc qdisc show dev \$iface | grep -q "htb 1:"; then

        # Set up a HTB queuing discipline.
		#
		# This will help prioritize traffic in the case of congestion.
		#
		# HTB was chosen over QCB because it allows for more flexibility in the future.
		# 
		# Manually test that traffic is getting routed correctly by running:
		# tc -s class show dev eth1
		#
		# Read more: https://lartc.org/howto/lartc.qdisc.classful.html#AEN1071
        tc qdisc add dev \$iface \
			root \
			handle 1: \
			htb \
			default 10

		# Create a root class with a max bandwidth
		tc class add dev \$iface \
			parent 1: \
			classid 1:1 \
			htb \
			rate 10Gbit

        # Game traffic class with high priority
		#
		# Low bandwidth limit = game servers are not expected to use much bandwidth
		# High priority = packets take priority in the case of congestion
		tc class add dev \$iface \
			parent 1:1 \
			classid 1:10 \
			htb \
			rate 100Mbit \
			prio 0

        # Background traffic class with lower priority
		#
		# High bandwidth = peak performance when there is no network congestion
		# Low priority = packets are dropped first in the case of congestion
		tc class add dev \$iface \
			parent 1:1 \
			classid 1:20 \
			htb \
			rate 1000Mbit \
			prio 1

        # Forward packets with different marks to the appropriate classes.
		#
		# prio x = sets filter priority
		# handle x = handle packets marked x by iptables
		# fw classid x = send matched packets to class x
		# action change dsfield set x = set the packet's TOS (0x10 = low delay, 0x8 = high throughput)
		tc filter add dev \$iface \
			protocol ip \
			parent 1:0 \
			prio 1 \
			handle 1 \
			fw classid 1:10
		tc filter add dev \$iface \
			protocol ip \
			parent 1:0 \
			prio 2 \
			handle 2 \
			fw classid 1:20

        echo "HTB qdisc and class rules added."

    else
        echo "HTB qdisc and class rules already exist."
    fi
done

# MARK: iptables
add_ipt_chain() {
    local ipt="\$1"
    local table="\$2"
    local chain="\$3"

    if ! "\$ipt" -t "\$table" -L "\$chain" &>/dev/null; then
        "\$ipt" -t "\$table" -N "\$chain"
        echo "Created \$ipt \$table chain: \$chain"
    else
        echo "Chain already exists in \$ipt \$table: \$chain"
    fi
}

add_ipt_rule() {
    local ipt="\$1"
    local table="\$2"
    local chain="\$3"
    local rule="\$4"

    if ! "\$ipt" -t \$table -C "\$chain" \$rule &>/dev/null; then
        "\$ipt" -t \$table -A "\$chain" \$rule
        echo "Added \$ipt \$table \$chain rule: \$rule"
    else
        echo "Rule already exists in \$ipt \$table \$chain: \$rule"
    fi
}

for ipt in iptables ip6tables; do
	# Define SUBNET_VAR based on iptables version
	if [ "\$ipt" == "iptables" ]; then
		SUBNET_VAR="$SUBNET_IPV4"
	else
		SUBNET_VAR="$SUBNET_IPV6"
	fi

	# MARK: Chains
	add_ipt_chain "\$ipt" "filter" "$ADMIN_CHAIN"

	add_ipt_chain "\$ipt" "mangle" "RIVET-FORWARD"
	add_ipt_rule "\$ipt" "mangle" "FORWARD" "-j RIVET-FORWARD"

	add_ipt_chain "\$ipt" "filter" "RIVET-INPUT"
	add_ipt_rule "\$ipt" "filter" "INPUT" "-j RIVET-INPUT"

	add_ipt_chain "\$ipt" "mangle" "RIVET-INPUT"
	add_ipt_rule "\$ipt" "mangle" "INPUT" "-j RIVET-INPUT"

	# MARK: Create GG TOS
	#
	# Sets the TOS to minimize delay if not already set.
    if ! "\$ipt" -t mangle -L "RIVET-TOS-GG" &>/dev/null; then
        "\$ipt" -t mangle -N "RIVET-TOS-GG"
        echo "Created \$ipt chain: RIVET-TOS-GG"
    else
        echo "Chain already exists in \$ipt: RIVET-TOS-GG"
    fi
	add_ipt_rule "\$ipt" "mangle" "RIVET-TOS-GG" "-m tos ! --tos 0x0 -j RETURN"
	add_ipt_rule "\$ipt" "mangle" "RIVET-TOS-GG" "-j TOS --set-tos 0x10"

	# VLAN only applicable ot IPv4
	if [ "\$ipt" == "iptables" ]; then
		# MARK: GG TOS
		add_ipt_rule "\$ipt" "mangle" "RIVET-FORWARD" "-s __GG_VLAN_SUBNET__ -d \$SUBNET_VAR -j RIVET-TOS-GG"
		add_ipt_rule "\$ipt" "mangle" "RIVET-FORWARD" "-s \$SUBNET_VAR -d __GG_VLAN_SUBNET__ -j RIVET-TOS-GG"

		# MARK: GG ingress
		# Prioritize traffic
		add_ipt_rule "\$ipt" "filter" "$ADMIN_CHAIN" "-s __GG_VLAN_SUBNET__ -d \$SUBNET_VAR -j MARK --set-mark 1"
		# Accept traffic
		add_ipt_rule "\$ipt" "filter" "$ADMIN_CHAIN" "-s __GG_VLAN_SUBNET__ -d \$SUBNET_VAR -j ACCEPT"

		# MARK: GG egress
		# Prioritize response traffic
		add_ipt_rule "\$ipt" "filter" "$ADMIN_CHAIN" "-s \$SUBNET_VAR -m conntrack --ctstate NEW,ESTABLISHED -j MARK --set-mark 1"
		# Enable conntrack to allow traffic to flow back to the GG subnet
		add_ipt_rule "\$ipt" "filter" "$ADMIN_CHAIN" "-s \$SUBNET_VAR -m conntrack --ctstate NEW,ESTABLISHED -j ACCEPT"

		# MARK: ATS ingress
		# Maximize throughput from ATS
		add_ipt_rule "\$ipt" "mangle" "RIVET-INPUT" "-s __ATS_VLAN_SUBNET__ -j TOS --set-tos Maximize-Throughput"
		# Deprioritize traffic so game traffic takes priority
		add_ipt_rule "\$ipt" "filter" "RIVET-INPUT" "-s __ATS_VLAN_SUBNET__ -j MARK --set-mark 2"
	fi

	# MARK: Public egress
	# Prioritize traffic
	add_ipt_rule "\$ipt" "filter" "$ADMIN_CHAIN" "-s \$SUBNET_VAR -o __PUBLIC_IFACE__ -j MARK --set-mark 1"
    # Allow egress traffic
	add_ipt_rule "\$ipt" "filter" "$ADMIN_CHAIN" "-s \$SUBNET_VAR -o __PUBLIC_IFACE__ -j ACCEPT"

	# MARK: Deny
    # Deny all other egress traffic
	add_ipt_rule "\$ipt" "filter" "$ADMIN_CHAIN" "-s \$SUBNET_VAR -j DROP"
done
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
# We use ptp instead of bridge networking in order to isolate the pod's traffic. It's also more performant than bridge networking.
#
# See default Nomad configuration: https://github.com/hashicorp/nomad/blob/a8f0f2612ef9d283ed903721f8453a0c0c3f51c5/client/allocrunner/networking_bridge_linux.go#L152
cat << EOF > /opt/cni/config/rivet-job.conflist
{
	"cniVersion": "0.4.0",
	"name": "rivet-job",
	"plugins": [
		{
			"type": "loopback"
		},
		{
			"type": "ptp",
			"ipMasq": true,
			"ipam": {
				"type": "host-local",
				"ranges": [
					[
						{ "subnet": "$SUBNET_IPV4" }
					],
					[
						{ "subnet": "$SUBNET_IPV6" }
					]
				],
				"routes": [
					{ "dst": "0.0.0.0/0" },
					{ "dst": "::/0" }
				]
			},
			"dns": {
				"nameservers": [
					"8.8.8.8",
					"8.8.4.4",
					"2001:4860:4860::8888",
					"2001:4860:4860::8844"
				],
				"options": ["rotate", "edns0", "attempts:2"]
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
	collection_interval = "5s"
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

	# Nomad port range for GG-routed traffic.
	#
	# See firewall rules in docs/infrastructure/networking/EDGE_CLUSTER_NETWORKING.md
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
		"network-vlan-ipv4" = "__VLAN_IP__"
		"network-public-ipv4" = "${PUBLIC_IP}"
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

plugin "raw_exec" {
	config {
		enabled = true
		# Disable cgroups because we need to spawn child cgroups without constraints.
		no_cgroups = true
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


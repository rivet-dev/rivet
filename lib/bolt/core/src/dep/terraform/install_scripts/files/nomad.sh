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
SUBNET_IPV4="172.26.64.0/20"
SUBNET_IPV6="fd00:db8:2::/64"

# TODO: Prioritize TOS?  It can be added to the tc filter
#
# https://linux.die.net/man/8/tc-prio#:~:text=TOS%20%20%20%20%20Bits%20%20Means%20%20%20%20%20%20%20%20%20%20%20%20%20%20%20%20%20%20%20%20Linux%20Priority%20%20%20%20Band
# https://man7.org/linux/man-pages/man8/tc-prio.8.html

# Create queuing discipline (qdisc)
for iface in __PUBLIC_IFACE__ __VLAN_IFACE__; do
	# Check if the prio qdisc already exists
	if ! tc qdisc show dev $iface | grep -q "prio 1:"; then

		# Set up an priority queuing discipline.
		#
		# This includes 3 classes: 0 = highest priority, 2 = lowest priority. These classes correspond to flow IDs 1:1, 1:2, and 1:3, representing highest to lowest priority, respectively.
		#
		# 1:2 will be used for game traffic and 1:3 for less important background traffic (i.e. downloading from ATS).
		#
		# Read more about PRIO qdisc: https://lartc.org/howto/lartc.qdisc.classful.html#AEN899:~:text=9.5.3.%20The%20PRIO%20qdisc
		#
		# Manual (less helpful): https://man7.org/linux/man-pages/man8/tc-prio.8.html
		#
		# See all availble flows by running: tc -s class show dev $iface
		tc qdisc add dev $iface root handle 1: prio

		# Forward packets with different marks to the appropriate flows.
		#
		# handle x = handle packets marked x by iptables
		# fw flowid x = send matched packets to the given flow
		tc filter add dev $iface \
			parent 1: \
			protocol ip \
			handle 1 \
			fw flowid 1:1
		tc filter add dev $iface \
			parent 1: \
			protocol ip \
			handle 2 \
			fw flowid 1:2

	    echo "Prio qdisc and filter rules added."
	else
		echo "Prio qdisc and filter rules already exist."
	fi
done

cat << EOF > /usr/local/bin/setup_nomad_networking.sh
#!/bin/bash
set -euf

for ipt in iptables ip6tables; do
	if [ "\$ipt" == "iptables" ]; then
        SUBNET_VAR="$SUBNET_IPV4"
    else
        SUBNET_VAR="$SUBNET_IPV6"
    fi


    if ! \$ipt -t filter -L $ADMIN_CHAIN &>/dev/null; then
        \$ipt -t filter -N $ADMIN_CHAIN
        echo "Created \$ipt chain: $ADMIN_CHAIN"
    else
        echo "Chain already exists in \$ipt: $ADMIN_CHAIN"
    fi

	# MARK: GG ingress
	if [ "\$ipt" == "iptables" ]; then
		# Prioritize traffic
		RULE="-s __GG_VLAN_SUBNET__ -d \$SUBNET_VAR -j MARK --set-mark 1"
		if ! \$ipt -C $ADMIN_CHAIN \$RULE &>/dev/null; then
			\$ipt -A $ADMIN_CHAIN \$RULE
			echo "Added \$ipt rule: \$RULE"
		else
			echo "Rule already exists in \$ipt: \$RULE"
		fi

		# Change TOS to minimize delay
		RULE="-s __GG_VLAN_SUBNET__ -d \$SUBNET_VAR -j TOS --set-tos 0x10"
		if ! \$ipt -C $ADMIN_CHAIN \$RULE &>/dev/null; then
			\$ipt -A $ADMIN_CHAIN \$RULE
			echo "Added \$ipt rule: \$RULE"
		else
			echo "Rule already exists in \$ipt: \$RULE"
		fi

		# Accept traffic
		RULE="-s __GG_VLAN_SUBNET__ -d \$SUBNET_VAR -j ACCEPT"
		if ! \$ipt -C $ADMIN_CHAIN \$RULE &>/dev/null; then
			\$ipt -A $ADMIN_CHAIN \$RULE
			echo "Added \$ipt rule: \$RULE"
		else
			echo "Rule already exists in \$ipt: \$RULE"
		fi
	fi

	# MARK: GG egress
    # Prioritize resopnse traffic
    RULE="-s \$SUBNET_VAR -m conntrack --ctstate NEW,ESTABLISHED -j MARK --set-mark 1"
    if ! \$ipt -C $ADMIN_CHAIN \$RULE &>/dev/null; then
        \$ipt -A $ADMIN_CHAIN \$RULE
        echo "Added \$ipt rule: \$RULE"
    else
        echo "Rule already exists in \$ipt: \$RULE"
    fi

    # Change TOS to minimize delay
    RULE="-s \$SUBNET_VAR -m conntrack --ctstate NEW,ESTABLISHED -j TOS --set-tos 0x10"
    if ! \$ipt -C $ADMIN_CHAIN \$RULE &>/dev/null; then
        \$ipt -A $ADMIN_CHAIN \$RULE
        echo "Added \$ipt rule: \$RULE"
    else
        echo "Rule already exists in \$ipt: \$RULE"
    fi

    # Enable conntrack to allow traffic to flow back to the GG subnet
    RULE="-s \$SUBNET_VAR -m conntrack --ctstate NEW,ESTABLISHED -j ACCEPT"
    if ! \$ipt -C $ADMIN_CHAIN \$RULE &>/dev/null; then
        \$ipt -A $ADMIN_CHAIN \$RULE
        echo "Added \$ipt rule: \$RULE"
    else
        echo "Rule already exists in \$ipt: \$RULE"
    fi

	# MARK: Public egress
	# Prioritize traffic
    RULE="-s \$SUBNET_VAR -o __PUBLIC_IFACE__ -j MARK --set-mark 1"
    if ! \$ipt -C $ADMIN_CHAIN \$RULE &>/dev/null; then
        \$ipt -A $ADMIN_CHAIN \$RULE
        echo "Added \$ipt rule: \$RULE"
    else
        echo "Rule already exists in \$ipt: \$RULE"
    fi

	# Change TOS to minimize delay
    RULE="-s \$SUBNET_VAR -o __PUBLIC_IFACE__ -j TOS --set-tos 0x10"
	if ! \$ipt -C $ADMIN_CHAIN \$RULE &>/dev/null; then
		\$ipt -A $ADMIN_CHAIN \$RULE
		echo "Added \$ipt rule: \$RULE"
	else
		echo "Rule already exists in \$ipt: \$RULE"
	fi

    # Allow egress traffic
    RULE="-s \$SUBNET_VAR -o __PUBLIC_IFACE__ -j ACCEPT"
    if ! \$ipt -C $ADMIN_CHAIN \$RULE &>/dev/null; then
        \$ipt -A $ADMIN_CHAIN \$RULE
        echo "Added \$ipt rule: \$RULE"
    else
        echo "Rule already exists in \$ipt: \$RULE"
    fi

	# MARK: ATS ingress
	if [ "\$ipt" == "iptables" ]; then
		# Deprioritize traffic so game traffic takes priority
		RULE="-s __ATS_VLAN_SUBNET__ -j MARK --set-mark 2"
		if ! \$ipt -C $ADMIN_CHAIN \$RULE &>/dev/null; then
			\$ipt -A $ADMIN_CHAIN \$RULE
			echo "Added \$ipt rule: \$RULE"
		else
			echo "Rule already exists in \$ipt: \$RULE"
		fi

		# Change TOS to maximize throughput
		RULE="-s \$SUBNET_VAR -o __PUBLIC_IFACE__ -j TOS --set-tos 0x8"
		if ! \$ipt -C $ADMIN_CHAIN \$RULE &>/dev/null; then
			\$ipt -A $ADMIN_CHAIN \$RULE
			echo "Added \$ipt rule: \$RULE"
		else
			echo "Rule already exists in \$ipt: \$RULE"
		fi
	fi

	# MARK: Deny
    # Deny all other egress traffic
    RULE="-s \$SUBNET_VAR -j DROP"
    if ! \$ipt -C $ADMIN_CHAIN \$RULE &>/dev/null; then
        \$ipt -A $ADMIN_CHAIN \$RULE
        echo "Added \$ipt rule: \$RULE"
    else
        echo "Rule already exists in \$ipt: \$RULE"
    fi
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

# AppArmor config
#
# See default Docker config: https://github.com/moby/moby/blob/777e9f271095685543f30df0ff7a12397676f938/profiles/apparmor/template.go
cat << EOF > /etc/apparmor.d/rivet-job
#include <tunables/global>
#@{PROC}=/proc/

profile rivet-job flags=(attach_disconnected,mediate_deleted) {
  #include <abstractions/base>

  network,
  capability,
  file,
  umount,
  # Host privilaged processes may send signals to container processes. (i.e. Nomad)
  signal (receive) peer=unconfined,

  deny @{PROC}/* w,   # deny write for all files directly in /proc (not in a subdir)
  # deny write to files not in /proc/<number>/** or /proc/sys/**
  deny @{PROC}/{[^1-9],[^1-9][^0-9],[^1-9s][^0-9y][^0-9s],[^1-9][^0-9][^0-9][^0-9/]*}/** w,
  deny @{PROC}/sys/[^k]** w,  # deny /proc/sys except /proc/sys/k* (effectively /proc/sys/kernel)
  deny @{PROC}/sys/kernel/{?,??,[^s][^h][^m]**} w,  # deny everything except shm* in /proc/sys/kernel/
  deny @{PROC}/sysrq-trigger rwklx,
  deny @{PROC}/kcore rwklx,

  deny mount,

  deny /sys/[^f]*/** wklx,
  deny /sys/f[^s]*/** wklx,
  deny /sys/fs/[^c]*/** wklx,
  deny /sys/fs/c[^g]*/** wklx,
  deny /sys/fs/cg[^r]*/** wklx,
  deny /sys/firmware/** rwklx,
  deny /sys/kernel/security/** rwklx,

  # suppress ptrace denials when using 'docker ps' or using 'ps' inside a container
  ptrace (trace,read,tracedby,readby) peer=rivet-job,
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


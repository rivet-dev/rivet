cat << 'EOF' > /etc/sysctl.d/10-rivet.conf
# MARK: Max user watchers
fs.inotify.max_user_watches = 524288
fs.inotify.max_user_instances = 65536

# MARK: Networking balancer tuning, see https://gist.github.com/pensierinmusica/6206493 & https://doc.traefik.io/traefik/v1.4/benchmarks/
# Avoid a smurf attack
net.ipv4.icmp_echo_ignore_broadcasts = 1

# Turn on protection for bad icmp error messages
net.ipv4.icmp_ignore_bogus_error_responses = 1

# Turn on syncookies for SYN flood attack protection
net.ipv4.tcp_syncookies = 1

# Turn on and log spoofed, source routed, and redirect packets
net.ipv4.conf.all.log_martians = 1
net.ipv4.conf.default.log_martians = 1

# No source routed packets here
net.ipv4.conf.all.accept_source_route = 0
net.ipv4.conf.default.accept_source_route = 0

# Turn on reverse path filtering
net.ipv4.conf.all.rp_filter = 1
net.ipv4.conf.default.rp_filter = 1

# Make sure no one can alter the routing tables
# net.ipv4.conf.all.accept_redirects = 0
# net.ipv4.conf.default.accept_redirects = 0
# net.ipv4.conf.all.secure_redirects = 0
# net.ipv4.conf.default.secure_redirects = 0

# Don't act as a router
net.ipv4.ip_forward = 0
net.ipv4.conf.all.send_redirects = 0
net.ipv4.conf.default.send_redirects = 0

# Turn on execshild
# kernel.exec-shield = 1
# kernel.randomize_va_space = 1

# Tune IPv6
net.ipv6.conf.default.router_solicitations = 0
net.ipv6.conf.default.accept_ra_rtr_pref = 0
net.ipv6.conf.default.accept_ra_pinfo = 0
net.ipv6.conf.default.accept_ra_defrtr = 0
net.ipv6.conf.default.autoconf = 0
net.ipv6.conf.default.dad_transmits = 0
net.ipv6.conf.default.max_addresses = 1

# Optimization for port usefor LBs
# Increase system file descriptor limit
fs.file-max = 65535

# Allow for more PIDs (to reduce rollover problems); may break some programs 32768
kernel.pid_max = 4194304

# Increase system IP port limits
net.ipv4.ip_local_port_range = 2000 65000

# Increase TCP max buffer size setable using setsockopt()
net.ipv4.tcp_rmem = 4096 87380 16777216
net.ipv4.tcp_wmem = 4096 87380 16777216

# Increase Linux auto tuning TCP buffer limits
# min, default, and max number of bytes to use
# set max to at least 4MB, or higher if you use very high BDP paths
# Tcp Windows etc
net.core.rmem_max = 16777216
net.core.wmem_max = 16777216
net.core.netdev_max_backlog = 5000
net.ipv4.tcp_window_scaling = 1

# When the server has to cycle through a high volume of TCP connections,
# it can build up a large number of connections in TIME_WAIT state.
# TIME_WAIT means a connection is closed but the allocated
# resources are yet to be released. Setting this directive to 1
# will tell the kernel to try to recycle the allocation
# for a new connection when safe to do so.
# This is cheaper than setting up a new connection from scratch.
net.ipv4.tcp_tw_reuse = 1

# The minimum number of seconds that must elapse before
# a connection in TIME_WAIT state can be recycled.
# Lowering this value will mean allocations will be recycled faster.
net.ipv4.tcp_fin_timeout = 15

# Other tunings
net.core.somaxconn = 4096
net.ipv4.tcp_max_syn_backlog = 20480
net.ipv4.tcp_max_tw_buckets = 400000
net.ipv4.tcp_no_metrics_save = 1
net.ipv4.tcp_syn_retries = 2
net.ipv4.tcp_synack_retries = 2
vm.min_free_kbytes = 65536

# Increase conntrack
net.netfilter.nf_conntrack_max=262144
EOF

# Reload settings
sysctl --system


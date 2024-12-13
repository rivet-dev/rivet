# Append config
cat << 'EOF' > /etc/foundationdb/foundationdb.conf
## foundationdb.conf
##
## Configuration file for FoundationDB server processes
## Full documentation is available at
## https://apple.github.io/foundationdb/configuration.html#the-configuration-file

[fdbmonitor]
user = foundationdb
group = foundationdb

[general]
restart-delay = 60
## by default, restart-backoff = restart-delay-reset-interval = restart-delay
# initial-restart-delay = 0
# restart-backoff = 60
# restart-delay-reset-interval = 60
cluster-file = /etc/foundationdb/fdb.cluster
# delete-envvars =
# kill-on-configuration-change = true
 
## Default parameters for individual fdbserver processes
[fdbserver]
command = /usr/sbin/fdbserver
public-address = ___VLAN_IP___:$ID
listen-address = 0.0.0.0:$ID
datadir = /var/lib/foundationdb/data/$ID
logdir = /var/log/foundationdb
machine-id = ___SERVER_ID___
datacenter-id = ___DATACENTER_ID___
# logsize = 10MiB
# maxlogssize = 100MiB
# class = 
# memory = 8GiB
# storage-memory = 1GiB
# cache-memory = 2GiB
# metrics-cluster =
# metrics-prefix =
# TODO: TLS

# Individual servers
# 
# Each process requires 4 GB of RAM.
[fdbserver.4500]
 
[backup_agent]
command = /usr/lib/foundationdb/backup_agent/backup_agent
logdir = /var/log/foundationdb

[backup_agent.1]

EOF

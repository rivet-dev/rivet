#!/bin/bash

function configure_database() {
	echo "Configuring database..."
	until fdbcli -C /data/foundationdb/fdb.cluster --exec 'configure new single ssd' --timeout 10; do
		sleep 2
	done
	echo "Database configured."
}

# Background process will wait until FoundationDB is up and configure it.
if [ ! -e /var/fdb/fdb.cluster ]; then
	configure_database &
else
	echo "Database already configured."
fi

# Start FDB
echo 'fdb:fdb@127.0.0.1:4500' > /data/foundationdb/fdb.cluster
/usr/lib/foundationdb/fdbmonitor --conffile /etc/foundationdb/foundationdb.conf --lockfile /var/run/fdbmonitor.pid


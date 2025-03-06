#!/bin/bash

function configure_database() {
	echo "Configuring database..."
	until fdbcli --exec 'configure new single ssd' --timeout 10; do
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

# This will automatically populate the file contents with `docker:docker@$PUBLIC_IP:$FDB_PORT`
export FDB_NETWORKING_MODE=container
exec /var/fdb/scripts/fdb.bash "$@"

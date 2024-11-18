#!/bin/sh
exec > /var/log/clickhouse-health.log 2>&1

start_time=$(date +%s%3N)
         while ! (echo 'Running health check'; /etc/s6-overlay/scripts/clickhouse-healthcheck.sh); do
	echo 'Health check failed'
             sleep 0.25
         done

end_time=$(date +%s%3N)
elapsed_time=$((end_time - start_time))
echo "Health check passed in ${elapsed_time}ms."

         exit 0
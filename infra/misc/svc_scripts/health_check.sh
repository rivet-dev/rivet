#!/bin/sh
set -uf

# Log to file
exec >> "/var/log/health-checks.txt" 2>&1

URL_PATH="$1"
endpoint="127.0.0.1:8000${URL_PATH}"
echo "Checking $endpoint"
curl --fail --max-time 10 -v "$endpoint"
EXIT_STATUS=$?
if [ $EXIT_STATUS -ne 0 ]; then
	echo "Health server essential check failed"
	exit $EXIT_STATUS
fi

echo Ok

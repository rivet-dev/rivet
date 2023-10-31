set -uf

# Log to file
exec >> "/var/log/health-checks.txt" 2>&1

endpoint='127.0.0.1:8000/health/liveness'
echo "Checking $endpoint"
curl --fail -v "$endpoint"
EXIT_STATUS=$?
if [ $EXIT_STATUS -ne 0 ]; then
	echo "Health server liveness check failed"
	exit $EXIT_STATUS
fi

echo Ok

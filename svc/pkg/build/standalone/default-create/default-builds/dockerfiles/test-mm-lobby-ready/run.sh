#!/bin/sh
set -euf

echo
echo "Env:"
env

echo
echo "resolv.conf"
cat /etc/resolv.conf

READY_URL="$RIVET_API_URL/matchmaker/lobbies/ready"
echo "Sending ready to $READY_URL"
curl --verbose --fail --insecure --request POST --header "Content-Type: application/json" --header "Authorization: Bearer $RIVET_TOKEN" --data "{}" "$READY_URL"

echo "Success, waiting indefinitely"
tail -f /dev/null


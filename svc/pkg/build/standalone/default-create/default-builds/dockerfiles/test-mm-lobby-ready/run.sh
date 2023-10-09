#!/bin/sh
set -euf

echo
echo "Env:"
env

echo
echo "resolv.conf"
cat /etc/resolv.conf

parsed_host=$(echo $RIVET_API_URL | awk -F/ '{print $3}' | awk -F: '{print $1}')
parsed_port=$(echo $RIVET_API_URL | awk -F/ '{print $3}' | awk -F: '{print $2}')
scheme=$(echo $RIVET_API_URL | awk -F/ '{print $1}' | awk -F: '{print $1}')

if [ -z "$parsed_port" ]; then
  if [ "$scheme" = "https" ]; then
    parsed_port=443
  elif [ "$scheme" = "http" ]; then
    parsed_port=80
  fi
fi

echo "Looking up $parsed_host"
dig $parsed_host
hostname_resolved=$(dig +short $parsed_host | head -n 1)

echo "Pinging $hostname_resolved:$parsed_port"
nc -z -v -w5 $hostname_resolved $parsed_port || exit 1

READY_URL="$RIVET_API_URL/matchmaker/lobbies/ready"
echo "Sending ready to $READY_URL"
curl --verbose --fail --insecure --request POST --header "Content-Type: application/json" --header "Authorization: Bearer $RIVET_TOKEN" --data "{}" "$READY_URL"

echo "Success, waiting indefinitely"
tail -f /dev/null


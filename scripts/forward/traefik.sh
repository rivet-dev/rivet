#!/bin/sh
set -euf

echo 'Dashboard: http://localhost:9090/dashboard/'
echo 'API: http://localhost:9090/api/'

FORWARD_NS=traefik FORWARD_NAME=service/traefik-headless FORWARD_PORT=9000 ./scripts/forward/service.sh


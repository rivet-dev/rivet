#!/bin/sh
set -euf

# We don't know the value of $PORT yet
echo 'Dashboard: http://localhost:$PORT/dashboard/'
echo 'API: http://localhost:$PORT/api/'

FORWARD_NS=traefik FORWARD_NAME=service/traefik-headless FORWARD_PORT=9000 ./scripts/forward/service.sh


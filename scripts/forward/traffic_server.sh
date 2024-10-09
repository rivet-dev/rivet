#!/bin/sh
set -euf

FORWARD_NS=traffic-server FORWARD_NAME=service/traffic-server FORWARD_PORT=8080 ./scripts/forward/service.sh

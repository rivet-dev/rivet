#!/bin/sh
set -euf

FORWARD_NS=rivet-service FORWARD_NAME=service/rivet-api-public PORT=${PORT:-6669} FORWARD_PORT=8002 ./scripts/forward/service.sh

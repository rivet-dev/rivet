#!/bin/sh
set -euf

FORWARD_NS=vector FORWARD_NAME=service/vector PORT=${PORT:-8686} FORWARD_PORT=8686 ./scripts/forward/service.sh

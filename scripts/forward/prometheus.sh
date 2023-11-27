#!/bin/sh
set -euf

FORWARD_NS=prometheus FORWARD_NAME=service/prometheus-operated FORWARD_PORT=9090 ./scripts/forward/service.sh


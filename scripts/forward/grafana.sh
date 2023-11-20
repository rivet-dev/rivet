#!/bin/sh
set -euf

FORWARD_NS=prometheus FORWARD_NAME=service/prometheus-grafana FORWARD_PORT=80 ./scripts/forward/service.sh

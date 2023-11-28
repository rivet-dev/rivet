#!/bin/sh
set -euf

FORWARD_NS=prometheus FORWARD_NAME=service/alertmanager-operated FORWARD_PORT=9093 ./scripts/forward/service.sh

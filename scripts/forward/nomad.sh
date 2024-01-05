#!/bin/sh
set -euf

FORWARD_NS=nomad FORWARD_NAME=service/nomad-server FORWARD_PORT=4646 ./scripts/forward/service.sh


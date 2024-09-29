#!/bin/sh
set -euf

FORWARD_NS=cockroachdb FORWARD_NAME=service/cockroachdb PORT=5432 FORWARD_PORT=26257 ./scripts/forward/service.sh


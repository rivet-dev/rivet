#!/usr/bin/env bash
set -euo pipefail

CONTAINER_NAME="rivet-engine-postgres"

echo "Nuking postgres container and data..."

# Get the volume name before removing the container
VOLUME_NAME=""
if docker ps --all --format '{{.Names}}' | grep -qw "${CONTAINER_NAME}"; then
	VOLUME_NAME=$(docker inspect "${CONTAINER_NAME}" --format '{{range .Mounts}}{{if eq .Destination "/var/lib/postgresql/data"}}{{.Name}}{{end}}{{end}}' 2>/dev/null || true)
fi

# Stop and remove container
if docker ps --all --format '{{.Names}}' | grep -qw "${CONTAINER_NAME}"; then
	echo "Stopping and removing container '${CONTAINER_NAME}'..."
	if docker ps --format '{{.Names}}' | grep -qw "${CONTAINER_NAME}"; then
		docker stop "${CONTAINER_NAME}" >/dev/null 2>&1 || true
	fi
	docker rm "${CONTAINER_NAME}" >/dev/null 2>&1 || true
	echo "Container removed."
else
	echo "Container '${CONTAINER_NAME}' not found."
fi

# Remove volume if it exists
if [ -n "${VOLUME_NAME}" ]; then
	echo "Removing volume '${VOLUME_NAME}'..."
	docker volume rm "${VOLUME_NAME}" >/dev/null 2>&1 || true
	echo "Volume removed."
fi

echo "Postgres nuked successfully!"

#!/usr/bin/env bash
set -euo pipefail

CONTAINER_NAME="rivet-engine-postgres"

if [ $# -ne 1 ]; then
	echo "Usage: $0 <backup-path>"
	echo "Example: $0 /path/to/backup/postgres-backup.tar.gz"
	exit 1
fi

BACKUP_PATH="$1"

# Check if container exists
if ! docker ps --all --format '{{.Names}}' | grep -qw "${CONTAINER_NAME}"; then
	echo "error: container '${CONTAINER_NAME}' not found"
	exit 1
fi

# Get the volume name
VOLUME_NAME=$(docker inspect "${CONTAINER_NAME}" --format '{{range .Mounts}}{{if eq .Destination "/var/lib/postgresql/data"}}{{.Name}}{{end}}{{end}}')

if [ -z "${VOLUME_NAME}" ]; then
	echo "error: could not find postgres data volume for container '${CONTAINER_NAME}'"
	exit 1
fi

echo "Backing up postgres data from volume '${VOLUME_NAME}'..."

# Create backup directory if it doesn't exist
BACKUP_DIR="$(dirname "${BACKUP_PATH}")"
mkdir -p "${BACKUP_DIR}"

# Backup the volume data
docker run --rm \
	-v "${VOLUME_NAME}":/data \
	-v "${BACKUP_DIR}":/backup \
	alpine \
	tar czf "/backup/$(basename "${BACKUP_PATH}")" -C /data .

echo "Backup completed: ${BACKUP_PATH}"

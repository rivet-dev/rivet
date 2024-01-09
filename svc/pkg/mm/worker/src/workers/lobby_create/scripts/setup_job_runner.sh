#!/usr/bin/env bash
set -euf -o pipefail

log() {
    local timestamp=$(date +"%Y-%m-%d %H:%M:%S.%3N")
    echo "[$timestamp] [setup_job_runner] $@"
}

# Download job runner binary
curl -Lf "$NOMAD_META_job_runner_binary_url" -o "${NOMAD_ALLOC_DIR}/job-runner"
chmod +x "${NOMAD_ALLOC_DIR}/job-runner"
log "Finished downloading job-runner"


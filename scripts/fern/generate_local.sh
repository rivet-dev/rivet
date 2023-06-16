#!/usr/bin/env bash
set -euf -o pipefail

fern generate --group local_internal --log-level debug
fern generate --group local_external --log-level debug


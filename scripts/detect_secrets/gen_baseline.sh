#!/bin/sh
set -euf

detect-secrets scan --baseline gen/secrets.baseline.json --exclude-files '^gen/secrets.baseline.json$' --exclude-files '^target/' --no-verify


#!/bin/sh
set -euf

git ls-files -z | xargs -0 detect-secrets-hook --baseline gen/secrets.baseline.json


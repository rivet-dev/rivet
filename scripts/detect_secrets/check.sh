#!/bin/sh
set -euf

# Skip files that take a long time and are unlikely to include secrets
git ls-files -z | grep -zvE '^(sdks/|lib/smithy-output/|infra/helm/|infra/tf/k8s_infra/grafana_dashboards/)' | xargs -0 detect-secrets-hook --baseline gen/secrets.baseline.json


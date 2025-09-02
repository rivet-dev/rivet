#!/usr/bin/env bash
set -euf -o pipefail

# Generate Fern libraries
#
# TMPDIR required because of issue with nix-shell
(cd sdks/api && TMPDIR=/tmp FERN_NO_VERSION_REDIRECTION=true FERN_DISABLE_TELEMETRY=true npx -p fern-api@0.44.11 fern generate --local --group $FERN_GROUP --log-level debug)

# Add missing deps
(cd sdks/typescript/api-$FERN_GROUP && jq ".devDependencies[\"@types/node-fetch\"] = \"2.6.11\"" package.json > package.json.tmp && mv package.json.tmp package.json)

# Generate OpenAPI clients
./scripts/openapi/gen_rust.ts &

wait

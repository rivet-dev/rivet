#!/usr/bin/env bash
set -euf -o pipefail

# Generate Fern libraries
(cd sdks && FERN_NO_VERSION_REDIRECTION=true FERN_DISABLE_TELEMETRY=true npx -p fern-api@0.44.11 fern generate --local --group $FERN_GROUP --log-level debug)

# Add missing deps
(cd sdks/$FERN_GROUP/typescript && nix-shell -p jq --run 'jq ".devDependencies[\"@types/node-fetch\"] = \"2.6.11\"" package.json > package.json.tmp && mv package.json.tmp package.json')

# Build libraries
#
# See https://github.com/fern-api/fern-typescript/blob/3b1c33781bbd726cee26a21c1ff3464eeae70cad/README.md?plain=1#L379
(cd sdks/$FERN_GROUP/typescript && yarn install && yarn pack -f archive.tgz) &

# Generate OpenAPI clients
(./scripts/openapi/gen_spec_compat.sh && ./scripts/openapi/gen_rust.sh) &

wait


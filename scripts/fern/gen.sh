#!/usr/bin/env bash
set -euf -o pipefail

FERN_GROUP=runtime ./scripts/fern/gen_inner.sh &
FERN_GROUP=full ./scripts/fern/gen_inner.sh &
wait

# Pack API for frontend
# (cd sdks/api/full/typescript && yarn install && yarn pack --out ../../../../frontend/apps/hub/vendor/rivet-gg-api.tgz)

# Generate APIs
# ./site/scripts/generateApi.js

# Update lockfile
pnpm install


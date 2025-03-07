#!/usr/bin/env bash
set -euf -o pipefail

FERN_GROUP=runtime ./scripts/fern/gen_inner.sh &
FERN_GROUP=full ./scripts/fern/gen_inner.sh &
wait

# Pack API for frontend
# HACK: Have to move node_modules since the prepack script looks in the wrong node_modules folder
mv node_modules node_modules.tmp
(cd sdks/api/full/typescript && yarn install && yarn pack --out ../../../../frontend/apps/hub/vendor/rivet-gg-api.tgz)
mv node_modules.tmp node_modules

# Generate APIs
./site/scripts/generateApi.js

# Update lockfile
yarn


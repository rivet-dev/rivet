#!/usr/bin/env bash
set -euf -o pipefail

set +u
if [ -z "$FERN_REPO_PATH" ]; then
	echo 'Please clone and build https://github.com/rivet-gg/fern/tree/nathan/skip-token-local and set $FERN_REPO_PATH. This is a workaround until https://github.com/fern-api/fern/pull/2551 is resolved.'
	exit 1
fi
set -u

# Generate Fern libraries
echo "Using Fern from $FERN_REPO_PATH"
FERN_NO_VERSION_REDIRECTION=true node "$FERN_REPO_PATH/packages/cli/cli/dist/dev/cli.cjs" generate --local --group local --log-level debug

# Once the above changes are merged
# Generate Fern libraries
# set +u
# if [ -n "$FERN_REPO_PATH" ]; then
# 	echo "Using Fern from $FERN_REPO_PATH"
# 	FERN_NO_VERSION_REDIRECTION=true node "$FERN_REPO_PATH/packages/cli/cli/dist/dev/cli.cjs" generate --local --group local --log-level debug
# else
# 	fern generate --local --group local --log-level debug
# fi
# set -u

# Export missing types
cat <<EOF >> sdks/typescript/src/index.ts
export * as core from "./core";
export * as apiResponse from "./core/fetcher/APIResponse";
export * as fetcher from "./core/fetcher";
export * as serialization from "./serialization";
EOF

# Build libraries
#
# See https://github.com/fern-api/fern-typescript/blob/3b1c33781bbd726cee26a21c1ff3464eeae70cad/README.md?plain=1#L379
(cd sdks/typescript && yarn install && yarn build) &

# Generate OpenAPI clients
(./scripts/openapi/gen_spec_compat.sh && ./scripts/openapi/gen_rust.sh) &

wait


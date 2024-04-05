#!/usr/bin/env bash
set -euf -o pipefail

SDKS_PATH="sdks/$FERN_GROUP"
GEN_PATH_OPENAPI="$SDKS_PATH/openapi_compat/openapi.yml"
GEN_PATH_RUST="$SDKS_PATH/rust"
GEN_PATH_RUST_CLI="$SDKS_PATH/rust-cli"

rm -rf $GEN_PATH_RUST
rm -rf $GEN_PATH_RUST_CLI

docker run --rm \
	-u $(id -u):$(id -g) \
	-v "$(pwd):/data" openapitools/openapi-generator-cli:v6.4.0 generate \
	-i "/data/$GEN_PATH_OPENAPI" \
	--additional-properties=removeEnumValuePrefix=false \
	-g rust \
	-o "/data/$GEN_PATH_RUST" \
	-p packageName=rivet-api

# Fix OpenAPI bug (https://github.com/OpenAPITools/openapi-generator/issues/14171)
sed -i 's/CloudGamesLogStream/crate::models::CloudGamesLogStream/' "$GEN_PATH_RUST/src/apis/cloud_games_matchmaker_api.rs"
sed -i 's/PortalNotificationUnregisterService/crate::models::PortalNotificationUnregisterService/' "$GEN_PATH_RUST/src/apis/portal_notifications_api.rs"
sed -i 's/AdminPoolType/crate::models::AdminPoolType/' "$GEN_PATH_RUST/src/apis/admin_cluster_api.rs"

if [ "$FERN_GROUP" == "full" ]; then
	# Create variant specifically for the CLI
	cp -r $GEN_PATH_RUST $GEN_PATH_RUST_CLI
	sed -i 's/rivet-api/rivet-api-cli/' "$GEN_PATH_RUST_CLI/Cargo.toml"
	# HACK: Modify libraries to disallow unknown fields in config
	find $GEN_PATH_RUST_CLI -name "cloud_version_*.rs" -exec sed -i 's/\(#\[derive.*Deserialize.*\]\)/\1\n#[serde(deny_unknown_fields)]/g' {} \;
fi

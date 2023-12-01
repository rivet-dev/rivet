#!/usr/bin/env bash
set -euf -o pipefail

rm -rf gen/openapi/internal/rust/
docker run --rm \
	-u $(id -u):$(id -g) \
	-v "$(pwd):/data" openapitools/openapi-generator-cli:v6.4.0 generate \
	-i /data/gen/openapi/internal/spec_compat/openapi.yml \
	--additional-properties=removeEnumValuePrefix=false \
	-g rust \
	-o /data/gen/openapi/internal/rust \
	-p packageName=rivet-api

# Fix openapi bug (https://github.com/OpenAPITools/openapi-generator/issues/14171)
fix_file_path="gen/openapi/internal/rust/src/apis/cloud_games_matchmaker_api.rs"
sed -i 's/CloudGamesLogStream/crate::models::CloudGamesLogStream/' "$fix_file_path"
fix_file_path="gen/openapi/internal/rust/src/apis/portal_notifications_api.rs"
sed -i 's/PortalNotificationUnregisterService/crate::models::PortalNotificationUnregisterService/' "$fix_file_path"

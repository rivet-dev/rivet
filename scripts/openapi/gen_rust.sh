#!/usr/bin/env bash
set -euf -o pipefail

rm -rf gen/openapi/internal/rust/
docker run --rm \
	-u $(id -u):$(id -g) \
	-v "$(pwd):/data" openapitools/openapi-generator-cli:v6.4.0 generate \
	-i /data/gen/openapi/internal/spec_compat/openapi.yml \
	-g rust \
	-o /data/gen/openapi/internal/rust \
	-p packageName=rivet-api

# Generating SDKs

To make sure the openapi spec is up to date, run `cargo check -p rivet-dump-openapi`

Use `./scripts/fern/gen.sh` to generate the SDKs, which will be outputted under `sdks/api`

# Endpoints

A current list of API endpoints can be found at `packages/core/api-public/src/router.rs` or in the `out/openapi.json` file. 

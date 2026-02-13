# API

## Packages

- `packages/common/api-types`:
    - `src/{}/mod.rs`: Common API types shared between multiple packages.
    - `src/{}/{}.rs`: Request types used in api-peer (e.g. `get`, `list`). This is not in `rivet-api-peer` because these types must be used with `rivet-api-client` in packaes that don't depend ont `rivet-api-peer` (often because of cirular dependency issues, e.g. `rivet-api-peer` -> `pegboard` -> `rivet-api-peer`)
- `packages/common/api-client`: Helper utilities for making requests to `api-peer`
- `packages/common/api-builder`: Helper utilities for building API servers with Axum
- `packages/services/api-peer`: API endpoint used for peer-to-peer communication across datacenters, only used by internal Rivet packages
- `packages/services/api-public`: API endpoints used by clients calling Rivet APIs

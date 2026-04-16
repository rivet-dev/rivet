# SQLite Metrics

- `sqlite_commit_phase_duration_seconds{phase,path}`: Engine-side histogram for commit request phases. `path` is `fast` or `slow`. `phase` is `decode_request`, `meta_read`, `ltx_encode`, `pidx_read`, `udb_write`, or `response_build`.
- `sqlite_commit_stage_phase_duration_seconds{phase}`: Engine-side histogram for staged commit uploads. `phase` is `decode`, `stage_encode`, or `udb_write`.
- `sqlite_commit_finalize_phase_duration_seconds{phase}`: Engine-side histogram for staged commit finalize work. `phase` is `stage_promote`, `pidx_write`, or `meta_write`.
- `sqlite_commit_dirty_page_count{path}`: Histogram of dirty page counts per commit path.
- `sqlite_commit_dirty_bytes{path}`: Histogram of raw dirty-page bytes per commit path.
- `sqlite_udb_ops_per_commit{path}`: Histogram of UniversalDB operations per commit path.
- `sqlite_commit_envoy_dispatch_duration_seconds`: Pegboard-envoy histogram for websocket frame arrival to `sqlite-storage` dispatch.
- `sqlite_commit_envoy_response_duration_seconds`: Pegboard-envoy histogram for `sqlite-storage` return to websocket response send.
- `sqlite_commit_phases`: Actor inspector labeled timing metric exposed from `/inspector/metrics`. Values are `request_build`, `serialize`, `transport`, and `state_update`.

## Scrape Points

- Engine and pegboard-envoy Prometheus metrics come from the shared `/metrics` endpoint on port `6430`.
- Actor-local commit timings come from `GET /inspector/metrics` on the actor gateway route.

## Tracing

- Set `RUST_LOG=sqlite_storage=debug,pegboard_envoy=debug,sqlite_v2_vfs=debug` to emit per-commit phase spans and VFS phase logs.
- `sqlite-storage` commit handlers use debug spans for the high-level request and sub-phase work.
- The VFS logs request-build, serialize, transport, and state-update timings after each successful commit.

## Diagnosis

- High `decode_request` or `sqlite_commit_envoy_dispatch_duration_seconds` usually means envoy-side validation or actor lookup is slow before storage work starts.
- High `meta_read` or `pidx_read` points at UniversalDB read pressure or cache misses.
- High `ltx_encode` means commit encoding and compression are doing real work. Check dirty page counts and raw dirty bytes together.
- High `udb_write`, `meta_write`, or `sqlite_commit_envoy_response_duration_seconds` points at write-path latency after encode.
- A healthy actor should show non-zero `sqlite_commit_phases` totals after commits in `/inspector/metrics`. If SQL runs but those timings stay zero, the native VFS metrics path is broken.

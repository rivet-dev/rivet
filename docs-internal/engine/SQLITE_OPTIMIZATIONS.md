# SQLite Optimizations

Brief tracker for SQLite cold-read, VFS, and storage performance work.

Current baseline: `.agent/notes/sqlite-cold-read-before.txt` records a 50 MiB cold full-scan read at 20.14s e2e, 1,249 VFS `get_pages` calls, and 19.33s VFS transport.

Implementation tracking lives in `scripts/ralph/prd.json`.

## Existing Optimizations

- Actor startup can preload SQLite VFS pages through `OpenConfig.preload_pgnos`, `OpenConfig.preload_ranges`, and `OpenConfig.max_total_bytes`; pegboard-envoy currently uses the default config, so this mostly preloads page 1.
- The VFS keeps an in-memory page cache seeded from `sqlite_startup_data.preloaded_pages`.
- The VFS has speculative read-ahead via `prefetch_depth` and `max_prefetch_bytes`; the default forward-scan budget is 64 pages, which reduced the cold-read benchmark from 1,249 to 368 VFS `get_pages` calls.
- Actor Prometheus metrics expose VFS read counters, fetched bytes, cache hits/misses, and `get_pages` duration at `/gateway/<actor_id>/metrics`.
- `sqlite-storage` keeps an in-memory PIDX cache and decodes each unique DELTA/SHARD blob once per `get_pages(...)` call.
- `sqlite-storage` compaction folds DELTA pages into SHARD blobs for steadier read behavior.

## Recommended Optimizations

- Record VFS predictor access on cache hits so prefetch learns real sequential scans.
- Cache repeated pegboard-envoy SQLite actor validation and local-open checks for active actors.
- Return SQLite meta from `sqlite-storage::get_pages(...)` instead of doing a second META read in pegboard-envoy.
- Track recently used VFS pages or ranges, persist capped preload hints on sleep/close, and feed them into `OpenConfig` on the next actor start.
- Add a bulk or range page-read protocol so cold scans do not require page-list request loops.
- Reduce storage read amplification from chunked values and whole-blob LTX decode with larger chunks, range reads, decoded blob caching, or page-frame-addressable storage.
- Benchmark compacted and un-compacted cold reads separately.

## Update Rules

- Add new SQLite read/write performance ideas here before implementation if they change VFS, storage layout, actor startup preload, or metrics.
- Move completed ideas into "Existing Optimizations" with the measured benchmark delta.
- Keep benchmark artifacts under `.agent/notes/sqlite-cold-read-*.txt`.

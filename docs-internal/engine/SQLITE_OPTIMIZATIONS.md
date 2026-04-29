# SQLite Optimizations

Brief tracker for SQLite cold-read, VFS, and storage performance work.

Current baseline: `.agent/notes/sqlite-cold-read-before.txt` records a 50 MiB cold full-scan read at 20.14s e2e, 1,249 VFS `get_pages` calls, and 19.33s VFS transport.

Implementation tracking lives in `scripts/ralph/prd.json`.

Range page-read protocol details live in `.agent/specs/sqlite-range-page-read-protocol.md`.

## Existing Optimizations

- Actor startup can preload SQLite VFS pages through `OpenConfig.preload_pgnos`, `OpenConfig.preload_ranges`, and persisted `/PRELOAD_HINTS`; first pages, hint mechanisms, and the preload byte budget are configured through central SQLite optimization flags.
- The VFS keeps an in-memory page cache seeded from `sqlite_startup_data.preloaded_pages`; capacity, fetched/prefetched/startup cache classes, and scan-resistant protected-cache budget are configured through central SQLite optimization flags.
- The VFS has speculative read-ahead via `prefetch_depth` and `max_prefetch_bytes`; the default forward-scan budget is 64 pages, which reduced the cold-read benchmark from 1,249 to 368 VFS `get_pages` calls.
- The VFS tracks bounded recent page hints as hot pages plus coalesced scan ranges; `NativeDatabase::snapshot_preload_hints()` exposes the in-memory plan for future flush wiring.
- Actor Prometheus metrics expose VFS read counters, fetched bytes, cache hits/misses, and `get_pages` duration at `/gateway/<actor_id>/metrics`.
- `sqlite-storage` keeps an in-memory PIDX cache and decodes each unique DELTA/SHARD blob once per `get_pages(...)` call.
- `sqlite-storage` exposes `get_page_range(...)` for bounded contiguous reads; it reuses `get_pages(...)` source resolution and currently caps ranges at 256 pages / 1 MiB.
- `sqlite-storage` reassembles large chunked logical values with one bounded chunk-prefix range read by default, with `RIVETKIT_SQLITE_OPT_BATCH_CHUNK_READS=false` preserving the serial 10 KB chunk-get path.
- `sqlite-storage` caches decoded DELTA/SHARD LTX blobs across repeated reads by default, with `RIVETKIT_SQLITE_OPT_DECODED_LTX_CACHE=false` preserving per-read decode behavior.
- `sqlite-storage` compaction folds DELTA pages into SHARD blobs for steadier read behavior.

## Recommended Optimizations

- Gate SQLite cold-read optimizations behind central env-backed feature flags that default on, so each optimization can be benchmarked on and off.
- Add adaptive forward-scan read-ahead that can grow beyond shard-sized batches for mostly sequential reads while shrinking back for scattered access.
- Extend adaptive scan read-ahead to support both forward and backward sequential page access.
- Record VFS predictor access on cache hits so prefetch learns real sequential scans.
- Cache repeated pegboard-envoy SQLite actor validation and local-open checks for active actors.
- Return SQLite meta from `sqlite-storage::get_pages(...)` instead of doing a second META read in pegboard-envoy.
- Persist capped VFS preload hints on sleep/close and feed them into `OpenConfig` on the next actor start.
- Add a bulk or range page-read protocol so cold scans do not require page-list request loops.
- Reduce storage read amplification from whole-blob LTX decode further with page-frame-addressable storage.
- Benchmark compacted and un-compacted cold reads separately.

## Preload Hint Policy

- VFS preload hints are page-number based. SQLite index, table, schema, and overflow pages all hit VFS on pager-cache misses, but SQLite's pager cache can hide repeat access after the first read.
- Preload selection should consider early-after-wake pages in addition to frequency and scan ranges, because index/root/schema pages may be important even when VFS only observes them once per actor lifetime.
- Preload hint mechanisms must be independently configurable through the central SQLite optimization feature flag/config file, not scattered `std::env` reads.
- Supported preload mechanisms should include first pages, persisted hot pages, early-after-wake pages, and persisted scan ranges.
- All preload mechanisms should default on only when bounded by `OpenConfig.max_total_bytes` or an equivalent preload byte budget.

## Scan Read-Ahead Notes

- SQLite B-trees are sorted logically, not guaranteed linearly by page number; append-heavy `INTEGER PRIMARY KEY` tables are more likely to produce forward page scans than mixed inserts or freelist reuse.
- `INTEGER PRIMARY KEY` aliases rowid, so rowid/primary-key range scans are usually the best case for forward or backward VFS read-ahead.
- Non-integer primary keys on rowid tables use a separate index; index range scans can produce scattered table-page reads unless the index order is correlated with rowid or the query is covered by the index.
- `WITHOUT ROWID` tables are keyed by the declared primary key, but page splits can still make logical key order differ from physical page-number order.
- Adaptive read-ahead should grow only when observed VFS misses are directional, including both increasing and decreasing page numbers, and shrink for scattered access.

## Update Rules

- Add new SQLite read/write performance ideas here before implementation if they change VFS, storage layout, actor startup preload, or metrics.
- Move completed ideas into "Existing Optimizations" with the measured benchmark delta.
- Keep benchmark artifacts under `.agent/notes/sqlite-cold-read-*.txt`.

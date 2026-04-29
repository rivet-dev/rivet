# SQLite Storage Notes

- Gate SQLite storage performance optimizations behind central env-backed feature flags that are enabled by default and read once through a `OnceCell` or equivalent; do not scatter ad hoc `std::env` reads across storage modules.
- Persist preload hints under the separate `/PRELOAD_HINTS` key with generation fencing; do not mix hint records with normal META, SHARD, DELTA, or PIDX page data.
- `SqliteEngine::get_pages` returns `GetPagesResult` with pages and transaction-read meta; reuse that meta for successful responses instead of issuing a second META read.
- `SqliteEngine::get_page_range` shares `get_pages` source resolution through `read_pages`; use it for contiguous range reads and keep its 256-page / 1 MiB hard cap aligned with the range protocol.
- SQLite startup preload policy is configured in `optimization_flags.rs`; keep first pages, persisted hint mechanisms, and byte budget there instead of hardcoding open-time preload behavior.
- SQLite read-pool rollout knobs are configured in `optimization_flags.rs`; build `NativeConnectionManagerConfig` from those shared flags instead of hardcoding reader counts or TTLs.
- Native VFS page cache policy is configured as `off|target|startup|prefetch|all` in `optimization_flags.rs`; keep capacity and protected-cache budgets there.
- Large chunked logical values are reassembled with a bounded chunk-prefix range read by default; `RIVETKIT_SQLITE_OPT_BATCH_CHUNK_READS=false` selects serial 10 KB chunk gets for comparison runs.
- Repeated DELTA/SHARD LTX decodes are cached inside `SqliteEngine`; `RIVETKIT_SQLITE_OPT_DECODED_LTX_CACHE=false` preserves per-read decode behavior.
- LTX decoding validates header, page frames, page index structure, and a zeroed checksum trailer.

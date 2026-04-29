# SQLite Storage Notes

- Gate SQLite storage performance optimizations behind central env-backed feature flags that are enabled by default and read once through a `OnceCell` or equivalent; do not scatter ad hoc `std::env` reads across storage modules.
- Persist preload hints under the separate `/PRELOAD_HINTS` key with generation fencing; do not mix hint records with normal META, SHARD, DELTA, or PIDX page data.
- `SqliteEngine::get_pages` returns `GetPagesResult` with pages and transaction-read meta; reuse that meta for successful responses instead of issuing a second META read.
- `SqliteEngine::get_page_range` shares `get_pages` source resolution through `read_pages`; use it for contiguous range reads and keep its 256-page / 1 MiB hard cap aligned with the range protocol.
- Large chunked logical values are reassembled with a bounded chunk-prefix range read by default; `RIVETKIT_SQLITE_OPT_BATCH_CHUNK_READS=false` preserves the serial 10 KB chunk-get fallback.

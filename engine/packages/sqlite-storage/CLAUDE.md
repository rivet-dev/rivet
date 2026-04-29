# SQLite Storage Notes

- Gate SQLite storage performance optimizations behind central env-backed feature flags that are enabled by default and read once through a `OnceCell` or equivalent; do not scatter ad hoc `std::env` reads across storage modules.
- Persist preload hints under the separate `/PRELOAD_HINTS` key with generation fencing; do not mix hint records with normal META, SHARD, DELTA, or PIDX page data.

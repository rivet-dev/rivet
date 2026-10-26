# 0007: cache memory usage

**What it changes.** Adds `DbCache::estimated_memory_usage`, with a default of None, implemented for the foyer, moka, split and wrapper caches.

**Needed by.** The shared SlateDB cache in `engine/packages/env/src/backends/production/slatedb.rs`, for its usage metrics and for the bytes `ClearableCache::clear` reports.

**Upstream.** A one-method trait addition with a default; candidate for upstream. Depot could instead read foyer's usage directly, since it already wraps that cache.

**Drop when.** Upstream has it, or Depot reads foyer directly.

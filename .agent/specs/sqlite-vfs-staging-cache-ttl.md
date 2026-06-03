# SQLite VFS Staging Cache TTL Plan

Date: 2026-05-03

This plan changes the SQLite VFS page cache from a broad second-level pager cache into a short-lived staging cache for speculative pages. Demand pages fetched for `xRead` should be handed to SQLite and then forgotten by the VFS.

## Goals

- Avoid retaining pages in VFS memory after SQLite has already received them through `xRead`.
- Keep startup preload and read-ahead useful by retaining speculative pages briefly.
- Evict speculative pages on first successful target read so TTL is only the fallback for unused preloads.
- Keep lazy loading correct when all cache and preload features are disabled.
- Treat page 1 as staging data after `xRead` while keeping parsed page-size and database-size metadata.

## Non-Goals

- Do not change the remote `get_pages` protocol.
- Do not change SQLite pager settings.
- Do not add read pools back.
- Do not implement persisted preload hints in this branch.

## Current Behavior

- `resolve_pages` classifies fetched pages as `Target` when SQLite requested them and `Prefetch` when they were predicted.
- `fetch_initial_pages_for_registration` seeds startup pages as `Startup`.
- `should_cache_page` allows target, prefetch, and startup caching based on `SqliteVfsPageCacheMode`.
- Page 1 is always cacheable.
- Early protected pages live in `protected_page_cache`, which is an `scc::HashMap` with no TTL.

## Proposed Behavior

- Target pages should not be inserted into the VFS page cache by default.
- Target reads should remove speculative read pages from the cache after bytes are copied to the caller.
- Prefetch pages should be inserted into a TTL cache.
- Startup preload pages should be inserted into the same TTL cache.
- Commit completion should stage dirty pages in a separate TTL cache so SQLite can reread its own writes without retaining them permanently.
- Page 1 should follow the same staging rule as other pages after `xRead`. The VFS keeps parsed page-size and database-size metadata, and it can synthesize the empty page-1 header again before the first commit when depot has no database yet.
- Protected cache should no longer protect speculative pages forever. It should be removed or left unused in favor of the TTL cache.

## Configuration

- Add `RIVETKIT_SQLITE_OPT_VFS_STAGING_CACHE_TTL_MS`.
- Default to a short TTL such as `30000` ms.
- A value of `0` disables speculative retention while preserving lazy target fetches.
- Keep `RIVETKIT_SQLITE_OPT_VFS_PAGE_CACHE_MODE=off` as the stronger kill switch for all non-page-1 VFS caching.
- Do not use `RIVETKIT_SQLITE_OPT_VFS_PROTECTED_CACHE_PAGES` to pin VFS page bytes beyond `xRead`.

## Implementation Plan

1. Extend `SqliteOptimizationFlags` and `VfsConfig` with a bounded staging TTL field.
2. Build `page_cache` with `time_to_live(Duration::from_millis(ttl_ms))` when TTL is nonzero.
3. Split cache insertion semantics so `PageCacheInsertKind::Target` is not retained by default.
4. Add an explicit `evict_pages_after_target_read` helper that removes every consumed page from both normal and protected speculative caches.
5. Call that helper after `io_read` copies returned bytes into SQLite's buffer.
6. Evict dirty page numbers from the staging cache after commit completion.
7. Rework `protected_page_cache` so it cannot pin speculative pages forever.
8. Keep `seed_main_page` behavior intact for parsed page 1 metadata.
9. Update metrics naming only if needed. `page_cache_entries` can continue to report retained VFS entries.

## Expected Cache Matrix

| Page source | Retained after fetch | Evicted on target read | TTL fallback |
| --- | --- | --- | --- |
| Target `xRead` miss | No | Not needed | No |
| Read-ahead prefetch | Yes | Yes | Yes |
| Startup preload | Yes | Yes | Yes |
| Page 1 | Yes during bootstrap or preload | Yes | Yes when retained |
| Dirty write buffer | Existing behavior | Existing behavior | No |

## Tests

- Add a VFS test proving a target read miss does not increase retained VFS cache entries.
- Add a VFS test proving prefetch pages are retained before use and removed after target read.
- Add a VFS test proving startup preload pages are retained briefly and removed after target read.
- Add a VFS test proving `VFS_STAGING_CACHE_TTL_MS=0` still lazily fetches pages.
- Add a VFS test proving `VFS_PAGE_CACHE_MODE=off` still lazily fetches pages and does not retain non-page-1 pages.
- If practical, use Tokio time pause/advance to verify TTL expiry deterministically instead of sleeping.

## Open Questions

- Should target retention remain available as an explicit benchmark mode, or should we remove target caching from the shipped matrix?
- Should `VFS_PROTECTED_CACHE_PAGES` be deprecated now that VFS pages are staging-only?

# SQLite Range Page-Read Protocol

## Status

Specified for `SQLITE-COLD-012`. Runtime implementation starts in the following stories.

## Goal

Reduce cold forward-scan round trips by letting the actor-side SQLite VFS request a bounded contiguous page range instead of building large page-number lists for every scan window. Page-list `get_pages` remains the compatibility and random-read path.

## Protocol Shape

Add a SQLite request/response pair next to `SqliteGetPagesRequest` in `engine/sdks/schemas/envoy-protocol/v2.bare`, using a new protocol version rather than mutating an already published shape.

```bare
type SqliteGetPageRangeRequest struct {
	actorId: Id
	generation: SqliteGeneration
	startPgno: SqlitePgno
	maxPages: u32
	maxBytes: u64
}

type SqliteGetPageRangeOk struct {
	startPgno: SqlitePgno
	pages: list<SqliteFetchedPage>
	meta: SqliteMeta
}

type SqliteGetPageRangeResponse union {
	SqliteGetPageRangeOk |
	SqliteFenceMismatch |
	SqliteErrorResponse
}
```

The top-level wrappers should mirror the existing get-pages wrappers:

- `ToRivetSqliteGetPageRangeRequest { requestId, data }`
- `ToEnvoySqliteGetPageRangeResponse { requestId, data }`

Request fields:

- `actorId`: actor whose SQLite v2 database is being read.
- `generation`: SQLite generation fence for the actor open.
- `startPgno`: first requested page. Page `0` is invalid.
- `maxPages`: client requested page cap. `0` is invalid.
- `maxBytes`: client requested byte cap. `0` is invalid.

Response fields:

- `startPgno`: echoes the effective start page so callers can assert response alignment.
- `pages`: ordered contiguous `SqliteFetchedPage` entries starting at `startPgno`. Missing pages beyond `meta.dbSizePages` use `bytes = null`, matching existing `get_pages` semantics.
- `meta`: the `SqliteMeta` read in the storage transaction. Successful handlers should reuse this meta and should not call `load_meta` again.

## Caps

The server must clamp the requested range to a local hard cap before reading storage:

- `effective_pages = min(maxPages, server_max_pages)`
- `effective_bytes = min(maxBytes, server_max_bytes)`
- `page_budget_from_bytes = max(1, effective_bytes / meta.pageSize)`
- `returned_pages <= min(effective_pages, page_budget_from_bytes)`

Initial constants should match the current adaptive scan budget unless benchmarking proves a safer value:

- `server_max_pages = 256`
- `server_max_bytes = 1 MiB`

The request is invalid if `startPgno == 0`, `maxPages == 0`, or `maxBytes == 0`. The response must never exceed the server cap, even when the actor sends a larger request.

## Storage Semantics

`sqlite-storage` should expose a contiguous range-read method before the envoy protocol is wired:

```rust
get_page_range(actor_id, generation, start_pgno, max_pages, max_bytes) -> GetPagesResult
```

The method should reuse existing `get_pages` source resolution, PIDX cache, stale PIDX cleanup, zero-page fallback, and generation fencing. The main difference is that storage builds the contiguous page set internally after reading meta, rather than receiving a fully expanded list from the VFS.

Range reads should return the same bytes and meta as an equivalent `get_pages(actor_id, generation, start_pgno..start_pgno+n)` call for the same effective range.

## Fencing And Stale Ownership

Range reads must match existing `get_pages` behavior:

- pegboard-envoy validates actor ownership and namespace before storage access.
- Repeated active-actor validation may use the same `Conn.active_actors` fast path only when the cached active actor is running or stopping and its SQLite generation matches the request generation.
- Serverless local-open checks may use `Conn.serverless_sqlite_actors` only when the cached generation matches.
- A cached serverless generation mismatch returns `SqliteFenceMismatch`, not a silent reopen.
- A storage generation mismatch returns `SqliteFenceMismatch { actualMeta, reason }`.
- `actualMeta` is loaded from storage through the same helper used by `get_pages` fence responses.
- Stale-owner behavior must not fall back to a successful read from a different generation.

Only ordinary storage or validation failures use `SqliteErrorResponse`. Fence mismatches remain structured so the VFS can refresh metadata without treating takeover as data corruption.

## VFS Selection

The native SQLite VFS should use range reads only when all of these are true:

- `RIVETKIT_SQLITE_OPT_RANGE_READS` is enabled.
- The negotiated envoy protocol version supports the range request.
- Adaptive read-ahead selected `ReadAheadMode::ForwardScan`.
- The missing/prefetch plan is contiguous from the seed page.
- The selected window is larger than the shard-sized page-list path, initially `> 64` pages or `> 256 KiB`.

The VFS should continue to use page-list `get_pages` when:

- The read is a point read or small bounded prefetch.
- Access has decayed back to scattered/random mode.
- The desired pages are non-contiguous.
- Range reads are disabled by flag or unsupported by protocol version.
- A range request returns `SqliteErrorResponse` for an implementation or compatibility problem.

Do not fall back on `SqliteFenceMismatch`; handle it exactly as the current `get_pages` path does.

## Benchmark Expectations

Implementation stories should keep writing full cold-start benchmark output with:

```bash
pnpm --filter kitchen-sink exec tsx scripts/sqlite-cold-start-bench.ts --wake-delay-ms 10000 2>&1 | tee .agent/notes/sqlite-cold-read-after-<STORY_ID>.txt
```

Expected artifacts:

- `SQLITE-COLD-013`: `.agent/notes/sqlite-cold-read-after-SQLITE-COLD-013.txt`
- `SQLITE-COLD-014`: `.agent/notes/sqlite-cold-read-after-SQLITE-COLD-014.txt`
- `SQLITE-COLD-015`: `.agent/notes/sqlite-cold-read-after-SQLITE-COLD-015.txt`

Each implementation story should record insert e2e, hot read e2e, wake read e2e, wake read server, wake overhead estimate, wake read VFS request count, pages fetched, bytes fetched, prefetch pages, prefetch bytes, and VFS transport. Compare against the baseline plus the previous completed story.

The target for `SQLITE-COLD-015` is materially fewer VFS transport requests for cold full scans than the current adaptive read-ahead path, while keeping hot read e2e within normal local variance.

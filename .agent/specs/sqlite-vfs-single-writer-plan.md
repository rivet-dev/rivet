# SQLite VFS Single-Writer Remote Storage Plan (Option F)

## Status

Draft. Captures the Option F design from
`.agent/research/remote-sqlite-prior-art.md` under the hard constraints:

1. Pure VFS. No local SQLite file at any layer.
2. Single writer per database.
3. Preserve actor mobility. State must follow an actor across node moves.
4. Preserve SQLite durability semantics. Commit returns only after the
   KV-side write acks durably.

## Three pieces

### 1. Enable and pre-populate the VFS page cache at file open

- Remove the env-var gate on `read_cache` in
  `rivetkit-typescript/packages/sqlite-native/src/vfs.rs` so it defaults on.
  Tracked by **US-020**.
- Bump `PRAGMA cache_size` at database open so SQLite's own pager cache
  covers the working set without thrashing the VFS. Tracked by **US-021**.
- On VFS file open for the main SQLite database, issue a bounded parallel
  bulk fetch of the whole file and populate `read_cache` before the first
  SQL statement executes. Tracked by **US-025**.
- The existing `apply_flush_to_read_cache` (`vfs.rs:1064-1092`) already
  promotes dirty pages into `read_cache` on every successful commit, so
  writes keep the cache hot for free. No new machinery on the write path.
- Hydration budget: default 64 MiB per database, configurable per caller.
  Oversized DBs hydrate up to the budget and fall through to lazy fill for
  the tail.

### 2. Batched `sqlite_read_many` server op

- New envoy operation `sqlite_read_many(actor_id, file_tag, ranges)`
  returning page bodies for the requested chunk ranges in one response.
  Tracked by **US-026**.
- Versioned protocol addition. Clean fallback to existing `batch_get`
  against old servers.
- Symmetric with the existing `sqlite_write_batch` fast path and lives
  beside it in `engine/packages/pegboard/src/actor_kv/mod.rs`.
- **No fencing on reads.** Single writer means there is nothing to
  serialize against, and the writer already knows the only version that
  exists.
- Both hydration (Piece 1) and prefetch (Piece 3) route through this op.

### 3. VFS stride prefetch predictor

- Small per-file stride detector watching recent `xRead` offsets. Tracked
  by **US-027**.
- On cache miss with a detected stride, fire one `sqlite_read_many` for the
  next N predicted pages and populate `read_cache` so the subsequent
  pager-driven `xRead` calls hit the cache.
- Hard cap on prefetch window (initial: 128 pages). Auto-disable on
  sustained prediction miss rate.
- Covers workloads where the database is too large to hydrate fully.

## KV data structure: no changes required

All three pieces work on the existing page key layout
(`[0x08, 0x01, 0x01, file_tag, chunk_index_u32_be]` in
`engine/packages/pegboard/src/actor_kv/mod.rs`). Hydration calls `batch_get`
(upgrading to `sqlite_read_many` once US-026 lands). Prefetch calls the
same op. The write path is untouched.

**This is a feature.** Shipping Option F does not break `inspector`,
`delete_all`, generic `get` range scans, or quota accounting, because it
does not fork the page key schema. It also avoids the storage-migration
hazard that the previous spec marked as a hard constraint.

## One optional data-structure optimization (not in this plan)

**Drop per-page `EntryMetadataKey` writes for SQLite pages.** Today every
SQLite page write produces two KV entries: a metadata key (carrying version
string + `update_ts`, ~40 bytes) and a value chunk key (the page body).
SQLite owns versioning through its own pager state, so the metadata is
dead weight for pages. Removing it would roughly halve the KV-write count
on commit and save ~100 KB per 10 MiB commit.

**Why it's not in this plan:** the adversarial review showed
`EntryBuilder::build` at `engine/packages/pegboard/src/actor_kv/entry.rs`
calls `bail!("no metadata for key")` on missing metadata, and several
generic-path consumers (`get`, `inspector`, `delete_all`, quota accounting)
walk the SQLite subspace through that builder. Skipping per-page metadata
would require a dedicated SQLite-only server read path that bypasses the
generic entry builder. That is real work and it is worth doing **only
after** US-025 through US-027 prove their wins on the read side.

**Explicitly rejected data-structure changes** (these do not become
interesting under the single-writer constraint either):

- **Page bundling (pack N pages into one 128 KiB KV value).** The
  hydration path would benefit, but random-access reads after hydration
  pay 32x more bytes per miss and random writes do read-modify-write of
  the whole bundle. Kills OLTP workloads. Not worth it once the cache is
  hot.
- **Dedicated SQLite subspace with packed keys.** Saves ~15 bytes of
  tuple-encoding overhead per 4 KiB page (~0.4%) and breaks every
  generic-path consumer that walks `actor_kv::subspace(actor_id)`. Bad
  trade.
- **Blob mode for contiguous writes.** Redundant with the
  transaction-scoped dirty buffer that already exists in the VFS, and the
  `MAX_VALUE_SIZE = 128 KiB` cap forces re-chunking above that anyway.
- **zstd on the wire.** Compression CPU cost exceeds localhost wire time,
  and the server still writes uncompressed bytes to RocksDB. No win after
  the read path is batched.

## Rollout order

1. **US-020** and **US-021** ship first. One-line changes, biggest
   tactical win, prerequisites for everything else.
2. **US-025** lands hydration against the existing `batch_get` op.
3. **US-026** lands `sqlite_read_many` and hydration upgrades to it.
4. **US-027** lands the prefetch predictor, tuned against the broadened
   benchmark shapes from **US-023**.
5. Re-run the `examples/sqlite-raw` bench after each step and record the
   deltas in `BENCH_RESULTS.md`.

## Expected wins

- **Verify on hot cache**: ~5000 ms → ~50 ms. Pager cache and VFS cache
  serve every page.
- **Cold start for 10 MiB DB**: ~200 ms for one bulk hydration, then hot.
- **Cold start for 100 MiB DB**: bounded by the 64 MiB budget; the hot
  portion hydrates in ~1 s, rest lazy with prefetch.
- **Insert**: unchanged at the existing ~900 ms write-path floor. Writes
  are already batched through the fast path.
- **End-to-end `sqlite-raw`**: ~8800 ms → ~1500 ms after US-025 alone,
  ~1200 ms after US-026 + US-027.

## Open questions

1. **Hydration memory budget.** 64 MiB default per database. Does this
   match the existing actor memory allocation, or should it be derived
   from a per-actor budget?
2. **Hydration blocking.** Does hydration block the first SQL statement,
   or race it in the background and fall through to on-demand fetch for
   pages the pager touches before hydration completes?
3. **Predictor disable knob.** Do we need a per-database switch to turn
   off the stride predictor on pathological workloads, or are the
   auto-disable heuristics enough?
4. **`read_cache` data structure.** The existing `HashMap<Vec<u8>,
   Vec<u8>>` `read_cache` is fine at today's size. Hydration makes it
   16x bigger (up to 16000 entries for a 64 MiB budget). Do we need to
   swap it for a `BTreeMap<u32, Bytes>` or a slab layout keyed directly
   by chunk index before landing US-025, or is that a follow-up?
5. **Invalidation edge cases.** Single-writer means no concurrent
   invalidation, but we should confirm no code path issues `kv_put`
   against SQLite page keys outside the fast-path commit. If any exists,
   the cache could go stale. The existing fence-clearing on generic KV
   mutations in `engine/packages/pegboard-envoy/src/ws_to_tunnel_task.rs`
   should cover this.

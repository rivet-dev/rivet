# SQLite storage crash course

How the v2 SQLite storage engine reads, writes, compacts, and fences. Read this before changing anything in `engine/packages/sqlite-storage/`.

For VFS-side parity rules (native Rust ↔ WASM TS), see [sqlite-vfs.md](sqlite-vfs.md). This doc is about the storage backend that the VFS talks to.

For point-in-time recovery, actor forking, checkpoint retention, and restore suspend/resume orchestration, see [sqlite-pitr-forking.md](sqlite-pitr-forking.md).

## Storage layout

Every actor's data lives in UDB under per-actor prefix `[0x02][actor_id]` and four kinds of suffix keys:

| Suffix | Holds | Role |
|---|---|---|
| `META` | `DBHead` blob (vbare-encoded) | Per-actor header. Single key. |
| `PIDX/delta/{pgno: u32 BE}` | `txid: u64 BE` | Page index. Routes pgno → which DELTA owns it. |
| `DELTA/{txid: u64 BE}/{chunk_idx: u32 BE}` | LTX blob chunks | Per-commit page payloads. Multi-chunk because UDB chunks values internally past ~10 KB. |
| `SHARD/{shard_id: u32 BE}` | LTX blob | Cold compacted state. 64 pages per shard. |

Page bytes for any pgno live in **exactly one of two places**: a DELTA blob (recent commit, not yet compacted) or a SHARD blob (compacted cold state).

`DBHead` (`engine/packages/sqlite-storage/src/types.rs`) load-bearing fields:

- `generation` — fence. Bumps on takeover.
- `head_txid` — last committed txid.
- `next_txid` — reserved-but-not-yet-committed counter. `next_txid > head_txid` always.
- `materialized_txid` — last compaction watermark. `head_txid - materialized_txid` is delta lag.
- `db_size_pages` — current DB EOF in pages.
- `sqlite_storage_used` / `sqlite_max_storage` — quota.
- `page_size`, `shard_size` — fixed at 4096 and 64 respectively.

## Read path

When SQLite asks the VFS for page N, the VFS calls `get_pages(actor_id, generation, [pgno])` (`engine/packages/sqlite-storage/src/read.rs`).

```
1. Read META in-tx          → fence + db_size_pages + shard_size
2. If N > db_size_pages     → return missing (above EOF)
3. Look up N in PIDX:
     - Hit (txid T)         → page N is in DELTA T
     - Miss                 → page N is in SHARD (N / 64)
4. Read the chosen blob, decode LTX, extract page N's bytes
5. Stale-PIDX fallback: if PIDX said DELTA T but DELTA T is missing
   (compaction deleted it), fall back to SHARD (N / 64)
```

PIDX is the **routing table**. Without it, you'd scan every DELTA blob to find each pgno. With it, page N → one PIDX lookup → one blob read.

## Write path

When SQLite commits a transaction with N dirty pages, the VFS calls `commit(actor_id, generation, head_txid, dirty_pages, ...)` (`engine/packages/sqlite-storage/src/commit.rs`).

```
1. Read META in-tx          → fence (generation + head_txid) + allocate txid T
2. Encode all dirty pages into one LTX blob
   → write to DELTA/{T}/0, DELTA/{T}/1, ...
3. For each dirty page N: write PIDX/delta/{N} = T  (overwrites prior owner)
4. Update META: head_txid=T, next_txid=T+1, db_size_pages, sqlite_storage_used
5. Commit UDB tx
```

The PIDX writes are how a commit "claims" pages. Most-recent PIDX entry wins the read.

After commit succeeds, prior owners of those pgnos are now orphaned in their old DELTAs (no PIDX entry references them anymore). Compaction will eventually fold the orphans into shards.

## Compaction (the janitor's job)

```
1. Read META in-tx, PIDX, and the K oldest unmaterialized DELTAs
2. Group their pages by shard_id (= pgno / 64)
3. For each affected shard:
     - Read existing SHARD blob, merge in newer page versions, rewrite SHARD
     - Delete PIDX entries for pages that just got folded
4. Delete the K old DELTA blobs (no PIDX still references them)
5. Update META: materialized_txid = highest folded txid,
                sqlite_storage_used adjusted for bytes freed
```

After compaction, those pages are no longer in PIDX → reads fall through to the shard.

## Where PIDX is used

Three paths:

1. **Reads** — routing table. Every `get_pages` consults PIDX.
2. **Commits** — every dirty page writes a new PIDX row, overwriting the prior owner.
3. **Compaction** — reads PIDX to find what to fold, deletes PIDX rows for folded pages.

### The in-RAM PIDX cache

`SqliteEngine.page_indices: scc::HashMap<actor_id, DeltaPageIndex>` (`engine/packages/sqlite-storage/src/page_index.rs`) is a RAM snapshot of the `PIDX/delta/*` prefix.

- **Cold cache:** on `get_pages`, scan PIDX prefix in-tx, populate cache for next time.
- **Warm cache:** skip the scan, look up in RAM.
- **Commit:** update cache after the UDB write succeeds (add/overwrite the new pgno → txid mappings).
- **Stale entry:** cache says DELTA T owns pgno N but compaction deleted T. The read misses the DELTA blob, falls back to SHARD (`read.rs:144-150`), evicts the stale row.

The cache is **safe to be stale** because PIDX→DELTA misses always fall back to SHARD, and shards are the long-term home. Correctness lives in UDB; the cache is perf only.

## Cross-references

- VFS parity rules: [sqlite-vfs.md](sqlite-vfs.md)
- PITR and forking: [sqlite-pitr-forking.md](sqlite-pitr-forking.md)
- Storage metrics: [SQLITE_METRICS.md](SQLITE_METRICS.md)
- Engine-wide CLAUDE notes on SQLite quirks: `engine/CLAUDE.md` `## SQLite storage tests` and `## Pegboard Envoy`

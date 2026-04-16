> **Superseded (2026-04-15):** Original research doc from the design-exploration phase. The current design uses sharded LTX + delta log (Option D). See `docs-internal/rivetkit-typescript/sqlite-ltx/constraints.md` for the locked architecture.

# SQLite VFS v2 Redesign: LTX-style Log + Materialized Pages on Remote KV

Status: **Research draft, pre-adversarial**
Date: 2026-04-15
Scope: Replace the current page-per-key SQLite VFS in `rivetkit-typescript/packages/sqlite-native` with an LTX-inspired log-structured design backed entirely by the engine actor KV (UDB/FoundationDB), keeping a v1/v2 selector so old actors continue to work.

---

## 1. Why we are doing this

The current VFS stores every SQLite page as its own KV key. Every `xWrite` becomes a `kv_put`, every `xRead` becomes a `kv_get`. Even with the existing `BEGIN/COMMIT_ATOMIC_WRITE` batching path and the optional read cache, this layout has structural problems for databases that do not fit in the actor's memory:

- **Write amplification**: Updating one row in a B-tree page touches 1–4 pages but each becomes its own KV value. SQLite's own sub-4 KiB writes get rounded up to a full 4 KiB page in KV, so storage-side billing and RTTs scale badly.
- **No write coalescing across pages**: Even when 200 pages dirty, we serialize them one round-trip per page (or batched in groups limited by the engine's 128-key/976 KiB batch ceiling).
- **Cold reads are expensive**: A query that touches 50 pages is 50 KV gets unless the read cache is warm.
- **The journal file is itself in KV**: `FILE_TAG_JOURNAL` (`kv.rs:26`) means rollback journal pages are stored alongside main pages. Doubles the write cost of every transaction.
- **Startup is slow for non-trivial DBs**: The bounded preload only covers the working set if it was previously hot; cold actors pay round-trips for every header/freelist/root page.

We have a clean break opportunity: actors store a SQLite VFS schema-version flag in their KV subspace, and we can ship two side-by-side implementations chosen by that flag. v1 is what's there today; v2 is the new design.

This document is the research basis for v2. It does not yet include code; an implementation plan goes in `.agent/specs/sqlite-vfs-v2.md` after we close the open questions below.

---

## 2. What we have today (verified from source)

### 2.1 Current VFS shape
Code lives in `rivetkit-typescript/packages/sqlite-native/src/`:

- `vfs.rs` (1660 lines): full SQLite VFS callback implementation. Key callbacks:
  - `kv_io_read` (line 518), `kv_io_write` (line 645), `kv_io_truncate` (line 833)
  - `kv_io_file_control` (line 998) handles `SQLITE_FCNTL_BEGIN_ATOMIC_WRITE` / `COMMIT_ATOMIC_WRITE` / `ROLLBACK_ATOMIC_WRITE`
  - `kv_io_lock` / `kv_io_unlock` are no-ops (single-writer enforced at actor level)
- `kv.rs`: key-layout constants
  - `CHUNK_SIZE = 4096`, `SQLITE_PREFIX = 0x08`, `SQLITE_SCHEMA_VERSION = 0x01`
  - File tags: `FILE_TAG_MAIN = 0x00`, `FILE_TAG_JOURNAL = 0x01`, `FILE_TAG_WAL = 0x02`, `FILE_TAG_SHM = 0x03`
  - Meta key (4 bytes): `[SQLITE_PREFIX, SCHEMA_VER, META_PREFIX, file_tag]`
  - Chunk key (8 bytes): `[SQLITE_PREFIX, SCHEMA_VER, CHUNK_PREFIX, file_tag, chunk_idx_be32]`
- `sqlite_kv.rs`: the `SqliteKv` trait that the VFS calls into. The trait gives us:
  - `batch_get(actor_id, keys) -> KvGetResult { keys, values }`
  - `batch_put(actor_id, keys, values)`
  - `batch_delete(actor_id, keys)`
  - `delete_range(actor_id, start, end)`
- The engine-side concrete impl is `EnvoyKv` in `rivetkit-typescript/packages/rivetkit-native/src/database.rs:24-107`. It maps directly onto the engine's existing `kv_get` / `kv_put` / `kv_delete` / `kv_delete_range` runner-protocol ops.

### 2.2 Pragmas the VFS sets on every connection
From `vfs.rs:1513-1531`:
```
PRAGMA page_size = 4096;
PRAGMA journal_mode = DELETE;
PRAGMA synchronous = NORMAL;
PRAGMA temp_store = MEMORY;
PRAGMA auto_vacuum = NONE;
PRAGMA locking_mode = EXCLUSIVE;
```
Atomic batched writes are advertised via `kv_io_device_characteristics` returning `SQLITE_IOCAP_BATCH_ATOMIC`. SQLite recognises this and elides journal writes for transactions that fit inside a single `BEGIN/COMMIT_ATOMIC_WRITE` window.

### 2.3 Existing batching
`kv_io_file_control` for `SQLITE_FCNTL_COMMIT_ATOMIC_WRITE` (`vfs.rs:1018`) flushes a `BTreeMap<u32, Vec<u8>>` of dirty chunks plus the metadata key in a single `batch_put`. It hard-fails if the batch exceeds `KV_MAX_BATCH_KEYS = 128`. There is no fallback path for transactions that touch more than 128 pages.

### 2.4 Hard limits on the path to UDB
Verified against `engine/packages/pegboard/src/actor_kv/mod.rs:19-26` and `engine/packages/universaldb/src/`:

| Limit | Value | Source |
|---|---|---|
| `MAX_KEY_SIZE` | 2,048 B | `actor_kv/mod.rs:21` |
| `MAX_VALUE_SIZE` | 128 KiB | `actor_kv/mod.rs:22` |
| `MAX_KEYS` per batch | 128 | `actor_kv/mod.rs:23` |
| `MAX_PUT_PAYLOAD_SIZE` | 976 KiB | `actor_kv/mod.rs:24` |
| `MAX_STORAGE_SIZE` | 10 GiB per actor | `actor_kv/mod.rs:25` |
| `VALUE_CHUNK_SIZE` (FDB-internal) | 10 KB | `actor_kv/mod.rs:26` |
| Default list limit | 16,384 keys | `actor_kv/mod.rs:168` |
| FDB transaction time | 5 s | `universaldb/src/transaction.rs:18` |
| FDB transaction size | 10 MB | `universaldb/src/options.rs:140` |
| FDB raw value max | 100 KB | `universaldb/src/atomic.rs:66` |
| Billing chunk | 4 KiB | `util/src/metric.rs` |

Each `kv_put` from the actor side becomes exactly one FDB transaction (`actor_kv/mod.rs:284`). The actor side cannot multi-statement-transact across two `kv_put` calls today.

**The binding constraint for one atomic write is therefore not FDB's 10 MB / 5 s — it is the engine's 976 KiB / 128-key per-call envelope.** A 976 KiB batch encodes at best ~244 raw 4 KiB pages without overhead, and after metadata framing closer to ~200. Anything bigger has to be split across multiple `kv_put` calls and therefore multiple FDB transactions, and the v2 design has to give us atomicity across that split.

### 2.5 What we cannot change easily
The `*.bare` runner protocol (`engine/sdks/schemas/runner-protocol/v7.bare`) is versioned and cannot be modified in place per `CLAUDE.md`. New KV ops have to land as a new schema version with `versioned.rs` migration. We can add ops; we cannot mutate existing ones.

---

## 3. Prior art (verified from source, not hand-waved)

### 3.1 LTX file format (`github.com/superfly/ltx`)

LTX is a binary container for "the set of pages changed by a transaction range." It is not a storage system — it is a *file format and a Go encoder/decoder*. Litestream and LiteFS both write LTX, but where the bytes live and how they're applied is each tool's own problem.

Header (100 bytes, from `ltx.go`):
```
Version, Flags, PageSize, Commit (DB size in pages, post-tx)  -- 4×u32
MinTXID, MaxTXID                                              -- 2×u64
Timestamp                                                     -- i64
PreApplyChecksum (CRC64-ISO)                                  -- u64
WALOffset, WALSize                                            -- 2×i64
WALSalt1, WALSalt2                                            -- 2×u32
NodeID                                                        -- u64
```

Page block (verified from `encoder.go`):
- Each page is `PageHeader (6B: Pgno+Flags) + size (4B) + LZ4-compressed page bytes`.
- Pages must be written in ascending pgno order.
- "Snapshot" LTX files must include pages 1, 2, 3, … strictly contiguous (skipping the lock page).
- "Incremental" LTX files include only changed pages, still ascending.
- Empty `PageHeader{}` terminates the page block.

Then a varint-encoded **page index** (pgno → offset, size) and an 8-byte page-index-size trailer field, then the 16-byte trailer:
```
PostApplyChecksum (CRC64-ISO of full DB after applying)
FileChecksum      (CRC64-ISO of all bytes encoded above)
```

Pages are LZ4-block-compressed individually (`encoder.go:EncodePage`). The PostApplyChecksum is computed by hashing **uncompressed page bytes** as they're encoded, so an LTX file is self-verifying for the slice of pages it contains plus the rolling checksum.

### 3.2 LTX compaction (`compactor.go`)
The compactor takes N input LTX readers (assumed contiguous TXIDs), and walks them in parallel page-by-page in pgno order. For each pgno, **the latest input wins**. The output covers `[inputs[0].MinTXID, inputs[N-1].MaxTXID]`, uses the first input's PreApplyChecksum and the last input's PostApplyChecksum.

Two consequences:
1. **Compaction is purely a merge of changed-page lists**, not a materialization onto a SQLite file. After compaction the result is still an LTX file; you still need a separate step to apply it to a database.
2. **Snapshot LTX files (where every page is included) are equivalent to the materialized DB**. A "full snapshot" is just an LTX whose page list is dense over `[1, Commit]`. So "materializing" reduces to "compacting all LTX files from genesis to head, with the genesis being a zero-page snapshot."

### 3.3 LiteFS (`github.com/superfly/litefs`)

LiteFS uses **FUSE**, not a SQLite VFS shim. From `docs/ARCHITECTURE.md`:

> LiteFS passes all these file system calls through to the underlying files, however, it intercepts the journal deletion at the end to convert the updated pages to an LTX file.

LiteFS keeps a **real SQLite database file on a real local filesystem**, lets SQLite write through FUSE to that file normally, and uses FUSE callbacks to detect "transaction has committed":
- Rollback journal mode: `WriteJournalAt` notices the journal header was zeroed (PERSIST commit) → calls `CommitJournal()` which reads the dirty pages from the now-stable DB file and writes an LTX.
- WAL mode: `UnlockSHM` notices `WAL_WRITE_LOCK` was released → calls `CommitWAL()` which reads the WAL frames and writes an LTX.

Replicas receive LTX files over HTTP and apply them with `ApplyLTXNoLock` (`db.go`), which decodes pages and writes them straight to the local DB file with `writeDatabasePage`, then truncates to the LTX header's `Commit` size.

**LiteFS therefore always has two representations side-by-side: the materialized SQLite file (for reads) and the LTX log (for replication/recovery).** This is critical context: even LiteFS does not satisfy reads from LTX. It satisfies reads from a fully-materialized SQLite file.

We cannot copy this directly because we have no local filesystem. But the principle — that reads come from a page-addressable form, not from the log — is the right one.

### 3.4 mvSQLite (`github.com/losfair/mvsqlite`)

mvSQLite is the closest prior art to what we want. From `mvfs/src/vfs.rs` and `docs/commit_analysis.md`:

- It is a real SQLite VFS layer (not FUSE), shipped as a library plus an `LD_PRELOAD` shim.
- The VFS is split: **mvfs** runs in the SQLite process, **mvstore** is a separate FoundationDB-aware HTTP service. mvfs talks HTTP to mvstore, mvstore talks FDB. We can collapse this into a single Rust path because our actor and our engine already speak a runner protocol.
- Per-connection state in `mvfs::Connection`:
  - `page_cache: moka::Cache<u32, Bytes>` — LRU, default 5,000 pages.
  - `write_buffer: HashMap<u32, Bytes>` — dirty pages within the current transaction.
  - `predictor: PrefetchPredictor` — Markov bigram + stride detector that recommends "you just read page P, also fetch these next ones."
  - `txn: Option<Transaction>` — the current mvstore (HTTP) transaction.
- **Read path** (`do_read_raw`, `vfs.rs:~150`):
  1. Mark page in the txn read set (used for PLCC conflict detection — we do not need this).
  2. Hit page cache → return.
  3. Hit write buffer → return.
  4. Else: ask predictor for up to `PREFETCH_DEPTH` predicted pages, build one `read_many` HTTP request that batches `[current, ...predicted]`, populate the cache for all returned pages.
- **Write buffering**: every write goes to `write_buffer` first. On `force_flush_write_buffer`, batched into chunks of `WRITE_CHUNK_SIZE = 10` and pushed to mvstore as `write_many` RPCs. **`maybe_flush_write_buffer` triggers when the buffer hits 1000 pages**, i.e., during a long transaction the buffer gets pre-drained without waiting for commit.
- **Multi-phase commit** (`docs/commit_analysis.md`):
  - When `num_total_writes >= 1000`, mvstore uses **two** FDB transactions instead of one.
  - **Phase 1** writes the page contents into FDB (content-addressed by hash) and sets a per-namespace commit token. This phase can be split across multiple FDB transactions because FDB writes are append-only and not yet visible to readers.
  - **Phase 2** verifies the commit token unchanged, writes the (pgno, version) → contentHash index entries with `SetVersionstampedKey`, updates the namespace last-write-version, and commits in one FDB transaction.
  - Conflict detection is done at page level via FDB's read-conflict / write-conflict ranges. We don't need this because we have a single writer per actor.
- **Virtual version counter trick** (`do_read`): when serving page 1, mvSQLite overwrites bytes 24–28 and 92–96 with a per-connection counter. These are SQLite's `change_counter` and `version-valid-for-number`. Bumping them forces SQLite's own page cache to invalidate after an external change. Useful for us only if we ever support an external mutator; not needed for single-writer.

### 3.5 mvSQLite prefetch predictor (`docs/prefetch.md`, `mvfs/src/prefetch.rs`)
Worth pulling in basically as-is:
- Per-connection ~1.5 KB fixed-size memory.
- Bigram Markov chain on page **deltas** (not page numbers) to capture B-tree access patterns.
- Stride detector for sequential scans.
- 16-entry recent-history ring buffer for cold-start.
- Counts halved every 256 record() calls to adapt to changing access patterns.
- Predictions emitted only above probability/confidence thresholds.
- Reset on transaction end (history+stride cleared, Markov preserved).

This is a self-contained module we can port to our VFS without changing anything else.

---

## 4. Design proposal for v2

### 4.1 Goals
1. Databases that exceed actor RAM must be supported. Page reads must be lazy.
2. Atomic commits up to the actor's storage quota (10 GiB) must be possible, even though the engine `kv_put` envelope is 976 KiB.
3. Steady-state writes should fit in **one** runner-protocol round-trip when possible.
4. Cold reads should pay at most one round-trip per *prefetch group*, not one per page.
5. Implementation effort should stay below "rewrite mvstore." We already have UDB and an actor-scoped KV.
6. v1 actors must keep working unchanged. v2 is selected by an actor-side schema-version flag in their KV subspace.

### 4.2 High-level architecture

```
              SQLite (in-actor process)
                   │
   ┌───────────────┴────────────────┐
   │  KvVfsV2 (Rust SQLite VFS)     │
   │                                │
   │  - in-memory page cache (LRU)  │
   │  - per-tx write buffer         │
   │  - prefetch predictor          │
   │  - LTX encoder for log entries │
   │  - "head" pointer + log scanner│
   └───────────────┬────────────────┘
                   │ runner-protocol (existing kv_get/put/delete + a few new ops)
                   ▼
              Engine actor KV (UDB / FDB)
                   │
   ┌───────────────┴────────────────┐
   │  Subspace per actor:           │
   │   PAGE/<pgno_be32>  → page bytes (materialized)   │
   │   LOG/<txid_be64>/<frame_be16> → LTX frame bytes  │
   │   META → DBHead{txid, pgcount, schema_v=2, ...}   │
   │   COMPACT_CURSOR → next-txid-to-materialize       │
   └────────────────────────────────┘
```

There are **two storage forms in KV simultaneously**:
- **Materialized form** (`PAGE/<pgno>` → page bytes): the latest committed value for each page, addressable in O(1).
- **Log form** (`LOG/<txid>/<frame>` → LTX-encoded page batch): the tail of recent transactions that have not yet been collapsed into the materialized form.

A transaction's pages are written to the **log form first**, then the head pointer is flipped, then a **background materializer** rewrites them into the materialized form and trims the log. Reads consult the log tail before falling back to the materialized form, so newly-committed writes are visible immediately even before materialization runs.

### 4.3 Subspace key layout

All keys are scoped under the actor's existing `(RIVET, PEGBOARD, ACTOR_KV, actor_id)` prefix, then under a new `SQLITE_V2` byte. Inside that:

```
META                      → DBHead struct (Bare-encoded)
PAGE  / pgno_be32         → 4 KiB page bytes
LOG   / txid_be64 / frame_be16 → LTX frame bytes (1 frame ≤ ~120 KiB after LZ4)
LOGIDX/ txid_be64         → LTX header bytes only (lets us scan the tail without
                            pulling all frame bodies)
```

`DBHead` lives at META and is the single source of truth for "what is committed":
```rust
struct DBHead {
    schema_version: u32,    // 2
    db_size_pages: u32,     // SQLite "Commit" — file size in pages
    page_size: u32,         // 4096
    head_txid: u64,         // last committed LTX txid
    materialized_txid: u64, // largest txid fully merged into PAGE/
    log_min_txid: u64,      // oldest LTX still in LOG/
    creation_ts_ms: i64,
}
```

Reasons for this layout:
- The materialized PAGE form is **mvSQLite-shaped**: one key per page, fast point lookup, no scanning required to satisfy a read once you've consulted the log.
- The LOG form is **LTX-shaped**: lets us write a large transaction across many KV calls without intermediate visibility, and lets us reuse the LTX format for backup/inspect tooling later.
- `LOGIDX/` exists so the VFS can ask "what pgnos are dirty in transactions newer than `materialized_txid`" without fetching every page body. Each LOGIDX value is just the LTX header + the page-index varint stream from the LTX trailer, which is small (8 bytes per dirty page).
- The two forms never interleave: every txid is either fully in LOG/ (and not yet in PAGE/), fully in PAGE/, or in transition.

### 4.4 Write path

**Inside a SQLite transaction**, SQLite calls `xWrite` for each dirty page. With `SQLITE_IOCAP_BATCH_ATOMIC` set, SQLite skips the journal entirely (because we tell it we can do atomic batched writes). The VFS already handles `BEGIN_ATOMIC_WRITE` / `COMMIT_ATOMIC_WRITE` (`vfs.rs:1011-1080`); v2 keeps that callback shape and changes only what `COMMIT_ATOMIC_WRITE` does:

```
COMMIT_ATOMIC_WRITE:
  let dirty = state.dirty_buffer;          // BTreeMap<pgno, page_bytes>
  let new_db_size = state.saved_file_size; // SQLite committed size
  let new_txid = head.head_txid + 1;

  // 1. Encode dirty pages as a sequence of LTX *frames*.
  //    Each frame is an LTX file fragment that fits inside one kv_put envelope:
  //    - target frame size: <= MAX_PUT_PAYLOAD_SIZE / 8 ≈ 120 KiB compressed
  //    - we cap "pages per frame" so the encoded LTX fits before LZ4 worst-case
  //    - frames are numbered 0..F and concatenate to a complete LTX file
  let frames = encode_ltx_frames(dirty, new_db_size, new_txid, head.head_checksum);

  if frames.len() == 1 && fits_with_meta(frames[0]) {
    // FAST PATH: one round-trip, atomic at FDB level.
    kv.batch_put(actor_id,
      keys = [LOG/<new_txid>/0, LOGIDX/<new_txid>, META],
      values = [frames[0], header_bytes, encode_head(new_head)]);
  } else {
    // SLOW PATH: multi-phase commit.
    // Phase 1: stage frames in LOG/ but DO NOT update META yet.
    // Each kv_put is its own FDB tx; readers cannot see them because
    // head.head_txid is unchanged.
    for chunk in frames.chunks(MAX_FRAMES_PER_BATCH) {
      kv.batch_put(actor_id, keys=[LOG/<new_txid>/<frame_idx>...], values=[...]);
    }
    // Phase 2: atomic commit. Writes LOGIDX + new META in one kv_put.
    // After this returns, the new txid is visible.
    kv.batch_put(actor_id,
      keys = [LOGIDX/<new_txid>, META],
      values = [header_bytes, encode_head(new_head)]);
  }

  // 3. Update in-memory page cache with the dirty pages we just wrote
  //    (so the next read does not have to fetch them back from KV).
  for (pgno, bytes) in dirty { page_cache.insert(pgno, bytes); }
  // 4. Update head.head_txid in our local copy.
```

**Atomicity argument**: a transaction is committed iff META.head_txid was updated. Phase 1 writes are invisible because they are addressed under a future txid; if we crash between phase 1 and phase 2, the next actor startup sees orphan LOG entries with txid > head.head_txid and **deletes them** (see Recovery, §4.7). There is no partial-commit window from a reader's perspective.

**Why LTX framing instead of just splitting raw pages across keys**:
- LZ4-compressed page bytes are typically 50–70% of raw, so a 120 KiB frame holds ~170 4-KiB pages instead of ~30. Big multiplier on slow-path commit RTTs.
- LTX has a built-in PostApplyChecksum, which gives us crash-detection for free.
- LTX frames can be concatenated and fed to the existing Go/Rust LTX decoders as if they were a single file, so backup/inspect tooling works without a custom format.
- The existing LTX compactor can be used for "compact two LTX entries into one" if we ever want offline log compaction.

### 4.5 Read path

```
xRead(pgno):
  if cache.get(pgno) → return
  if write_buffer.get(pgno) → return    // current transaction's own writes

  // Check the unmaterialized log tail for this pgno.
  // We keep an in-memory map: dirty_pgnos_in_log: HashMap<pgno, txid>
  // populated at startup (§4.6) and updated after every commit + materializer run.
  if let Some(txid) = dirty_pgnos_in_log.get(pgno) {
      // Need to fetch the LTX frame containing this page.
      // We know which txid; LOGIDX tells us which frame within that txid holds the pgno.
      let frame = kv.batch_get([LOG/<txid>/<frame_idx>])?;
      decode_lz4_page(frame, pgno) → cache + return
  }

  // Materialized fast path with prefetch (mvSQLite-style).
  let predictions = predictor.multi_predict(pgno, PREFETCH_DEPTH);
  let to_fetch = [pgno, ...predictions]
                 .filter(|p| !cache.contains(p) && !dirty_pgnos_in_log.contains(p));
  let pages = kv.batch_get(to_fetch.map(|p| PAGE/<p>))?;
  for (p, bytes) in pages { cache.insert(p, bytes); }
  return cache[pgno];
```

Single-page random read: usually 1 round-trip. Cache hit: 0 round-trips. Sequential scan: ~1 round-trip per `PREFETCH_DEPTH` pages.

The unmaterialized-log path adds at most one extra round-trip, and only for pages dirtied by very recent transactions that the materializer hasn't caught up with.

### 4.6 Startup path

What we have to load:
1. `META` → `DBHead`. **One** `batch_get` of one key.
2. **All `LOGIDX/*` entries between `materialized_txid` and `head_txid`**. These are small (header + per-page index varints, ~16 bytes per dirty page). For a typical actor with tens to hundreds of unmaterialized pages, this is one or two `batch_get`s of up to 128 keys each, and we can `kv_list` in prefix mode for the LOGIDX prefix to avoid knowing the txid range up front.
3. From the LOGIDX entries, build `dirty_pgnos_in_log: HashMap<pgno, txid>`. This is the only state the read path needs to know the log tail.
4. Page 1 (the SQLite header). One more `batch_get`. Without this SQLite cannot open the connection.
5. **Nothing else.** No page bodies, no log frame bodies. Everything else is lazy.

Cold startup cost: 3 round-trips, regardless of database size, in the common case. Compare to today, where the bounded preload is the only thing standing between us and "round-trip per page during the first query."

We do NOT need to "load the entire LTX log that has not been materialized" the way Nathan suspected. We only need the **page index** of those entries. The frame bodies are fetched on demand the first time someone reads a page they cover. This is the single most important deviation from a literal LTX/Litestream reading of the problem.

### 4.7 Crash recovery on startup

The actor process can die mid-commit. v2's recovery is simpler than SQLite's journal recovery because the head-pointer flip is atomic:

```
on startup:
  head = kv.get(META)
  // Find any orphan log entries with txid > head.head_txid.
  // These are partial commits from a previous run.
  let orphans = kv.list(LOGIDX/, start=head.head_txid+1, end=∞)
  if !orphans.is_empty() {
      // Delete LOGIDX and LOG bodies for each orphan txid.
      for txid in orphans {
          kv.delete_range(LOG/<txid>/0, LOG/<txid+1>/0)
          kv.delete([LOGIDX/<txid>])
      }
  }
```

Orphans can only exist between the last successful phase 1 and the never-completed phase 2. They are guaranteed not to be referenced from META, so deleting them is safe.

Failure during recovery itself is also safe: orphan deletion is idempotent, so re-running it on the next startup attempt finishes the job.

### 4.8 Background materializer

A separate task running inside the actor periodically:
```
materialize_step:
  let head = current head (from in-memory mirror)
  let to_apply = LOG entries with txid in (head.materialized_txid, head.head_txid]
  // Pick a budget: e.g. K pages or N txids per pass.
  let batch = pick_budget(to_apply, max_pages = ~200)

  // For each page in the batch, latest-txid wins (mvSQLite-style merge).
  let merged: BTreeMap<pgno, page_bytes> = {};
  for txid in batch {
      let frames = fetch_log_frames(txid);
      for (pgno, bytes) in decode_ltx_pages(frames) { merged.insert(pgno, bytes); }
  }

  // Write merged pages to PAGE/ and bump head.materialized_txid in one atomic batch_put.
  // Then delete LOG/<txid>/ and LOGIDX/<txid> for all txids ≤ new_materialized_txid.
  kv.batch_put(actor_id,
    keys=[PAGE/<p>... , META],
    values=[merged_pages..., encode_head(new_head)]);
  kv.delete_range(LOG/<min_txid>, LOG/<new_materialized_txid+1>);
  kv.delete_range(LOGIDX/<min_txid>, LOGIDX/<new_materialized_txid+1>);
```

The materializer has the same 976 KiB / 128-key constraint, so it processes at most ~200 pages per pass, but it can run many passes in succession. Because it never crosses `head.head_txid`, it never races with the writer.

The materializer is **eventually-consistent for storage** (the unmaterialized log uses extra space until it runs) but **synchronous for visibility** (reads see committed data immediately via the log tail).

### 4.9 SQLite pragma changes for v2

```
PRAGMA page_size      = 4096;     // unchanged — still aligns with KV billing
PRAGMA journal_mode   = MEMORY;   // CHANGE: rollback journal in RAM; we do not
                                  //   need a persistent journal because IOCAP_BATCH_ATOMIC
                                  //   tells SQLite to skip it during atomic-write
                                  //   transactions, and MEMORY is the cheapest fallback
                                  //   for the rare cases that don't take that path.
PRAGMA synchronous   = OFF;       // CHANGE: KV layer provides durability; no fsync needed.
PRAGMA temp_store     = MEMORY;
PRAGMA auto_vacuum    = NONE;
PRAGMA locking_mode   = EXCLUSIVE;
```
Net effect: the VFS is **never** asked to read or write the rollback journal file, so we can drop `FILE_TAG_JOURNAL` handling entirely from v2. (We still keep the file open / file delete / size callbacks because SQLite wants them to exist; they just no-op for v2.)

`OFF` synchronous is safe because:
- SQLite "synchronous" controls fsync behavior. Our writes don't go to a local FS. They go to UDB, which has its own durability guarantees independent of fsync.
- An actor crash before COMMIT_ATOMIC_WRITE is no different in v2 than v1 — partial in-memory state is discarded.
- An actor crash after COMMIT_ATOMIC_WRITE returns is fully durable because UDB has acknowledged the META write.

### 4.10 New KV ops we may want to add (versioned schema bump)

Strictly speaking, the design above works with the existing `kv_get` / `kv_put` / `kv_delete` / `kv_delete_range` / `kv_list` ops. But there are two that would materially help:

1. **`kv_put_if_meta_unchanged(meta_key, expected_value, puts)`** — compare-and-swap on META, used to make Phase 2 of the multi-phase commit safe even if the runner protocol grows asynchronous behavior in the future. Currently we have a single-writer guarantee at the actor level so we can rely on it, but a CAS makes the design defensible without that.

2. **`kv_put_with_range_delete(puts, delete_ranges)`** — combine the materializer's "write merged pages + delete log entries" into a single FDB transaction, avoiding the window where new PAGE values exist alongside stale LOG entries. Today these are two separate kv_put / kv_delete_range calls, which is correct (the head.materialized_txid is the source of truth, not the presence of LOG entries) but slightly wasteful.

Both can be deferred to v2.1 if we want a smaller initial diff. The single-writer assumption makes them optional.

If we add ops, they go in a new runner-protocol version per the `CLAUDE.md` rule about not mutating published `*.bare` files.

---

## 5. Direct answers to Nathan's questions

> **How do we materialize LTX back into the normal pages? Is that part of native SQLite or are we going to have to do something custom for it?**

Custom. The LTX Go package (and any port we do) gives us encode/decode/compact only; it does not know about our storage. SQLite itself has no awareness of LTX. We write our own `apply_ltx_frames_to_kv` routine (the materializer, §4.8). It is conceptually 30 lines: decode pages, merge by latest-txid-wins, write to PAGE/, advance META. The mechanism mirrors LiteFS's `ApplyLTXNoLock` (`db.go`), except we write to KV instead of to a file descriptor.

> **What data do we need to load into the actor on startup?**

Three things, all small (§4.6):
1. `META` (one key, ~50 bytes Bare-encoded).
2. The `LOGIDX/*` entries for unmaterialized txids (small per-entry, listable in one or two `kv_list` calls).
3. Page 1 (one key, 4 KiB).

We do **not** need to load every LTX frame body. Frames are fetched on first read of the pages they contain, which is the same lazy-load behavior the materialized PAGE/ form gets.

> **How does this map to the VFS operations? How do we intercept log writes and then put it into the LTX form?**

The existing VFS already buffers writes via `BEGIN_ATOMIC_WRITE` / `COMMIT_ATOMIC_WRITE` (`vfs.rs:998-1080`). v2 keeps that exact callback shape — SQLite still calls `xWrite` for each dirty page during a transaction, the VFS still buffers them in a `BTreeMap<pgno, page_bytes>`, and the only thing that changes is what `COMMIT_ATOMIC_WRITE` does on flush:
- v1 today: `kv_put(actor_id, [PAGE_v1/<pgno>...], [pages...])` — fails if >128 keys.
- v2: encode the dirty buffer as LTX frames, write them to LOG/, flip META. Two paths (single-batch fast path and multi-batch slow path), both end with META being updated atomically.

We do not "intercept log writes" because we never let SQLite write a journal in the first place: with `SQLITE_IOCAP_BATCH_ATOMIC` advertised, SQLite skips the rollback journal for transactions inside an atomic-write window. For the rare transactions that don't take that path (mostly schema changes), `journal_mode = MEMORY` keeps the journal in RAM and our VFS never sees journal I/O.

> **What write mode, what journal mode are we using?**

- `journal_mode = MEMORY` (the journal lives in RAM — never written to KV).
- `synchronous = OFF` (KV layer handles durability).
- `locking_mode = EXCLUSIVE` (single writer per actor — already the case).
- `IOCAP_BATCH_ATOMIC` advertised so SQLite groups dirty pages into an atomic batch and skips the journal entirely for the common case.

WAL mode is **not** what we want. WAL gives SQLite its own log-structured write path, which would conflict with our LTX log. Rollback-journal-with-IOCAP_BATCH_ATOMIC lets us tell SQLite "I'll handle atomicity, just give me one big batch of dirty pages on commit" and is exactly the behavior we want.

> **How do large writes that exceed FoundationDB's transaction size work?**

Two layers of "doesn't fit":
1. **Doesn't fit in one `kv_put` envelope** (976 KiB / 128 keys). This is the binding constraint, hit before FDB's own 10 MB / 5 s limits. The slow path in §4.4 splits the transaction across multiple `kv_put` calls, each its own FDB transaction, all writing under the same future txid in the LOG namespace, with a final atomic `kv_put` flipping META. Atomicity comes from the head-pointer pattern, not from a single FDB transaction.
2. **Doesn't fit in one FDB transaction** (10 MB / 5 s). Same mechanism. Each `kv_put` from the actor side opens its own FDB transaction (`actor_kv/mod.rs:284`), so as long as each individual `kv_put` stays under 976 KiB, FDB's 10 MB and 5 s limits are not binding on the actor's perceived transaction size. The actor sees one logical SQLite transaction; it lands in UDB as N short, well-bounded FDB transactions.

> **UDB's 10 MB keys, 5-second transaction limits.**

Verified the actual numbers (§2.4): FDB native limits are 10 MB transaction size, 5 s transaction time, 100 KB per value; UDB inherits all of these. The actor KV layer wraps that with stricter envelopes — 128 KiB per value, 128 keys per batch, 976 KiB per put, 2 KiB per key. v2's slow path is designed against the **wrapper** limits, which are tighter, so it is automatically safe against the FDB native limits. We never need to issue an FDB-level transaction; everything goes through `kv_put`.

---

## 6. What this design *does not* solve

Spelling these out so adversarial review can hit them harder:

1. **Storage amplification while the materializer is behind**: dirty pages are stored in both LOG/ and (eventually) PAGE/. If the materializer is starved or paused, LOG/ grows. We need to bound LOG/ to keep the 10 GiB actor quota meaningful. Probably: refuse new commits (or block the writer) when `head_txid - materialized_txid` exceeds a threshold.

2. **Single-page random updates pay log+materialize cost**: a workload of "update one row, commit, update one row, commit" will write to LOG, then later the materializer rewrites the same page in PAGE. This is the classic LSM write amplification. For very write-heavy workloads it might be worse than v1's "write the page directly." Mitigation: the fast path in §4.4 is one round-trip too; the materializer can coalesce multiple txids touching the same page into one PAGE write.

3. **Cold reads of materialized pages are still one RTT**: there is no way around this without speculative prefetch. The mvSQLite predictor helps for sequential / B-tree access patterns but not for genuinely random access. Same problem v1 has.

4. **Compaction of LOG entries is not part of v2.0**: if the materializer keeps up, LOG entries are short-lived and compaction is unnecessary. If we ever want to keep LOG entries for replication or PITR, we'd add the LTX compactor as a v2.1 thing.

5. **`schema_v2` flag is per-actor, not per-database file**: actors with multiple SQLite files all live on the same VFS. We cannot mix v1 and v2 files inside one actor. This is a one-way migration when the actor first opens its DB.

6. **The frame size choice is a tradeoff and we don't have measurements yet**: bigger frames mean fewer round-trips on the slow path but more wasted bandwidth if a transaction touches few pages but happens to span the boundary. We need a benchmark before fixing the constant.

7. **The materializer adds CPU and KV bandwidth in the background**: actors that idle expensively are now slightly less idle. We need to gate it (run only when LOG nonempty, back off when caught up).

8. **No explicit handling of SQLite's "lock page"** at byte offset 1 GiB: SQLite refuses to read/write the page that contains the byte at 0x40000000 (the SQLITE_FCNTL_LOCK_PROXY page). LTX skips it explicitly in `EncodePage` (`encoder.go:~180`). We need to do the same when encoding/decoding LTX frames.

9. **The recovery path assumes META updates are atomic on the engine side**: our `kv_put` is one FDB transaction, so a single META key write is atomic. If the META key itself ever exceeded 128 KiB (it won't — it's <100 bytes) the assumption breaks. Worth a static assertion in code.

10. **The dirty_pgnos_in_log map can grow**: pathological case is "one transaction touches every page of a 10 GiB DB and the materializer is stopped." That's 2.6M entries × ~12 bytes = ~30 MB of in-memory state. Probably fine but worth bounding.

---

## 7. Open questions for adversarial review

These should be the targets of the adversarial passes:
- Is the fast/slow split at "fits in one `kv_put`" the right boundary, or should we always go through LOG/ for uniform behavior?
- Does the multi-phase commit truly preserve atomicity in every failure mode the actor + UDB combination can exhibit?
- Is the materializer's "latest txid wins" merge safe for SQLite's freelist / pointer-map / overflow page interactions?
- Are we correct that `journal_mode = MEMORY` + `IOCAP_BATCH_ATOMIC` lets SQLite skip the journal in all the cases we care about, including ALTER TABLE and VACUUM?
- Is the `dirty_pgnos_in_log` startup-load actually small enough to be unconditional, or should it itself be lazy?
- What happens if SQLite issues `xRead` on a page that the LOG frame says is dirty, but the LOG frame body is missing because the materializer raced and deleted it before updating LOGIDX? (This is an ordering bug we need to nail down.)
- Does any of the 10 KiB FDB-internal value chunking (`actor_kv/mod.rs:26`) interact badly with our 4 KiB SQLite pages stored as values?
- What's the v1→v2 migration story for an actor that already has data?

---

## 8. References

- Current VFS: `rivetkit-typescript/packages/sqlite-native/src/{vfs.rs,kv.rs,sqlite_kv.rs}`
- Engine actor KV: `engine/packages/pegboard/src/actor_kv/mod.rs`
- UDB transaction limits: `engine/packages/universaldb/src/{transaction.rs:18, options.rs:140, atomic.rs:66}`
- Runner protocol KV ops: `engine/sdks/schemas/runner-protocol/v7.bare:12-106`
- LTX format: `github.com/superfly/ltx` — `ltx.go`, `encoder.go`, `compactor.go`, `file_spec.go`
- LiteFS: `github.com/superfly/litefs` — `docs/ARCHITECTURE.md`, `db.go` (`WriteDatabaseAt`, `CommitJournal`, `CommitWAL`, `ApplyLTXNoLock`)
- mvSQLite VFS: `github.com/losfair/mvsqlite` — `mvfs/src/vfs.rs`, `mvfs/src/prefetch.rs`, `docs/commit_analysis.md`, `docs/prefetch.md`

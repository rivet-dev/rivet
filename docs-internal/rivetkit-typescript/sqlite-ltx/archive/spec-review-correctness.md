# SQLite VFS v2 Spec -- Adversarial Correctness Review

Reviewed: SPEC.md (sections 1-15)
Cross-referenced: `actor_event_demuxer.rs`, `universaldb/transaction.rs`, `sqlite-native/src/vfs.rs`, `keys/actor_kv.rs`, `utils/keys.rs`

---

## Findings

### 1. [CRITICAL] Schema-version dispatch probes the wrong byte

Section 8 says: "probe the actor's UDB subspace for the first key. If prefix byte is 0x01: route to v1. If prefix byte is 0x02: route to v2."

This is wrong. The actor KV subspace is `(RIVET=0, PEGBOARD=3, ACTOR_KV=72, actor_id)` with tuple-layer encoding. Keys inside it are tuple-encoded (nested bytes with escape sequences). v1 SQLite keys start with raw byte `0x08` (SQLITE_PREFIX), not `0x01`. The `0x01` in v1 is the schema-version byte at offset 1, not the first byte of the key within the subspace. Meanwhile, general `c.kv.*` keys use tuple-encoded `KeyWrapper` with a leading `NESTED` code byte (`0x05`).

The dispatch probe must either: (a) match on the raw first byte within the subspace (`0x08` = v1 SQLite, `0x05` = general KV, proposed new prefix = v2 SQLite), or (b) use a completely separate subspace prefix for v2. As written, an actor with only general KV data (first byte `0x05`) would match neither `0x01` nor `0x02` and would be misrouted or error. An actor with v1 SQLite data (first byte `0x08`) would also match neither. The spec's dispatch scheme is broken.

### 2. [CRITICAL] StoreTx trait is sync but UDB transactions are async

Section 6.2 defines `StoreTx` with sync methods (`fn get(&self, key) -> Result<Option<Vec<u8>>>`). The actual UDB transaction API (`universaldb::Transaction`) has async reads (`async fn get`, `async fn read`). Writes are sync (fire-and-forget into the transaction buffer), but reads require `.await`.

The `transact` signature takes `Box<dyn FnOnce(&dyn StoreTx) -> Result<()>>` (sync closure, sync trait). This cannot call `tx.get()` on a real UDB transaction without `block_on`, which would deadlock inside a tokio runtime. The trait needs async reads, or `transact` needs to accept an async closure matching `db.run(|tx| async { ... })`. The `MemorySqliteStore` would work (sync BTreeMap), but the production `UdbSqliteStore` cannot implement `StoreTx` as specified.

### 3. [IMPORTANT] PIDX cache not rebuilt after crash between commit and cache update

Section 6.4 says the commit handler updates the in-memory PIDX cache after a successful UDB transaction. If the engine process crashes after the UDB commit but before the cache update, the PIDX cache on restart will be stale. Section 6.3 says the cache is "loaded lazily from PIDX/delta/* on first access via prefix scan," which would fix this on a clean restart. However, the spec does not explicitly state that the cache is invalidated or rebuilt on engine restart. If the engine process is long-lived and handles multiple actors, a crash-and-restart of the engine means all actors' PIDX caches are rebuilt lazily, which is correct. This needs an explicit statement that the PIDX cache is ephemeral and always rebuilt from persistent PIDX keys on first access per engine process lifetime. The persistent PIDX in UDB is the source of truth, but the spec should say so clearly.

### 4. [IMPORTANT] Compaction may incorrectly advance materialized_txid

Section 7.3 step 6 says "advance materialized_txid." But `materialized_txid` should only advance to the highest txid fully consumed across all shards, not per-shard. If delta txid=5 touches shards 0 and 3, compacting shard 0 should not advance `materialized_txid` to 5 because shard 3 still has unconsumed pages from that delta. The spec says the delta is only deleted when "no PIDX entries reference it" (section 7.4), which is correct for deletion, but `materialized_txid` advancement logic is underspecified. Advancing it prematurely could cause a reader to skip checking PIDX for a delta that still has unmaterialized pages in other shards.

### 5. [IMPORTANT] xSync creating many tiny deltas

Section 5.6 says "The next xSync call commits them as a single-page delta." SQLite may call xSync multiple times during journal-mode recovery or schema changes. Each call would create a separate delta with potentially one page each. The spec acknowledges this in the failure table ("Writes outside atomic window: Buffered and flushed on next xSync as a single-page delta") but does not address the performance impact: many single-page deltas degrade read performance (PIDX lookups, more batch_get keys) and increase compaction pressure. The spec should either batch consecutive non-atomic writes until the next atomic-write window, or document that this is an accepted degradation for a rare path.

### 6. [IMPORTANT] Concurrent takeover race is not fully addressed

Section 4.2 asks: what if two actors call `sqlite_takeover` simultaneously? The spec says the CAS check uses `expected_generation`. If both send `expected_generation=G` simultaneously, UDB serializable transactions ensure only one commits (the other gets a conflict and retries). On retry, the retrying actor reads generation=G+1, which mismatches its `expected_generation=G`, so it gets `SqliteFenceMismatch`. This is correct IF UDB transactions provide serializable isolation with conflict detection on the META key. The spec should explicitly state that the META read + write in takeover must be in a single UDB transaction with read-your-writes isolation. The RocksDB driver does provide this via `OptimisticTransactionDB`, but the spec should not assume this implicitly.

### 7. [CLARIFICATION] Compaction merge with db_size_pages truncation

Walking through: delta D1 (txid=1, pages {1,2,3}, db_size_pages=3), D2 (txid=2, pages {2,65}, db_size_pages=66), D3 (txid=3, pages {1,3}, db_size_pages=2, truncation). Compacting shard 0 (pgnos 1-64): merge gives page 1 from D3, page 2 from D2, page 3 from D3. But db_size_pages=2 from D3 means pages 3-64 should not exist. The compaction merge is "latest-txid-wins per pgno" but does not account for truncation. The merged shard would contain page 3 from D3 even though it is beyond the new db_size_pages. The spec does not describe how compaction handles truncation. The shard should either exclude pages beyond db_size_pages or the reader should filter by db_size_pages.

### 8. [CLARIFICATION] Missing db_size_pages propagation from compaction to actor

Section 3.2 stores `db_size_pages` in META. Compaction does not change `db_size_pages` (only commits do). But if the actor reads `meta.db_size_pages` from a commit response and then compaction runs, the actor's cached `db_size_pages` remains correct because compaction does not modify it. This is fine. However, if compaction were extended to handle truncation cleanup (removing pages beyond db_size_pages from shards), the actor would not be notified. The spec should clarify that compaction never modifies db_size_pages.

### 9. [CLARIFICATION] delta_count gauge accuracy

The metric `sqlite_v2_delta_count` is an IntGauge. It is unclear when this gauge is updated. If it is only updated on commit and compaction, it could be stale between operations. If it is per-actor, it needs a label. If it is global, it needs to aggregate across all actors. The spec should clarify the scope and update frequency.

### 10. [VERIFIED-OK] Fast-path commit atomicity

Walked through: CAS check + DELTA write + PIDX writes + META update all happen in one `SqliteStore::transact`. If the transaction commits, all are visible atomically. If it fails, none are visible. Crash before commit: no state change. Crash after commit: consistent state. A reader running concurrently sees either the old META (before commit) or the new META + DELTA (after commit), never a partial state. The read path uses a single UDB snapshot (section 6.5: "one UDB read operation total"), so it cannot see a half-committed transaction.

### 11. [VERIFIED-OK] Slow-path commit atomicity

Stage chunks are invisible to readers (section 4.5: "Stage entries are invisible to readers until commit_finalize"). The finalize step assembles staged chunks into a DELTA + PIDX + META update in one transaction, deleting STAGE entries atomically. If finalize crashes before commit, orphan STAGE entries are cleaned up on next takeover. If finalize crashes after commit, consistent state. A reader never sees partial staged data.

### 12. [VERIFIED-OK] Compaction coordinator deduplication

Section 7.1 uses `HashMap::entry(Vacant)` to skip spawning a worker if one is already running. This prevents duplicate compaction for the same actor. The reap interval cleans up finished workers. A commit arriving while compaction is running is deduplicated. After compaction finishes and is reaped, the next commit will spawn a new worker. No starvation risk for other actors because workers are per-actor tasks.

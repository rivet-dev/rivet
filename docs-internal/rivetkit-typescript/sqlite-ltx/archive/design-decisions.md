# SQLite VFS v2 — Design Decisions, Corrections, Action Items

Companion to [`constraints.md`](./constraints.md) and [`walkthrough.md`](./walkthrough.md). This is the running log of decisions, corrections, and outstanding work.

> **Read [`constraints.md`](./constraints.md) first.** It holds the locked C0–C8 constraint set and the architecture decision (Option D: sharded LTX + delta log). Everything below is downstream of those constraints.

> **Status (2026-04-15):** Active. Updated as decisions land.

---

## 1. Critical corrections to earlier drafts

### 1.1 v1 does NOT hard-reject large transactions

**Earlier claim (wrong):** "v1 hard-rejects transactions over 128 dirty pages with `SQLITE_IOERR`. v2 fixes a correctness gap."

**Reality:** Per [SQLite tech-note 714f6cbbf7](https://sqlite.org/cgi/src/technote/714f6cbbf78c8a1351cbd48af2b438f7f824b336): when `COMMIT_ATOMIC_WRITE` returns an error, SQLite rolls back and **retries the same transaction through its normal rollback-journal path.** The transaction still succeeds; it just takes 3–10× longer because the journal path issues many small writes.

**Implication:** The "287 puts for 1 MiB" measurement in `examples/sqlite-raw/BENCH_RESULTS.md` is the journal-fallback path being exercised. v1 handles all transaction sizes correctly. v2 is **purely a performance project, not a correctness fix**. The framing in any earlier draft that said "v2 lets transactions over 128 pages succeed at all" should be replaced with "v2 lets transactions over 128 pages succeed *fast*."

### 1.2 LTX rolling checksum is not "scan the whole DB"

**Earlier confusion:** "LTX checks in its entire database for every change."

**Reality:** LTX's `PostApplyChecksum` is a *running* CRC64 maintained as a single 8-byte scalar by XOR-ing new page bytes in and old page bytes out. You never re-hash the whole DB; you just need the OLD page bytes when you write a page (so you can XOR them out of the running checksum). Since SQLite reads-before-writes in normal operation, the cost on the hot path is essentially zero.

**Decision:** **Drop the rolling checksum entirely.** It exists for LiteFS replica validation and we don't replicate. SQLite has its own page integrity, and UDB guarantees byte fidelity. We use the LTX format as a serialization wrapper only and write zeros into all checksum fields. Removes a lot of complexity.

### 1.3 UDB does not have a 10 MB transaction-size limit

**Earlier claim (wrong):** "FDB tx size = 10 MB at `options.rs:140`."

**Reality:** UDB's actual drivers are postgres and rocksdb (`engine/packages/universaldb/src/driver/`). There is no `fdb/` driver. The "10 MB" reference at `options.rs:140` is a docstring describing upstream FDB behavior and is not enforced anywhere. Similarly the `100 KB per value` from `atomic.rs:66` is scoped only to `apply_append_if_fits`, not a general cap.

**The only enforced UDB limit is the 5-second transaction timeout** (`transaction.rs:18`). Everything else is set by the engine's actor KV layer, which we control.

**Implication:** All "FDB" framing in the doc should be restated as "UDB." The binding constraint on a single op is the 5-second deadline, not a byte budget.

### 1.4 Single-writer is not enforced by the engine

**Earlier claim (wrong):** "Single writer per actor — already the case. We can rely on it."

**Reality:** The runner_id check at `pegboard-runner/src/ws_to_tunnel_task.rs:205-220` runs in a separate UDB transaction from the subsequent `actor_kv::put`. During runner reallocation, two processes can briefly both believe they own the actor. Without explicit fencing, both can corrupt each other's commits.

**Decision:** v2 **requires** generation-token fencing. Every `kv_sqlite_*` op carries `(generation, expected_head_txid)`. The engine-side op is a CAS — fails closed if the generation doesn't match. On startup, every new actor calls `kv_sqlite_takeover` which CASes the generation forward.

This means the v2 design hard-depends on the new SQLite-dedicated KV ops landing first. It cannot ship on the existing `kv_put` path even with workarounds.

### 1.5 The migration story is "no migration"

**Decision (per Nathan, 2026-04-15):** v1 actors stay v1 forever. v2 actors start v2 and stay v2 forever. Schema-version dispatch happens at actor open time by reading the version byte of the first key in the actor's KV subspace. There is no v1→v2 migration code, ever. If a user wants to move a v1 actor to v2, they export and reimport.

### 1.6 The pragma changes from §4.9 of the earlier draft are reverted

**Earlier proposal:** `journal_mode = MEMORY`, `synchronous = OFF`.

**Reality:** Per a [SQLite forum thread](https://sqlite.org/forum/forumpost/3bd8d497b2), this combination has had bugs where writes leak outside the batch atomic group. We also don't have empirical evidence today that `IOCAP_BATCH_ATOMIC` actually elides journal writes in our workload (the bench's 287 puts for 1 MiB is consistent with the journal-fallback path being taken).

**Decision:** Keep the v1 pragma defaults: `journal_mode = DELETE`, `synchronous = NORMAL`. v2's perf win comes from the LTX-framed log replacing the journal-fallback path, not from changing pragmas.

### 1.7 We can change the KV protocol freely

**Per Nathan:** The runner protocol is versioned and we can add new schema versions whenever we want. We are not constrained to making v2 work over the existing `kv_put` op. New ops are encouraged.

This unlocks the entire `kv_sqlite_*` op family below.

---

## 2. The new `kv_sqlite_*` KV protocol

These ops live in a new runner-protocol schema version (post-v7). They are dedicated to the SQLite VFS and have different (larger) limits than the general actor KV. Existing actor KV ops are unchanged.

### 2.1 Limits

| Limit | Existing actor KV | New `kv_sqlite_*` |
|---|---|---|
| Max value size | 128 KiB | ~1 MiB |
| Max keys per call | 128 | ~512 |
| Max payload per call | 976 KiB | ~9 MiB |
| Max key size | 2 KiB | 2 KiB (unchanged) |
| Total actor storage | 10 GiB | shared with actor KV |
| Transaction time limit | 5 s | 5 s (UDB-enforced) |

The 9 MiB envelope leaves headroom under the implicit "fits in one UDB transaction within 5 s" constraint. With LZ4 compression on SQLite pages (~2× ratio in practice), 9 MiB of compressed LTX corresponds to roughly 4,500 raw pages per atomic commit. Most application transactions fit comfortably.

### 2.2 Op definitions (sketch)

```bare
type KvSqliteCommit struct {
    actor_id:           ActorId
    generation:         u64
    expected_head_txid: u64

    log_writes:         list<KvKeyValue>     // LOG/<txid>/<frame> + LOGIDX/<txid>
    meta_write:         KvValue              // new META bytes
    range_deletes:      list<KvKeyRange>     // optional: cleanup of stale orphans
}

type KvSqliteCommitStage struct {
    actor_id:           ActorId
    generation:         u64
    txid:               u64                  // for orphan-cleanup scoping
    log_writes:         list<KvKeyValue>     // LOG/<txid>/<frame_idx> only
    wipe_txid_first:    bool                 // true on first stage, false otherwise
}

type KvSqliteMaterialize struct {
    actor_id:           ActorId
    generation:         u64
    expected_head_txid: u64

    page_writes:        list<KvKeyValue>     // PAGE/<pgno>
    range_deletes:      list<KvKeyRange>     // LOG/<a>..<b>, LOGIDX/<a>..<b>
    meta_write:         KvValue              // new META with advanced materialized_txid
}

type KvSqlitePreload struct {
    actor_id:           ActorId
    get_keys:           list<KvKey>          // META, PAGE/1, user-specified hints
    prefix_scans:       list<KvKeyRange>     // LOGIDX/, optional user hints
    max_total_bytes:    u64                  // safety bound
}

type KvSqliteTakeover struct {
    actor_id:           ActorId
    expected_generation: u64
    new_generation:      u64
}
```

All ops are CAS where applicable. All ops fail closed with explicit error variants.

### 2.3 Engine-side implementation

Each op is one `db.run(|tx| ...)` closure. The order inside the closure is:

```
1. Read META
2. CAS check (generation + expected_head_txid)
   If mismatch: return KvSqliteFenceMismatch with current values
3. Optional range-delete cleanup
4. Apply writes
5. Commit
```

Combined put + range-delete + meta update is the key new capability that lets the materializer maintain its invariants atomically. The existing `kv_put` cannot do this.

### 2.4 Trait surface

The Rust-side `SqliteKv` trait at `rivetkit-typescript/packages/sqlite-native/src/sqlite_kv.rs` needs new methods:

```rust
trait SqliteKv {
    // ... existing batch_get / batch_put / batch_delete / delete_range ...

    async fn sqlite_commit(&self, actor_id: &str, op: KvSqliteCommit)
        -> Result<(), KvSqliteError>;

    async fn sqlite_commit_stage(&self, actor_id: &str, op: KvSqliteCommitStage)
        -> Result<(), KvSqliteError>;

    async fn sqlite_materialize(&self, actor_id: &str, op: KvSqliteMaterialize)
        -> Result<(), KvSqliteError>;

    async fn sqlite_preload(&self, actor_id: &str, op: KvSqlitePreload)
        -> Result<KvSqlitePreloadResult, KvSqliteError>;

    async fn sqlite_takeover(&self, actor_id: &str, op: KvSqliteTakeover)
        -> Result<(), KvSqliteError>;
}
```

`EnvoyKv` (`rivetkit-typescript/packages/rivetkit-native/src/database.rs`) implements them by delegating to new napi methods on `EnvoyHandle`.

The in-memory test driver (see [`test-architecture.md`](./test-architecture.md), forthcoming) implements them against an in-process `BTreeMap<Vec<u8>, Vec<u8>>`.

---

## 3. Action items (prioritized)

Tagged as **P0** (do first, blocks everything), **P1** (needed for v2 launch), **P2** (nice to have).

### Verification & investigation

- [ ] **P0** Confirm the v1 journal-mode fallback hypothesis by running `examples/sqlite-raw` with `RUST_LOG=rivetkit_sqlite_native::vfs=debug` and grepping for journal-tag writes vs. main-tag writes. We expect mostly journal-tag writes during the 1 MiB insert.
- [ ] **P0** Write a small test that artificially exceeds the 128-key limit and confirms SQLite re-issues the transaction through the journal path.
- [ ] **P1** Empirically measure LZ4 compression ratio on actual SQLite pages from a real workload (not synthetic). The sub-agent results will inform the frame-sizing constant.

### Protocol & engine work

- [ ] **P0** Bump runner-protocol to a new schema version and define the `kv_sqlite_*` op family per §2.2 above.
- [ ] **P0** Implement the engine-side handlers in `engine/packages/pegboard/src/actor_kv/sqlite.rs` (new file). Each is one `db.run` closure with CAS + writes.
- [ ] **P0** Wire napi bindings on `EnvoyHandle` for the new ops.
- [ ] **P0** Add the methods to the `SqliteKv` trait and implement them in `EnvoyKv`.

### v2 VFS implementation

- [ ] **P0** New file `rivetkit-typescript/packages/sqlite-native/src/vfs_v2.rs` that registers under a separate VFS name and implements the v2 design. Keep v1 untouched.
- [ ] **P0** Schema-version dispatch: add a probe at registration time that reads the first key in the actor's subspace to determine v1 vs v2. New actors use v2 by default behind a config flag.
- [ ] **P1** Port the mvSQLite prefetch predictor (Apache-2.0, attribution required) to `vfs_v2.rs` as the read-side optimizer.
- [ ] **P1** Implement the in-memory page cache (LRU, configurable size, default 5,000 pages).
- [ ] **P1** Implement `dirty_pgnos_in_log` with a read-write lock so reads are consistent with materializer updates.
- [ ] **P1** Implement the four-layer read path with the LOG-miss retry-against-fresh-state fallback.
- [ ] **P1** Implement the write path: BEGIN/COMMIT_ATOMIC_WRITE, fast path (1 round trip), slow path (Phase 1 stages + Phase 2 commit).
- [ ] **P1** Implement the background materializer task with budget-bounded passes and back-pressure on the writer.
- [ ] **P1** Implement preload hints (configurable per-actor list of keys/ranges to preload).
- [ ] **P2** Add VFS metrics for cache hit rate, prefetch effectiveness, materializer lag, log size.

### Testing

- [ ] **P0** Build the in-memory `SqliteKv` test driver: deterministic, supports failure injection (return errors after N ops, simulate fencing failures, simulate partial writes).
- [ ] **P0** Build the preload-aware test harness so test cases can declare initial KV state and expected post-conditions.
- [ ] **P1** Port the existing v1 driver test suite to also run against v2 (the SQLite engine should be indistinguishable).
- [ ] **P1** Add v2-specific tests for: orphan cleanup on startup, generation fencing, materializer correctness under churn, preload hint behavior, large-transaction slow-path round-trip count.
- [ ] **P1** Extend `examples/sqlite-raw/BENCH_RESULTS.md` with a v2 column for direct comparison.

### Drop / explicitly out of scope

- [x] **DROPPED** Rolling LTX checksum maintenance — see §1.2.
- [x] **DROPPED** Migration from v1 to v2 — see §1.5.
- [x] **DROPPED** `journal_mode = MEMORY` / `synchronous = OFF` — see §1.6.
- [x] **DROPPED** VACUUM support — declare unsupported in v2.0.

---

## 4. Open questions for the parallel workload sub-agents

Three sub-agents are running in parallel against this design, evaluating:

1. **Large reads** — workloads that scan many pages (reporting queries, full-table scans). How does the prefetch predictor perform? What's the round-trip count vs. v1?
2. **Aggregations** — `count(*)`, `avg()`, `sum()`. Same as large reads but with a different access pattern (sequential page scan + small result set).
3. **Point reads and point writes** — typical OLTP. How does v2's commit path compare to v1's atomic-write path for a 4-page commit? How does the materializer cost amortize?

Their findings will land in `workload-analysis.md` in this folder.

A fourth sub-agent is designing the test architecture, including the in-memory KV driver and the preload-aware harness. Findings in `test-architecture.md`.

---

## 5. Outstanding design questions

- **Exact frame size constant.** Need the LZ4 compression ratio measurement from §3 above before fixing this.
- **Materializer back-pressure threshold.** What fraction of the 10 GiB quota can LOG/ consume before we start blocking the writer? Probably bounded by absolute size (e.g., 200 MiB) rather than a quota fraction, but TBD.
- **Preload hint API.** Should it be config-time only, or can the actor add hints at runtime? Leaning toward config-time + per-action override.
- **Cache size default.** mvSQLite uses 5,000 pages = 20 MiB per connection. Is that too much for our actor density? Probably make it configurable with a smaller default (e.g., 1,000 pages = 4 MiB).
- **What happens to the existing `BENCH_RESULTS.md` numbers when v2 lands?** Keep v1 numbers as a baseline column, add v2 alongside. Don't overwrite.

---

## 6. Update log

- **2026-04-15** — Initial decisions log. Reverted the pragma changes, dropped the rolling checksum, locked in the no-migration policy, sketched the `kv_sqlite_*` op family, ordered the action items.

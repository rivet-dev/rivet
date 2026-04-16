# SQLite VFS v2 — Engine-Side Compaction Subsystem

> **Read [`constraints.md`](./constraints.md) first.** This document is downstream of C0–C8 and the Option D architecture decision (sharded LTX + delta log).

> **Status (2026-04-15):** Design. Unimplemented. Runs inside the engine process, not the actor. Treats every SQLite page as an opaque 4 KiB blob.

Companion docs: [`constraints.md`](./constraints.md) (SHARD + DELTA rationale), [`protocol-and-vfs.md`](./protocol-and-vfs.md) (the `sqlite_*` op family and actor-side VFS).

---

## 0. Summary

This document specifies the **engine-side compaction subsystem** for the v2 SQLite VFS. Compaction runs inside the engine process. The actor never touches it directly — the actor sends `sqlite_get_pages` / `sqlite_commit` / `sqlite_takeover` ops and the engine handles the sharded storage layout internally. Compaction folds LTX deltas (`DELTA/<txid>`) into shards (`SHARD/<shard_id>`) so that:

1. `DELTA/` never grows without bound.
2. Cold reads hit one shard fetch (plus, worst case, one delta fetch for unfolded pages).
3. The actor's 10 GiB KV quota is not blown by accumulated deltas.

The design is byte-level — no SQLite linking, no SQL parsing, no page-format awareness. Pages are opaque 4 KiB blobs, merged by latest-txid-wins. The only format dependency is the `litetx` Rust crate (crates.io, Apache-2.0) for LTX encoding and decoding. Compaction operates only under the `v2/` key prefix, preserving C8 v1/v2 separation structurally.

---

## 1. Storage layout

Scoped under `keys::actor_kv::subspace(actor_id)` (see `engine/packages/pegboard/src/keys/actor_kv.rs`). Schema-version byte `0x02` prefixes everything:

```
v2/META                       → DBHead { generation, head_txid, materialized_txid,
                                         db_size_pages, next_txid, ... }
v2/SHARD/<shard_id_be32>      → LTX blob for pages [shard_id*S .. (shard_id+1)*S),
                                S = 64 (working default)
v2/DELTA/<txid_be64>          → LTX blob for pages dirtied by one committed tx
v2/DELTAREF/<txid_be64>       → i64 remaining-unfolded-pages refcount (§4.4)
v2/PIDX/delta/<pgno_be32>     → txid_be64 — sparse "which delta holds the
                                freshest copy of pgno" index (§3)
```

`shard_id = pgno / S` — computational, no key needed. All delta and shard blobs are LZ4-compressed LTX per `constraints.md`.

---

## 2. Trigger policy

### 2.1 When compaction runs

A pass fires when any of the following holds for an actor:

1. **Delta count threshold** — `N_count = 64` unfolded deltas. Bounds worst-case page-index scan.
2. **Delta byte threshold** — `B_soft = 16 MiB` compressed aggregate. 0.16% of the 10 GiB quota — plenty of runway.
3. **Idle timer** — ≥ 8 deltas present and no writes for `T_idle = 5 s`. Amortizes cost in quiet windows.
4. **Back-pressure floor** — aggregate > `B_hard = 200 MiB`. The engine stops accepting new commits until compaction drains. Last-resort safety valve.
5. **Startup recovery** — ≥ `N_recovery = 32` deltas present at takeover triggers an immediate pass.

All thresholds are per-actor configurable via preload metadata (same mechanism as `PreloadConfig` in `preload.rs:26`).

### 2.2 Event-driven, not polling

Every `kv_sqlite_commit` handler updates a per-actor `DeltaStats { count, total_bytes, last_commit_ts_ms }` kept in an `scc::HashMap<Id, Arc<DeltaStats>>` (cost: ~100 ns per commit). When the commit handler detects `count >= N_count` or `total_bytes >= B_soft` after its own write, it pushes the `ActorId` into the per-host scheduler's `mpsc` queue. An idle-scan task re-checks stats every second and fires idle-triggered compactions. Polling is never needed — zero wasted work for idle actors.

### 2.3 The commit path never blocks on compaction

Compaction runs entirely inside the engine. It pays **zero** actor↔engine RTTs (C6 is the 20 ms RTT; the UDB tx latency is a different, smaller cost). The commit handler fires the scheduler event *after* its UDB tx returns success, so commits never wait on compaction. Only rule 4 (hard back-pressure) actually blocks writers, and only when the 200 MiB quota is blown.

### 2.4 Per-actor work, globally coordinated

Scheduling is a per-host worker pool shared across actors. Each actor's compaction is serialized (one pass at a time per actor), matching C5 single-writer semantics. An `scc::HashSet<Id>` tracks `in_flight` actors to prevent double-scheduling.

---

## 3. The page index

The engine must answer: "for page P, where is the latest version — in a delta or in shard `P/S`?" Scanning all deltas per read is O(N) per cold read and degenerates at N > 16.

### 3.1 Persistent sparse index with in-memory cache

```
v2/PIDX/delta/<pgno_be32>  →  txid_be64
```

Each entry means: "page `pgno` currently has its freshest unfolded copy in `DELTA/<txid>`." Only pages currently unfolded consume an entry. Pages without an entry are served straight from their shard.

This is **sparse**. A 10 GiB (2.6M page) actor with 5,000 pages dirtied across the last 64 commits has ~5,000 entries, not 2.6M.

The engine caches this index in an in-memory `scc::HashMap<u32, u64>` per actor, loaded lazily on first access from a single prefix scan of `v2/PIDX/delta/` (one UDB tx, the same shape as `batch_preload` in `preload.rs:53`). Updates happen synchronously with commit and compaction handlers.

**Why this over alternatives:**

| Option | Memory | Cold start | Correctness | Verdict |
|---|---|---|---|---|
| (a) In-memory only, rebuild from delta LTX headers | ~sparse | Expensive: scan every `DELTA/<txid>` | OK | Cold-start hit unacceptable |
| (b) Persistent only, no cache | 0 heap | Free | OK | Extra UDB read per cold page read |
| (c) No index, scan all deltas | 0 | Free | OK | Degenerate at N > 16 |
| (d) **Persistent + in-memory cache** | ~sparse | One prefix scan | Atomic under tx | **Chosen** |

### 3.2 Memory budget at scale

At `N_count = 64` deltas with ~10 dirty pgnos per delta, ~640 pages per actor × 16 bytes (pgno + txid + scc overhead) = ~10 KiB heap per active actor. 10,000 active actors per host = ~100 MiB. Affordable.

A full 10 GiB actor database is 2.6M pages × 8 bytes = 20 MB *if dense*, but the index is sparse by construction. Dense-index fallback is never invoked.

### 3.3 Structure sketch

```rust
// engine/packages/pegboard/src/actor_kv/sqlite/page_index.rs
pub struct DeltaPageIndex {
    entries: scc::HashMap<u32, u64>, // pgno → txid
    loaded: AtomicBool,
}

impl DeltaPageIndex {
    pub async fn ensure_loaded(&self, db: &Database, actor_id: Id) -> Result<()>;
    pub async fn lookup(&self, pgno: u32) -> Option<u64>;
    pub async fn apply_commit(&self, txid: u64, pgnos: &[u32]);
    pub async fn apply_compaction(&self, folded_pgnos: &[u32]);
}
```

`scc::HashMap` is specifically required per the CLAUDE.md performance guideline (no `Mutex<HashMap>`). Its async methods do not hold locks across `.await`.

### 3.4 Atomicity of updates

The critical invariant: persistent `PIDX/delta/*` state must never disagree with `DELTA/*` and `SHARD/*` state, because the read path trusts the index.

- **Commit tx** writes `DELTA/<txid>`, writes `PIDX/delta/<pgno> = txid` for every pgno (overwriting previous), writes `DELTAREF/<txid> = num_pgnos`, updates `META`. All in one `db.run(|tx| ...)` closure.
- **Compaction tx** writes new `SHARD/<shard_id>`, deletes consumed `PIDX/delta/<pgno>` entries for folded pgnos, decrements `DELTAREF` atomically, deletes any `DELTA` whose refcount hit 0, updates `META`. All in one closure.

The in-memory mirror is mutated **after** the tx succeeds. If the engine crashes mid-update of memory, the next access rebuilds it from `PIDX/delta/*`. Persistent state is the source of truth.

### 3.5 Read path (`sqlite_get_pages`)

```
For each pgno in request:
  if DeltaPageIndex.lookup(pgno) = Some(txid) → fetch DELTA/<txid>
  else                                        → fetch SHARD/<pgno/S>
Batch fetches by key into one UDB tx.
LTX-decode, return pages in one response envelope (~9 MiB limit).
```

UDB's tx isolation gives us a snapshot-consistent view of the index and storage in one read op — compaction cannot rearrange storage underneath a single read.

---

## 4. The compaction step itself

### 4.1 Unit of work: one shard per pass

Each pass folds **all unfolded deltas that touch a single target shard** into a new version of that shard. A delta that touches 3 shards (5, 7, 42) is consumed across 3 passes. Delta deletion is refcount-gated (§4.4).

Why one-shard-at-a-time:

- **Bounded tx size.** One shard write (~128 KiB compressed) + a handful of index/META/refcount updates. Well under the 9 MiB envelope. Well under the 5 s UDB tx timeout (`transaction.rs:18`).
- **Composable failure.** Crash mid-batch means some shards got folded and others didn't. The next pass picks up from unchanged `META.materialized_txid` state.
- **Predictable cost.** A large commit that dirties 80 shards becomes 80 ~50 ms passes instead of one 4 s megatx pushing against the deadline.

The "fold every affected shard in one giant tx" alternative is rejected on tx-size grounds.

### 4.2 Pass sequence

```rust
// engine/packages/pegboard/src/actor_kv/sqlite/compaction.rs
pub async fn compact_shard(
    db: &universaldb::Database,
    actor_id: Id,
    expected_generation: u64,
    target_shard_id: u32,
) -> Result<CompactionPassReport> {
    db.run(|tx| async move {
        let tx = tx.with_subspace(v2_subspace(actor_id));

        // 1. Read META, CAS on generation.
        let head: DBHead = tx.read(&meta_key(), Serializable).await?;
        if head.generation != expected_generation {
            bail!(KvSqliteFenceMismatch { ... });
        }

        // 2. Read current shard (may be empty).
        let old_shard = tx.informal().get(&shard_key(target_shard_id), Serializable).await?;

        // 3. Scan PIDX/delta/ restricted to this shard's pgno range.
        //    Group by txid.
        let pidx_range = pidx_delta_range(
            target_shard_id * SHARD_PAGES,
            (target_shard_id + 1) * SHARD_PAGES,
        );
        let mut delta_pgnos: BTreeMap<u64, Vec<u32>> = BTreeMap::new();
        let mut stream = tx.get_ranges_keyvalues(pidx_range.into(), Serializable);
        while let Some(kv) = stream.try_next().await? {
            let (pgno, txid) = parse_pidx_delta(kv)?;
            delta_pgnos.entry(txid).or_default().push(pgno);
        }
        if delta_pgnos.is_empty() { return Ok(NoWork); }

        // 4. Batch-fetch all referenced DELTA blobs.
        let mut blobs: BTreeMap<u64, Vec<u8>> = BTreeMap::new();
        for &txid in delta_pgnos.keys() {
            let blob = tx.informal().get(&delta_key(txid), Serializable).await?
                .context("delta referenced by index but missing")?;
            blobs.insert(txid, blob.to_vec());
        }

        // 5. Decode old shard + deltas (ascending txid). Merge latest-wins.
        let mut merged: BTreeMap<u32, Vec<u8>> = BTreeMap::new();
        if let Some(b) = old_shard { litetx::decode_into(&b, &mut merged)?; }
        for (_, b) in &blobs { litetx::decode_into(b, &mut merged)?; }
        merged.retain(|&pgno, _| pgno / SHARD_PAGES == target_shard_id);

        // 6. Encode new shard, write it.
        let new_shard_bytes = litetx::encode_shard(target_shard_id, &merged)?;
        tx.informal().set(&shard_key(target_shard_id), &new_shard_bytes);

        // 7. Clear PIDX/delta/<pgno> for folded pgnos.
        for pgno in merged.keys() {
            tx.informal().clear(&pidx_delta_key(*pgno));
        }

        // 8. Decrement DELTAREF/<txid> atomically for each delta, by the
        //    number of pgnos this pass consumed from it.
        for (&txid, pgnos) in &delta_pgnos {
            tx.informal().atomic_op(
                &delta_refcount_key(txid),
                &(-(pgnos.len() as i64)).to_le_bytes(),
                MutationType::Add,
            );
        }

        // 9. Re-read refcount; delete DELTA/<txid> + DELTAREF/<txid> if 0.
        for &txid in delta_pgnos.keys() {
            let rc: i64 = tx.read(&delta_refcount_key(txid), Serializable).await?;
            if rc == 0 {
                tx.informal().clear(&delta_key(txid));
                tx.informal().clear(&delta_refcount_key(txid));
            }
        }

        // 10. Advance META.materialized_txid past fully-consumed deltas,
        //     write new META.
        let new_head = DBHead {
            materialized_txid: compute_new_mat_txid(head, &delta_pgnos),
            last_compaction_ts_ms: util::timestamp::now(),
            ..head
        };
        tx.write(&meta_key(), new_head)?;
        Ok(Folded { shard_id: target_shard_id, pages: merged.len(), deltas: delta_pgnos.len() })
    })
    .custom_instrument(tracing::info_span!("sqlite_compact_shard_tx"))
    .await
}
```

Pattern mirrors `actor_kv::put` (`mod.rs:283`) — one `db.run` closure, CAS read, writes, atomic commit.

### 4.3 Budget per pass

Conservative cost of one pass at 64-page shards with 8 touching deltas, ~5 KiB each:

| Step | Cost |
|---|---|
| Read old shard (~128 KiB) | ~1 ms UDB read |
| Read 8 deltas (~40 KiB total) | ~2 ms (batched) |
| LZ4 decode ~200 KiB | ~200 µs CPU |
| BTreeMap merge ~50 pgnos | µs |
| LZ4 encode ~256 KiB → ~128 KiB | ~500 µs CPU |
| Write shard + refcount + META | ~2 ms UDB write |

**Total: ~5 ms per pass**, ~700 µs CPU. One modern core sustains ~1,400 passes/sec.

### 4.4 Multi-shard delta refcounting

`DELTAREF/<txid>` is initialized to `num_pgnos` by the commit handler, decremented atomically by each compaction pass via `MutationType::Add`. When it hits 0 the delta is deleted.

Rejected alternatives: (a) multi-shard megatx (violates bounded-tx invariant in §4.1), (b) per-delta bitmap (same information, harder to reason about).

The rely on `MutationType::Add` is worth verifying — we re-read inside the same tx to decide deletion, which must observe the post-add value under Serializable isolation. Noted as pending validation in §8.

### 4.5 Failure model

Compaction is **idempotent at the pass granularity**. A crash before the tx commits leaves the previous state intact — the next pass reads the same META, does the same merge, writes the same shard (identical bytes, even). A crash after tx commit leaves persistent state fully consistent; the in-memory mirror rebuilds on next access.

A pass whose `merged` matches the existing shard exactly (possible after a spurious retry) is a harmless no-op. No fingerprinting needed.

---

## 5. Concurrency with writers

### 5.1 Shared META

Commit advances `head_txid`, compaction advances `materialized_txid`, both CAS on `generation`. Inside their UDB txs they both do read-then-CAS on META and UDB's optimistic concurrency control serializes them:

- Compaction tx commits first → commit tx retries, sees advanced `materialized_txid`, writes new delta on top. Clean.
- Commit tx commits first → compaction tx retries (via `db.run`'s retry loop), may or may not include the new delta depending on whether it touches the target shard. Clean.

### 5.2 Reads during compaction

`sqlite_get_pages` runs in its own UDB tx. Snapshot isolation gives it either the pre-compaction view (PIDX points at DELTA/X, DELTA/X present) or the post-compaction view (PIDX entry cleared, SHARD/Y updated). Never a torn state where both are gone.

### 5.3 Failover race

Two compactors for the same actor **should not happen** under C5. During a brief failover window, they might. Generation CAS defends:

```
old compactor reads META { gen: 7 }
  [failover: new runner calls sqlite_takeover → META { gen: 8 }]
new compactor reads META { gen: 8 }
old compactor tx commits, CAS expected gen=7 → FAILS, work discarded
new compactor tx commits, CAS gen=8 → SUCCEEDS
```

Old compactor's progress is safely lost; new compactor redoes it from scratch.

### 5.4 Back-pressure signaling

The commit handler checks `DeltaStats.total_bytes` before each accept:

- `> B_soft = 100 MiB` → succeed but return a `compaction_pressure: u8` scalar in the response. Actor client libs can use this to self-throttle.
- `> B_hard = 200 MiB` → return `KvSqliteCompactionBackpressure { retry_after_ms }`. Actor blocks its write path until compaction drains.

---

## 6. Scheduling and recovery

### 6.1 Per-host scheduler

One `CompactionScheduler` task per engine process. Holds an `antiox::sync::mpsc::UnboundedChannel<Id>` work queue fed by commit-handler events and the idle-scan ticker. Dispatches to a bounded `tokio::task::JoinSet` of `C = max(2, num_cpus / 2)` workers.

```rust
// engine/packages/pegboard/src/actor_kv/sqlite/scheduler.rs
pub struct CompactionScheduler {
    db: Arc<universaldb::Database>,
    queue: antiox::sync::mpsc::UnboundedChannel<Id>,
    stats: Arc<scc::HashMap<Id, Arc<DeltaStats>>>,
    in_flight: Arc<scc::HashSet<Id>>,
    workers: usize,
}
```

A worker dequeues an actor, verifies `in_flight.insert(actor_id)` (guard against double scheduling), loads META and the page index, computes which shards have unfolded deltas (ordered by unfolded-page count), and runs at most `shards_per_batch = 8` passes before releasing the slot. If the actor still has work it is re-enqueued at the tail for fairness.

### 6.2 Fairness

The 8-shards-per-batch limit prevents a noisy actor from monopolizing workers. `in_flight` serialization prevents parallel consumption of all workers by one actor. A starvation alarm fires if an actor remains above `B_soft` for 30+ s continuously — logged and metric-counted so operators see the pressure.

### 6.3 Lifecycle hooks

- **On takeover** (`kv_sqlite_takeover`): engine bumps generation, schedules a recovery pass (does not block the takeover response).
- **On graceful shutdown**: engine calls `drain_compaction(actor_id).await` which runs passes until 0 unfolded deltas remain. Leaves the next takeover with a clean state.
- **On ungraceful death**: nothing immediately. Next takeover's recovery pass cleans up.

### 6.4 Recovery specifics

Called from the `kv_sqlite_takeover` handler after bumping generation:

```
1. Scan DELTA/<txid> for txids > META.head_txid.
   These are orphans from crashed Phase 1 slow-path commits.
   Delete them plus their DELTAREF and any PIDX entries referencing them.
2. Scan DELTAREF/<txid> for keys whose DELTA/<txid> is missing (leaked trackers).
   Delete them.
3. Prefix-scan PIDX/delta/ into the in-memory DeltaPageIndex.
4. Ack the takeover.
5. Schedule a normal compaction pass if unfolded deltas exceed N_recovery.
```

All steps are idempotent. A crash mid-recovery repeats cleanly.

---

## 7. Performance characteristics

At the C6 operating point (20 ms RTT) with 1000 commits/sec × 10 dirty pages per commit:

- Each commit writes ~5 KiB LZ4 compressed delta → 5 MiB/s delta write rate.
- Compaction trigger fires every 64 commits (at `N_count = 64`) → every 64 ms.
- Touched pgnos per window: ~640, ~200 distinct after hot-page overlap → ~4 shards affected per trigger.

Per trigger: 4 shard passes × 5 ms each = **20 ms compaction work per 64 ms of commit activity** → ~30% of one CPU core on compaction for this workload. One core supports ~22 such hot actors; a 16-core engine host ~350. Well above expected concurrency.

**UDB throughput per actor:** 13 MiB/s write + 28 MiB/s read. At 10 concurrent hot actors that is 130 MiB/s write + 280 MiB/s read — non-trivial and must be verified against the postgres/rocksdb driver's sustained bandwidth.

**Storage amplification:** steady-state ~1.3× (old shard + unfolded deltas coexist until pass runs). Peak during a single tx is hidden by UDB isolation. At `B_hard = 200 MiB` the amplification over a 10 GiB actor is ~1.02× — the quota is essentially unaffected by compaction overhead.

**vs. actor-side materializer (original v2 plan):** that design paid 2 × 20 ms RTT per pass (read + write) through the actor→engine boundary. At 4 passes per window, 160 ms network work vs. ~20 ms engine-local work = **8× win** by moving compaction into the engine, on top of the actor no longer owning the state machine.

---

## 8. Open questions and risks

### 8.1 Needs measurement

- **Exact UDB tx cost for a ~128 KiB shard write** plus a dozen small key ops. The ~5 ms budget assumes postgres/rocksdb handles this in one round trip. If the driver fans out per mutation, the actual pass latency could be 10× higher. Benchmark before committing to `SHARD_PAGES = 64`.
- **Cost of reading 8 small deltas in one tx.** We rely on batched `get` being efficient. Confirm under both drivers.
- **`MutationType::Add` + re-read semantics.** §4.4 relies on reading the post-add value inside the same tx under Serializable isolation. Verify; if it does not hold, compute the post-add value in application code instead.
- **`get_estimated_range_size_bytes` cost per commit.** Used for back-pressure checks on the hot commit path. Must be cheap.
- **`scc::HashMap<u32, u64>` overhead at 6.4M entries across 10k actors.** ~300 MiB heap at scc's typical ~48 bytes/entry overhead. Tolerable but worth profiling. Fallback: per-actor sorted `Vec<(u32,u64)>` with binary search.

### 8.2 Constraint interaction watch list

- **C2 (writes primary):** the commit handler's bookkeeping overhead (DeltaStats update, optional scheduler enqueue) must stay in the tens of microseconds. Regression on commit latency is a deal-breaker.
- **C5 (single writer):** `in_flight` + generation CAS are the defenses. Both must remain load-bearing.
- **C6 (20 ms RTT):** compaction runs engine-local so it does not pay the 20 ms — but UDB tx latency is non-zero and must be measured.
- **C8 (v1/v2 separation):** all compaction keys are under `v2/`. Schema-version byte in `DBHead` is a second guard.

### 8.3 `litetx` crate dependency

We need: decode standalone LTX blob → (pgno, bytes), encode (pgno, bytes) → blob, zero the PostApplyChecksum field (we do not maintain it, per `design-decisions.md` §1.2), LZ4-compressed page bodies. Audit the crate against this list. If anything is missing: either contribute upstream or fork. Worst case, hand-roll a byte-level encoder (~200 lines).

### 8.4 Actor-side LRU cache interaction

Compaction changes where pages live in storage but never changes their content. The actor's LRU page cache holds page bytes, not storage locations. Compaction is transparent to it — no invalidation needed. A cached page served from the actor's LRU simply never reaches the engine. A cache miss after compaction fetches from the new location, getting identical bytes. No subtlety.

One caveat: a multi-page `sqlite_get_pages` op runs in a single UDB tx with snapshot isolation. A compaction pass cannot tear the storage layout under an in-flight read — UDB guarantees it.

---

## 9. Files to create

```
engine/packages/pegboard/src/actor_kv/sqlite/mod.rs            — module root
engine/packages/pegboard/src/actor_kv/sqlite/commit.rs         — kv_sqlite_commit handler
engine/packages/pegboard/src/actor_kv/sqlite/commit_stage.rs   — Phase 1 for slow-path commits
engine/packages/pegboard/src/actor_kv/sqlite/get_pages.rs      — kv_sqlite_get_pages handler
engine/packages/pegboard/src/actor_kv/sqlite/preload.rs        — kv_sqlite_preload handler
engine/packages/pegboard/src/actor_kv/sqlite/takeover.rs       — kv_sqlite_takeover + recovery
engine/packages/pegboard/src/actor_kv/sqlite/compaction.rs     — compact_shard
engine/packages/pegboard/src/actor_kv/sqlite/scheduler.rs      — CompactionScheduler + worker pool
engine/packages/pegboard/src/actor_kv/sqlite/page_index.rs     — DeltaPageIndex
engine/packages/pegboard/src/actor_kv/sqlite/delta_stats.rs    — DeltaStats bookkeeping
engine/packages/pegboard/src/actor_kv/sqlite/keys.rs           — SHARD, DELTA, PIDX, DELTAREF key types
engine/packages/pegboard/src/actor_kv/sqlite/ltx.rs            — litetx wrapper helpers
engine/packages/pegboard/src/actor_kv/sqlite/errors.rs         — KvSqlite*Error variants
engine/packages/pegboard/src/actor_kv/sqlite/metrics.rs        — compaction counters, lag gauges
```

Plus a workspace `Cargo.toml` dependency on `litetx = "<version>"` — add to `[workspace.dependencies]` and pull into `engine/packages/pegboard/Cargo.toml` with `litetx.workspace = true` per the dependency convention in CLAUDE.md.

---

## 10. Decisions still pending

- [ ] Confirm `SHARD_PAGES = 64` empirically via `examples/sqlite-raw` running against `vfs_v2`. If point reads over-fetch, shrink. If sequential workloads pay too many round trips, grow.
- [ ] Confirm thresholds `N_count = 64`, `B_soft = 16 MiB`, `B_hard = 200 MiB`, `T_idle = 5 s` empirically. Likely per-workload and per-actor configurable.
- [ ] Confirm `shards_per_batch = 8` fairness budget via load testing.
- [ ] Decide which thresholds are per-actor vs. per-engine. Current lean: `N_count` and `T_idle` per-actor, `B_hard` per-engine.
- [ ] Audit `litetx` crate API against §8.3 requirements. File upstream PRs or fork if missing.
- [ ] Decide whether to share the `estimate_kv_size` helper with `actor_kv/mod.rs:283` or duplicate for the compressed-delta accounting case.
- [ ] Finalize metric names and export them through `engine/packages/pegboard/src/metrics.rs`.
- [ ] `MutationType::Add` + same-tx re-read semantics under Serializable — verify or rework §4.4.
- [ ] Confirm `SHARD_PAGES` is a permanent format constant (no resharding primitive in v2.0).
- [ ] Design interaction between hard back-pressure (`B_hard`) and the actor's SQLite layer — the actor must surface the error so application code can retry gracefully rather than hanging.

---

## 11. Relationship to the rest of v2

This document covers compaction only. The `kv_sqlite_*` runner-protocol ops and schema-version bump are in `design-decisions.md` §2. The actor-side VFS (how pages reach `DELTA/` in the first place) is in `walkthrough.md` Chapters 4–6 (substitute SHARD/DELTA for LOG/PAGE). The in-memory actor LRU and prefetch predictor are independent and unchanged by anything here. Test architecture (in-memory engine driver with failure injection) is §3 of `design-decisions.md`'s action list.

The P0 ordering from `design-decisions.md` §3 still applies: runner-protocol bump + engine-side op handlers (including compaction) before any actor-side VFS work.

---

## 12. Update log

- **2026-04-15** — Initial draft. Trigger policy, page index, single-shard compaction unit, fencing, scheduling, recovery, performance math, open questions. Pending: UDB tx-cost validation and `litetx` API audit.

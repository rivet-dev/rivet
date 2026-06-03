# SQLite Storage Concurrency Design

A coherent concurrency story for `engine/packages/sqlite-storage/` that survives the three adversarial reviews of `sqlite-concurrency-cleanup.md`.

The previous proposal had the right *intuition* (most invariants are already enforced by FDB serializability, leases were defending against efficiency-only duplication) but the wrong *grounding* (most "Serializable" reads in code today are actually `Snapshot`, the SHARD blob exceeds FDB's 100 KB value limit, eviction "single-tx" is two txs in code, and several pre-existing bugs were swept up into the cleanup). This design fixes those holes.

## High-level position

- Keep the **eviction global lease** and the **cold compactor lease**. Drop the **hot compactor lease**. This is a smaller win than "no leases anywhere" but it survives the reviews.
- Convert every read that the design relies on for cross-tx invariants from `Snapshot` to `Serializable`. Today, most relevant reads are Snapshot, which silently breaks the proposal's serializability claims.
- Fix two pre-existing bugs that block the cleanup: SHARD blob chunking (>100 KB violates FDB) and hot fold deleting DELTAs without checking `desc_pin`.
- Bound multi-tx eviction to a single FDB tx **per branch** (not per sweep) — cleanly within the 5 s / 10 MB budgets.
- Preserve operational primitives: `pending/{uuid}.marker` becomes the durable forensic trail; structured logs key on `pass_uuid` + `pod_id`; backoff and retry budgets are explicit.

---

## 1. Inventory of every concurrency-touching operation

### 1.1 Commit (`pump/commit.rs::Db::commit`)

| Field | Detail |
|---|---|
| Reads (Serializable) | `branches_list/{branch_id}` (via `resolve_or_allocate_branch`); `/META/head` (`commit.rs:86`, `92`); `/META/head_at_fork` (`commit.rs:87`, `93`); `quota::read_branch` |
| Reads (Snapshot) | `/META/compact` (`commit.rs:105`); `burst_mode` branch signal (`commit.rs:206-211`); namespace pointer (lazy resolve, currently `Serializable`) |
| Writes | `set` PIDX/{pgno} (per dirty page); `set` DELTA/{txid}/{chunk_idx} (per chunk); `set` /META/head; `clear` /META/head_at_fork (one-shot); `SetVersionstampedValue` COMMITS/{txid}; `SetVersionstampedKey` VTX/{vs}; `Add` /META/quota delta; `Add` access-touch counter |
| Tx structure | Single FDB tx |
| S3 | None |
| NATS | Publishes `compact_payload` after commit returns (best-effort trigger, no correctness role) |
| Worst-case write set | Bounded by SQLite max-page-cache-on-conn (~MB scale) plus PIDX rows. Well under 10 MB for normal commits |
| Per-value | DELTA chunks are 10 KB (`DELTA_CHUNK_BYTES`), well under 100 KB. PIDX values are 8 bytes. /META/head is small |

### 1.2 Hot compactor (`compactor/compact.rs`)

Two FDB txs per pass: plan, then write. Multi-shard databases write all shards in a *single* write tx.

| Field | Detail |
|---|---|
| plan_batch reads (Snapshot — `compact.rs:132`,`133`,`138`) | scope; `/META/head`; `/META/compact`; PIDX rows; DELTA blobs (full body for selected pages) |
| write_batch reads (Serializable — `compact.rs:236`) | `/META/head` |
| write_batch reads (Serializable inside `enforce_shard_version_cap`) | shard versions list (`compact.rs:468`); pin txids (`compact.rs:506`,`519`); commit rows for hot retention sweep (`compact.rs:540`,`552`) |
| Writes | `set` SHARD/{shard_id}/{as_of_txid} (**violates 100 KB limit at full shard, see §2/§6**); `clear_range` DELTA chunk ranges (one range per selected delta); `compare_and_clear` PIDX/{pgno}; `set` /META/compact; `set` /META/manifest/last_hot_pass_txid; `clear` old SHARD version (oldest unpinned, when capped); `clear` retention-window-old COMMITS+VTX rows; `Add` /META/quota delta |
| Tx structure | Plan tx (Snapshot) → write tx (Serializable head + Serializable cap) — both fit in 5 s normally |
| Worst-case write set | Bounded by `batch_size_deltas` * 64 pages * 4 KB ≈ ~1 MB plus PIDX deletes. Within 10 MB |
| Per-value | SHARD blob can hit ~256 KB. **Pre-existing FDB violation.** |

### 1.3 Cold compactor — Phase A (`compactor/cold/phase_a.rs`)

| Field | Detail |
|---|---|
| Reads (Serializable, `register_pending_handoff`) | `/META/cold_compact`; `/META/compact`; `/META/manifest/last_hot_pass_txid`; `/META/manifest/cold_drained_txid` |
| Reads (Snapshot, `read_snapshot_plan` lines 308,314,322; `scan_prefix` line 512) | `branches_list/{branch_id}`; `/META/compact`; `/META/manifest/last_hot_pass_txid`; SHARD prefix scan; DELTA prefix scan; COMMITS prefix scan; VTX prefix scan |
| Writes | `set` /META/cold_compact (with `in_flight_uuid`) |
| Tx structure | Two FDB txs: handoff tx (Serializable) writes `in_flight_uuid`; snapshot tx (Snapshot) reads everything else |
| S3 | `put_object(pending/{uuid}.marker, marker)` between the two txs |
| Worst-case read set (snapshot tx) | Multi-MB DELTA chunks + 32 SHARD versions + thousands of COMMITS — risks 5 s tx-age timeout per FDB review 1.3 |

### 1.4 Cold compactor — Phase B (`compactor/cold/phase_b.rs`)

| Field | Detail |
|---|---|
| Reads | None (input is the `ColdPhaseAPlan`) |
| Writes | None (no FDB activity) |
| S3 | `put_object` for image, delta, pin, manifest chunks, catalog snapshot, pointer snapshot, pending marker rewrite. Filenames are deterministic from `(branch_id, pass_uuid, versionstamp)` |

### 1.5 Cold compactor — Phase C (`compactor/cold/phase_c.rs`)

| Field | Detail |
|---|---|
| Reads (Serializable) | `/META/cold_compact` (`phase_c.rs:90`); `BOOKMARK/.../pinned` per uploaded pin (`phase_c.rs:108`,`158`) |
| Writes | `set` /META/cold_compact (clears `in_flight_uuid`, advances `cold_drained_txid`); `set` /META/manifest/cold_drained_txid; `set` BOOKMARK/.../pinned for each transitioned pin (Pending → Ready or Failed) |
| Tx structure | Single FDB tx |
| Manual fence | Re-reads `cold_drained_txid` via Serializable and aborts if it differs from `state_before.cold_drained_txid` (`phase_c.rs:46-53`). This is the OCC fence the cleanup wanted to drop. **Keep it but reframe:** see §4 |

### 1.6 Eviction compactor (`compactor/eviction/mod.rs`)

| Field | Detail |
|---|---|
| Lease tx (Serializable, `take_global_lease`) | reads `CMPC/lease_global/{eviction}`; writes the lease |
| Plan tx — outer (Snapshot, `scan_eviction_index` line 269) | scans `CTR/eviction_index` prefix |
| Plan tx — inner (`plan_evictable_shard_versions`) | reads (Serializable) `cold_drained_txid`, `last_hot_pass_txid`, `desc_pin`, `bk_pin` (lines 303-318); reads (Snapshot) SHARD versions (line 563); reads (Snapshot) PIDX rows (line ~600) |
| Clear tx (Serializable inside the closure) | re-reads `last_hot_pass_txid` (line 381); re-reads pins (line 444-448); writes `compare_and_clear` against PIDX + SHARD; clears fully-evicted index entries |
| Tx structure | Three FDB txs: lease, plan, clear (plus release-lease tx). FDB review correctly flags this as not "single-tx" |
| Worst-case write set | At `batch_size=256` branches × 32 versions × 64 PIDX, easily exceeds 10 MB. Plus the SHARD `expected_value` for `compare_and_clear` carries the full 256 KB blob — read-conflict bytes balloon. **Must be bounded.** See §6 |

### 1.7 Fork database (`pump/branch.rs::fork_database` / `derive_branch_at`)

| Field | Detail |
|---|---|
| Reads (Serializable, `derive_branch_at`) | source `branches_list/{source}` (via `read_database_branch_record`); `bk_pin/{source}` (`branch.rs:551`, via `read_versionstamp_pin` Serializable line 876); VTX/{at_versionstamp}; COMMITS/{txid_at_versionstamp} |
| Reads (Serializable for namespace resolution) | namespace pointer + branch record |
| Writes | `set` `/META/head_at_fork/{new_branch_id}`; `set` `branches_list/{new_branch_id}`; `set` database_pointer for new database id; `Add` refcount/{source} (+1) and refcount/{new} (+1); `ByteMin` desc_pin/{source}; `set` NSCAT entry |
| Tx structure | Single FDB tx |
| NATS | Publishes `ForkWarmup` after commit returns |

### 1.8 Fork namespace (`pump/branch.rs::fork_namespace` / `derive_namespace_branch_at`)

Same shape as fork_database but on namespace-scoped keys. `bk_pin` read on namespace branch (line 662) is Serializable. Single FDB tx.

### 1.9 Pinned bookmark create (`pump/bookmark.rs::create_pinned_bookmark`)

| Field | Detail |
|---|---|
| Reads (Serializable) | namespace branch resolve (line 168); database branch resolve (line 171); `/META/head` (line 176); COMMITS/{head_txid} (line 182); existing BOOKMARK pinned record (line 190); `pin_count` (line 194 — **Serializable, into conflict set**) |
| Writes | `set` BOOKMARK/{db_id}/{bookmark}/pinned (Pending state); `Add` pin_count (+1); `ByteMin` bk_pin |
| Tx structure | Single FDB tx |
| NATS | Publishes `CreatePinnedBookmark` after commit |

### 1.10 Pinned bookmark delete (`pump/bookmark.rs::delete_pinned_bookmark`)

| Field | Detail |
|---|---|
| Reads (Serializable) | BOOKMARK/{...}/pinned (line 278); namespace branch resolve (line 284); all sibling BOOKMARK records (via `recompute_database_branch_bk_pin`) |
| Writes | `clear` bookmark + pinned keys; `Add` pin_count (−1); `set` recomputed bk_pin/{branch_id} (plain set, not atomic-min — overwrites) |
| Tx structure | Single FDB tx |
| NATS | Publishes `DeletePinnedBookmark` after commit |

### 1.11 GC pass per-branch (`gc/mod.rs`)

Two callable entry points: `sweep_branch_hot_history` (delete COMMITS/VTX/DELTA below `gc_pin`) and `sweep_unreferenced_branch` (delete entire branch when refcount=0 + no pins).

| Field | Detail |
|---|---|
| Reads (Serializable, `read_branch_gc_pin_tx` lines 73, 215, 230, 245) | `branches_list/{branch_id}`; refcount; `desc_pin`; `bk_pin` |
| Reads (Snapshot, `scan_prefix` lines 118, 128, 138, 174) | COMMITS prefix; VTX prefix; DELTA prefix; full `branch_prefix` for branch deletion |
| Writes | `clear` per matching key |
| Tx structure | Single FDB tx per call (caller batches across branches) |
| Worst-case write set | Many cleared keys per long-stale branch; needs batching by caller |

### 1.12 Delete database / delete namespace (`pump/branch.rs::delete_database`)

| Field | Detail |
|---|---|
| Reads (Serializable) | namespace branch resolve; visibility check |
| Writes | `SetVersionstampedValue` namespace tombstone; `Add` refcount/{branch_id} (−1) |
| Tx structure | Single FDB tx |
| Note | Actual key cleanup is GC's job, not this op's |

---

## 2. FDB primitive constraints — grounded in code

Every constraint below cites the FDB documentation behavior plus its sole or primary call site in this crate.

| # | Constraint | Where it bites |
|---|---|---|
| F1 | Tx wall-clock budget = 5 s from `start_read_version` to commit (`universaldb/src/transaction.rs:18` `TXN_TIMEOUT`) | Cold Phase A snapshot tx (`phase_a.rs:223-236` already wraps in `tokio::time::timeout(phase_a_read_timeout_ms)`). Eviction plan tx scanning `batch_size=256` branches with full SHARD blobs |
| F2 | 10 MB total write set per tx | Eviction clear tx with all `compare_and_clear` calls carrying SHARD `expected_value` (`eviction/mod.rs:400-410`). The `expected_value` enters the conflict-range size accounting in real FDB |
| F3 | 100 KB per-value limit | SHARD blob write at `compactor/shard.rs:119` (single `tx.informal().set(&key, &encoded)`). Full shard = 64 pages × 4 KB = 256 KB plus LTX header |
| F4 | Reads issued via the FDB driver are **Serializable by default**; `Snapshot` is opt-in. `Snapshot` reads do **not** enter the conflict set | All `IsolationLevel::Snapshot` call sites listed in §1: cold Phase A snapshot tx, eviction plan, hot compactor planning, GC scan_prefix. Their not-in-conflict-set behavior is the central correctness gap |
| F5 | Atomic `Add`, `ByteMin`, `ByteMax`, `SetVersionstampedKey`, `SetVersionstampedValue` are **write-only mutations**: they do **not** record a read at the key. They **cannot** detect concurrent writes via serializability | Cap enforcement on `pin_count` cannot be done with `Add` alone; it requires a Serializable read **before** the `Add`. Today's code does this correctly (`bookmark.rs:194`); the prior proposal's table mislabels this as "atomic-add commutative" without flagging the read-before-add cap pattern |
| F6 | `CompareAndClear` is server-side: applies the clear only if `current == expected`; otherwise no-op. Does **not** add a read-conflict | Used for stale PIDX cleanup (`compact.rs:288`) and stale SHARD cleanup in eviction (`eviction/mod.rs:402-410`). Stale `expected_value` simply no-ops; safe |
| F7 | Versionstamp uniqueness is per-cluster (commit version + tx-internal-order). Across re-elections the version range is strictly greater | VTX/COMMITS writes via `SetVersionstampedKey/Value` (`commit.rs:255-264`). The FDB review confirms this holds in real backends |
| F8 | `clear_range` is atomic at commit | DELTA chunk range clears (`compact.rs:301`). No reader sees half-cleared blobs |
| F9 | In-process atomic-op simulator (`universaldb/src/atomic.rs:20-23`) returns `Some(param.to_vec())` for `SetVersionstamped*` with TODO. The RocksDB driver substitutes correctly (`rocksdb/transaction_task.rs:359-374`) | Tests that use the in-memory backend silently get non-versionstamped writes. **Tests #1 and #2 in the prior proposal would silently lie.** Must be fixed |

---

## 3. Conflict graph

The pair-wise table. "Op A × Op B" = "what shared keys can collide if A and B run concurrently?"

Legend for **Resolution today**: `L` = lease; `S` = serializability via Serializable read; `Sx` = serializability **but the read is Snapshot today, so no protection**; `OCC` = manual same-tx fence; `IDM` = idempotent/monotonic write only; `PB` = pegboard exclusivity.

| A × B | Shared keys | Kind | Resolution today | Survives review? |
|---|---|---|---|---|
| Commit × Commit (same DB) | `/META/head`, PIDX | correctness | PB + S(`/META/head` read in `commit.rs:86,92,93`) | Yes — PB primary; FDB conflict-on-head as backstop |
| Commit × Hot fold | PIDX/{pgno}, DELTA range | correctness | S (hot reads `/META/head` Serializable in write tx; PIDX uses `compare_and_clear`) | Yes — head read in conflict set; PIDX CAC handles stale clears |
| Commit × Cold Phase C | none direct (Phase C touches `/META/cold_compact`, BOOKMARK; commit touches `/META/head`) | none | none required | Yes — disjoint |
| Commit × Eviction clear | PIDX/{pgno}, SHARD/{...} | correctness | CAC; eviction's plan-side reads are Snapshot (Sx) but CAC handles | Yes — CAC is server-side conditional, no race possible |
| Commit × Fork-database | `/META/head`, COMMITS, VTX, refcount | mostly disjoint | refcount is `Add` (commutative); fork reads VTX of `at_versionstamp` from history, doesn't touch live `/META/head` | Yes — disjoint enough |
| Commit × Pinned bookmark create | `/META/head`, COMMITS/{head_txid}, BOOKMARK | correctness | bookmark reads `/META/head` Serializable; if commit advances head between bookmark's read and the next commit, conflict aborts bookmark → retry | Yes |
| Commit × GC | none directly; GC operates below `gc_pin`, commit at `head_txid` | none | none required | Yes — temporally disjoint |
| Commit × Pegboard rollback (DBPTR move) | DBPTR + `/META/head` of two branches | correctness | PB exclusivity ensures old writer stops before new mapping accepts; FDB conflict on `/META/head` of either branch as backstop | Yes |
| Hot fold × Hot fold (same shard) | SHARD/{shard_id}/{as_of_txid}, DELTA range, PIDX | correctness | S(write-tx `/META/head` Serializable read); deterministic SHARD content; CAC for PIDX | Mostly — but **see C1 (review correctness): hot fold deletes DELTAs without reading `desc_pin`** |
| Hot fold × Hot fold (different shard, same DB) | none | none | none required | Yes |
| Hot fold × Cold Phase A snapshot | DELTA, SHARD, COMMITS, VTX | correctness | Sx — Phase A reads are Snapshot, hot writes don't conflict; cold Phase C must catch via OCC fence | **No today** — see §6 |
| Hot fold × Cold Phase C | `last_hot_pass_txid`, `cold_drained_txid` | correctness | S(Phase C reads `cold_drained_txid` Serializable, OCC-aborts if changed) | Yes — but only if Phase A also re-reads conflict-relevant keys serializably (see §6) |
| Hot fold × Eviction | `last_hot_pass_txid`, SHARD versions | correctness | Eviction's clear tx re-reads `last_hot_pass_txid` Serializable + aborts if changed | Yes — but cost is high (review M2 abort storms) |
| Hot fold × Fork | DELTA range below `at_versionstamp`, SHARD versions | **correctness** | nothing | **No** — review C1 |
| Hot fold × Pinned bookmark create | `bk_pin`, SHARD versions | correctness | hot fold's `enforce_shard_version_cap` reads `bk_pin` Serializable inside the same tx; bookmark's atomic-`ByteMin` lands → conflict aborts hot fold | Yes — **but only when at cap**. When not at cap (most common), hot fold doesn't read `bk_pin`, so no conflict catch. Review C1 generalization |
| Hot fold × GC | none direct (GC reads pin keys, hot writes shard/delta) | none | hot's `enforce_shard_version_cap` reads pins Serializable when capped; GC doesn't touch shards | Yes — but the gap is hot fold below the GC pin (review C1) |
| Cold Phase A × Cold Phase A | `in_flight_uuid` | duplicate work | last-writer-wins on `in_flight_uuid`; lease (today) prevents this | If lease removed: M3 (loser pin layer orphan) |
| Cold Phase C × Cold Phase C | `cold_drained_txid` | correctness | OCC fence in `phase_c.rs:46-53`; abort one | Yes — keep the OCC fence |
| Cold Phase B × Stale-marker sweep | `pending/{uuid}.marker`, planned object keys | correctness | sweep deletes only markers older than `STALE_MARKER_AGE_MS` | **No today** — review M4. Sweep age must be > max pass duration |
| Eviction × Eviction (sweep) | `eviction_index`, SHARD/PIDX | duplicate work | global lease (today) prevents this | Yes — keep eviction lease (see §4) |
| Eviction × Hot fold | `last_hot_pass_txid`, SHARD versions | correctness | Eviction's clear-tx Serializable re-read of `last_hot_pass_txid` → abort | Yes — **but** review FDB 2.2 notes plan tx's Snapshot reads on SHARD/PIDX leak; need to add Serializable conflict-range on `last_hot_pass_txid` in plan tx as well, or accept that CAC carries the day |
| Eviction × Fork | `desc_pin`, `bk_pin` | correctness | filter_now_pinned re-reads pins Serializable in clear tx → no-op for newly-pinned | Yes |
| Eviction × Pinned bookmark | `bk_pin` | correctness | filter_now_pinned re-reads `bk_pin` Serializable | Yes |
| Eviction × GC | branches list, refcount | correctness | both read `branches_list` Serializable; if one deletes the branch, the other's tx conflicts | Yes |
| Fork × Fork (same source) | refcount(source), desc_pin(source) | correctness | `Add` commutative; `ByteMin` commutative | Yes — review m1 confirms |
| Fork × GC | `desc_pin/{source}`, COMMITS/{txid_at}, VTX/{at_versionstamp} | correctness | fork reads `bk_pin`/COMMITS/VTX Serializable; GC reads pins Serializable; either order, the read is in conflict set | Yes |
| Fork × Pinned bookmark create | `bk_pin`, refcount, `pin_count` | mostly disjoint (different source vs new branch) | bookmark uses `bk_pin` of *current* branch; fork uses `bk_pin` of *source*; same key only when forking the same branch the bookmark targets — `ByteMin` is commutative | Yes |
| Fork × Pinned bookmark delete | `bk_pin/{source}` | correctness | delete writes recomputed `bk_pin` plain `set` (not atomic-min). Fork reads `bk_pin` Serializable. Delete's plain `set` could overwrite an in-flight fork's atomic-min → **lost update** | **Pre-existing bug, see §6** |
| Pinned bookmark × Pinned bookmark (same NS) | `pin_count` | cap enforcement | Serializable read of `pin_count` (`bookmark.rs:194`) → conflict catches | Yes |
| GC × GC (same branch) | branch keys | duplicate work | both txs converge; deletes are idempotent | Yes |
| Delete-database × everything | refcount/{branch_id}, NSCAT tombstone | mostly commutative | `Add` and `SetVersionstampedValue` are commutative | Yes |
| Pegboard rollback × Commit | DBPTR | correctness | PB exclusivity revokes the writer; FDB conflict on DBPTR Serializable read in commit's `resolve_or_allocate_branch` as backstop | Yes — exclusivity-leak metric (review C2) needed |

---

## 4. Coordination mechanisms — the menu

| Mechanism | What it gives | When to use | When NOT |
|---|---|---|---|
| **FDB native serializability** | Read-write conflict on any key read Serializable in tx A, written in tx B (commit-time abort) | Pairs where both ops fit in single FDB txs and **the relevant read is Serializable**. Free | When either side uses Snapshot, when reads cross multiple txs, when the protected invariant is on a key that's `Add` / `ByteMin` (atomic, no read recorded) |
| **Lease (TTL + renewal + cancel)** | Same-role exclusion. Bounded by lease TTL even when holder dies. Efficiency only — does not protect correctness | Eviction sweep (avoids 10× FDB scan amplification across N pods); cold compactor (avoids duplicate Phase B S3 PUTs and the M3 loser-orphans problem) | Hot compactor — multi-shard fold idempotency + commit-rate gating make duplicate folds bounded waste; pegboard already serializes commits per database |
| **Lease epoch** | Multi-tx protection where a stolen-lease takeover may interleave | Eviction (plan tx → clear tx). Lease key carries an epoch counter; clear tx asserts epoch unchanged Serializably | Single-tx ops |
| **Idempotent / monotonic writes** | Parallel writers converge. Tolerates duplicate triggers, lease loss, NATS redelivery | SHARD content (deterministic from inputs); `cold_drained_txid` (monotonic guard); BOOKMARK status (Pending → Ready/Failed one-way); refcount `Add`; `desc_pin`/`bk_pin` `ByteMin`; PIDX `CompareAndClear`; DELTA at unique txids; S3 layers with deterministic filenames | When a cap or downgrade is needed (use read-then-write instead) |
| **Read-then-write for caps** | Conditional write where atomic-add can't enforce a ceiling | `pin_count` ≤ `MAX_PINS_PER_NAMESPACE` (already correct in `bookmark.rs:194`); any future `MAX_*` cap | When the value is naturally monotonic (use atomic-min/max instead) |
| **Pegboard exclusivity** | Single writer per database. Trust this where it applies; instrument for leaks | Commit path; `/META/head` writer | Anywhere outside a database's commit path. Compactors are not gated by pegboard |
| **NATS queue group routing** | Best-effort first-line dedup | Compactor triggers; warmup | Never as a correctness mechanism |
| **Stale-marker sweep** | Orphan cleanup for partial S3 work | Cold Phase B partial uploads | As a fence — sweep age must be > max pass duration |
| **Pending marker is the forensic trail** | Durable record of "pod X started pass Y at time T against branch B" surviving log loss | Operations: who's working on this DB? `pending/{uuid}.marker` body lists `pod_id`, `started_at_ms`, `last_phase` | — |

---

## 5. Per-conflict resolution decisions

For each entry in §3, the chosen mechanism, justification, cost, and recovery.

### 5.1 Commit × Commit (same DB)

**Resolution:** Pegboard exclusivity primary; FDB serializability on `/META/head` as backstop.
**Why correct:** PB enforces single-writer-per-database via lost-timeout + ping. The `/META/head` Serializable read (`commit.rs:86,92,93`) is in the conflict set; if PB ever leaks, the second commit's read of `/META/head` conflicts with the first's write at that key.
**Cost:** zero in steady state. Backstop has zero amortized cost (already paid for the read).
**Failure mode:** PB leak → second writer's commit aborts at FDB layer. User sees a transient retryable error. Add metric `pegboard_exclusivity_violations_total` incremented when `/META/head`'s previous value's `branch_id` matches but `head_txid` skipped (review C2). In debug builds, write `runner_id` field into `/META/head` value (gated by `#[cfg(debug_assertions)]`) so the abort log line classifies the conflict.

### 5.2 Hot fold × Fork (review C1)

**Resolution:** Hot fold's write tx adds a Serializable read of `desc_pin/{branch_id}` and `bk_pin/{branch_id}` and asserts `min(desc_pin_txid, bk_pin_txid) > max(deleted_delta_txids)` before clearing the DELTA range.
**Why correct:** This brings the pin keys into the hot fold tx's conflict set. A fork's `ByteMin(desc_pin, at_versionstamp)` write that lands between hot fold's pin read and hot fold's commit forces hot fold's tx to abort. Since the cleanup model wants "FDB serializability" to handle this case, we have to actually *read* the keys we're claiming to be protected against.
**Cost:** Two extra Serializable reads (`desc_pin`, `bk_pin`) per write tx. Negligible; both are in-tx with the head read already.
**Failure mode:** Concurrent fork → hot fold tx aborts and retries with new pin floor; deletes fewer DELTAs.
**Note:** This is a pre-existing bug fix that must land alongside the cleanup. Without it, hot fold can delete data a fork still depends on.

### 5.3 Cold Phase A × Cold Phase A

**Resolution:** Keep the per-branch cold compactor lease. Single live Phase B per branch.
**Why correct:** Lease acquisition + renewal serializes Phase A → Phase B → Phase C across pods. Loser pods either skip or wait for lease release. M3 (loser pin layer orphan) does not occur because there is no loser.
**Cost:** Lease renewal task per active pass (already exists). Lease TTL window during pod death (≤30 s) before another pod takes over.
**Failure mode:** Pod death mid-Phase-B → lease expires → next NATS trigger picks up; same `in_flight_uuid` reused (Phase A reuse logic, CLAUDE.md line 76). Stale-marker sweep does **not** delete the active marker because `STALE_MARKER_AGE_MS > LEASE_TTL_MS + MAX_PHASE_B_MS` (constraint, see §6).
**Why we don't drop the lease:** Review concern M3 (pinned bookmark stuck Pending) is real and isn't solved by idempotent S3 alone. The lease is what makes Phase B → Phase C an atomic operation across pods. Cold compactor passes are also expensive (multi-MB S3 writes), so duplicate work is not just bounded waste.

### 5.4 Cold Phase C × Cold Phase C

**Resolution:** Keep the OCC fence. Phase C reads `cold_drained_txid` Serializable and aborts if it differs from `state_before.cold_drained_txid` (`phase_c.rs:46-53`). Reframe in code comment: this is FDB serializability over a load-bearing key, not a bespoke fence.
**Why correct:** With the cold lease in place, two Phase C calls can only happen if the lease was lost mid-pass (TTL expiry, manual force). The fence catches it.
**Cost:** One Serializable read on every Phase C (already paid).
**Failure mode:** Loser Phase C aborts. Its uploaded S3 layers are idempotent overwrites of the winner's; no S3 corruption.

### 5.5 Eviction × Eviction (sweep amplification)

**Resolution:** Keep the global eviction lease. Reframe the eviction operation as **per-branch tx, not per-sweep**.
**Why correct:** Without the lease, N pods scan the same `eviction_index`, all paying the FDB read cost (review M1 / C1 amplification). The lease makes scan effort N→1.
**Cost:** Lease renewal per active sweep (a few minutes at most).
**Failure mode:** Holder dies → next pod takes over after TTL. Review M1 acknowledges 10× scan load is unworkable at 100k tenants without the lease.
**Per-branch tx restructure:** Today eviction does (lease) → (plan all branches Snapshot) → (clear all branches Serializable). This blows the 10 MB write set (review FDB 1.2). New shape: outer iterator scans `eviction_index` Serializable in batches of ≤16 branches per outer step; for each candidate branch, **plan + clear in one FDB tx** with explicit Serializable reads on `last_hot_pass_txid`, `desc_pin`, `bk_pin`, SHARD versions, PIDX. One branch's eviction fits comfortably in 5 s / 10 MB.

### 5.6 Eviction × Hot fold

**Resolution:** Per-branch eviction tx reads `last_hot_pass_txid` Serializable; hot fold writes `last_hot_pass_txid` plain set inside its write tx. Conflict at commit aborts the loser.
**Why correct:** Once eviction's read is Serializable (already in the clear tx today, line 381) **and the read happens in the same tx as the writes** (so it's in the eviction tx's conflict set when eviction commits), hot fold's write to `last_hot_pass_txid` in a parallel tx triggers a conflict.
**Cost:** Under hot bursts, eviction aborts frequently. Mitigation: per-branch backoff (see §7) caps per-DB retry attempts per sweep at 3; on exhaustion, skip the branch this sweep and emit `eviction_branch_skipped_total{reason="hot_pressure"}`. Next sweep retries.
**Failure mode:** Sustained hot pressure → branch skipped → eviction lag rises → quota pressure builds. Operator sees `sqlite_eviction_branch_skipped_total{branch}` and `sqlite_eviction_lag_branches`. Forced fallback: lower SHARD_RETENTION_MARGIN via runtime config to evict closer to the head.

### 5.7 Eviction × Fork / Pinned-bookmark

**Resolution:** Eviction's per-branch clear tx Serializably re-reads `desc_pin` and `bk_pin` (`eviction/mod.rs:443-448`). Forks/bookmarks landing between plan and clear show up; clear filters out newly-pinned versions.
**Why correct:** With plan + clear merged into one tx (per §5.5), the pin reads happen inside the same tx as the clears. Any fork's `ByteMin(desc_pin)` writes register as a conflict because the eviction tx already read the pin keys Serializably.

### 5.8 Fork × GC

**Resolution:** FDB serializability. GC reads `desc_pin`, `bk_pin` Serializable (`gc/mod.rs:73,86,87`); fork writes `ByteMin(desc_pin)`. The conflict aborts whichever commits second.
**Why correct:** Both reads are Serializable in current code. The cleanup proposal's claim holds for this pair.
**Cost:** GC retry budget per pass (see §7).

### 5.9 Pinned bookmark create × Pinned bookmark create

**Resolution:** Read `pin_count` Serializable (already correct, `bookmark.rs:194`), gate, then `Add` (+1).
**Why correct:** The Serializable read enters the conflict set. Two parallel creates near the cap both read `pin_count = 1023`; one's commit lands first, the second's tx aborts on read-write conflict at `pin_count_key` and retries with `pin_count = 1024`, then sees the cap and returns `TooManyPins`. Review M1 was correct that atomic-add alone wouldn't enforce; but the existing code uses Serializable read + atomic-add, which is exactly the read-then-write-for-caps pattern. **Document this pattern in CLAUDE.md so future refactors don't break it.**

### 5.10 Pinned bookmark delete × Fork (pre-existing bug)

**Resolution:** Change `bk_pin` recompute on delete to use `ByteMin` semantics: read all live pins Serializably, compute the new minimum, write via plain `set`. Fork's concurrent `ByteMin(bk_pin, at_versionstamp)` either lands before delete's read (delete's recompute includes it; safe) or after delete's commit (fork's `ByteMin` either keeps delete's new min or lowers it; safe). The `set` of a recomputed value plus a parallel atomic-min is **not** safe: the atomic op runs against whatever value happens to be present at commit. To fix: delete's tx adds a Serializable read on `bk_pin` and uses `set` only if the read value matches the expected pre-delete snapshot. Otherwise re-read all pins inside this tx and recompute.
**Cost:** One additional Serializable read.
**Why this is a pre-existing bug:** Confirmed in `bookmark.rs:301-302`: `tx.informal().set(&keys::branches_bk_pin_key(...), &recomputed_pin)`. The plain `set` overwrites a parallel fork's `ByteMin` write. Lost update.

### 5.11 Cold Phase B × Stale-marker sweep (review M4)

**Resolution:** `STALE_MARKER_AGE_MS = max(LEASE_TTL_MS, MAX_PHASE_B_MS) + safety_margin`, where `MAX_PHASE_B_MS` is the wall-clock ceiling on Phase B (e.g., 10 minutes for multi-GB databases). Concretely: `STALE_MARKER_AGE_MS = 30 minutes`. Add a Phase B watchdog that aborts any pass exceeding `MAX_PHASE_B_MS` (lease expiry would already do this in practice).
**Why correct:** With the cold lease still in place, lease renewal bounds active pass wall-clock; sweep age set strictly greater than that wall-clock means active passes can't be swept.
**Cost:** Stale markers from crashed passes survive up to 30 min before cleanup. S3 storage cost is bounded (small per-marker payload).
**Metric:** `cold_pending_markers_total` gauge with alert if > 1000 sustained (review M5).

### 5.12 Pegboard rollback × Commit

**Resolution:** Pegboard exclusivity primary; FDB serializability on DBPTR + `/META/head` as backstop. Add debug-mode `runner_id` field in `/META/head` (review C2).
**Why correct:** PB revokes the old writer; writer can't issue a commit after revoke. If PB ever leaks, the new actor's writer reads DBPTR Serializable in `resolve_or_allocate_branch`, sees the new branch, and writes `/META/head` of the new branch — the old writer's tx conflicts on DBPTR.
**Metric:** `pegboard_exclusivity_violations_total` counter incremented when a commit aborts because `/META/head`'s prior `runner_id` differs from this writer's id (debug builds only).

---

## 6. Required code-level changes

Grouped by category. Each change is necessary for the design to be correct.

### 6.1 Convert load-bearing reads from Snapshot to Serializable

| File:line | Change | Rationale |
|---|---|---|
| `compactor/cold/phase_a.rs:308,314,322` (`read_snapshot_plan` reads of `branches_list`, `MetaCompact`, `last_hot_pass_txid`) | Change to `Serializable` | These reads inform Phase C's plan; if they're not in the conflict set, parallel commits during Phase B silently invalidate the plan |
| `compactor/cold/phase_a.rs:512` (`scan_prefix` for SHARD/DELTA/COMMITS/VTX) | Keep `Snapshot` (these are bulk scans for read-only S3 upload; size of the read set would blow the conflict set if Serializable) but **add explicit `add_conflict_key` on `last_hot_pass_txid` and `cold_drained_txid` in the Phase A snapshot tx** so Phase C's commit conflicts correctly | Bulk scans must remain Snapshot for performance; conflict set is added selectively on the load-bearing fence keys |
| `compactor/eviction/mod.rs:269` (`scan_eviction_index`) | Keep Snapshot for the eviction_index outer scan, but **per-branch plan reads must be Serializable in the same tx as the clears** (per §5.5 restructure) | Outer scan is a hint; correctness lives in the per-branch tx |
| `compactor/eviction/mod.rs:563` (`load_branch_shard_versions`), `:600` (`load_branch_pidx_rows`) | Change to `Serializable` once they live in the per-branch plan-and-clear tx | These reads' values are used for `compare_and_clear` expected_value; they must be in the conflict set so a hot fold's SHARD overwrite aborts eviction |
| `gc/mod.rs:118,128,138,174` (`scan_prefix` calls in GC) | Keep `Snapshot` (deletions happen inside same tx; the scan is over append-only DELTA/COMMITS/VTX which don't get rewritten). The pin reads (already Serializable, lines 73, 215, 230, 245) carry the conflict-set load | OK as-is |
| `compactor/compact.rs:132,133,138` (`plan_batch` reads) | Keep Snapshot in plan tx; the **write tx already re-reads `/META/head` Serializable** (line 236) | OK as-is |

### 6.2 Mutation type changes

| File:line | Change | Rationale |
|---|---|---|
| `pump/bookmark.rs:301-302` (`recompute_database_branch_bk_pin` plain `set`) | Read the existing `bk_pin` Serializable; write the recomputed value only if `recomputed <= existing` (manual byte-min). Or use `ByteMin` directly | The plain `set` overwrites a parallel fork's `ByteMin` (review-discovered pre-existing bug) |
| `compactor/cold/phase_c.rs:59-66` (writes to `cold_compact` and `cold_drained_txid`) | Add a guard: skip the write if `current_state.cold_drained_txid >= new_cold_drained_txid` (monotonic guard) | Defense-in-depth even with the OCC fence; tolerates lease loss and same-txid duplicate triggers |

### 6.3 Tx restructurings

| Op | Before | After |
|---|---|---|
| Eviction sweep | 3 txs: lease + (plan all branches) + (clear all branches), with batch_size=256 branches in plan | 3 + 2N txs: lease + (eviction_index outer scan, batch=16) + N×(per-branch plan-and-clear tx) per outer batch. Each per-branch tx fits in 5 s / 10 MB |
| Cold compactor | 2 txs (handoff + snapshot) before Phase B, 1 tx after Phase B | Same shape. Handoff tx (Serializable) writes `in_flight_uuid` and adds explicit conflict keys on `last_hot_pass_txid` and `cold_drained_txid`. Snapshot tx loads bulk plan. Phase C tx commits Serializable with OCC fence |

### 6.4 Pre-existing bug fixes

These must land **before or alongside** the cleanup. They are not part of the cleanup itself but block its correctness story.

| Bug | Fix |
|---|---|
| **SHARD blob >100 KB** (`compactor/shard.rs:119`) | Chunk SHARD blobs into ≤90 KB pieces under `SHARD/{shard_id}/{as_of_txid}/{chunk_idx:u32_be}` keys (mirrors DELTA chunking at `commit.rs:32` `DELTA_CHUNK_BYTES`). Updates `fold_shard_inner` to write multiple keys; updates `load_latest_*_shard_blob` to concat chunks; updates `eviction/mod.rs` to clear the chunk range and to no longer carry the full blob as `expected_value` (use a chunk-0 fingerprint or a `shard_version_marker` key instead) |
| **Hot fold deletes DELTAs without pin check** (`compactor/compact.rs:298-302`) | Before the DELTA range clear, in the same write tx, Serializably read `desc_pin` and `bk_pin` for the branch. Compute the smallest `txid` cap as `max(desc_pin_txid, bk_pin_txid)` — actually the **min** of the pin txids. Assert all `selected_delta_txids` are ≤ `materialized_txid` AND ≥ pin floor + 1; if any selected delta is below the pin floor, abort the write tx (caller retries; plan tx will replan with new pin floor) |
| **Versionstamp simulator stub** (`universaldb/src/atomic.rs:20-23`) | Implement versionstamp substitution in the in-memory atomic-op path. Use a process-local monotonic versionstamp counter for the sim. The RocksDB driver already does this correctly (`rocksdb/transaction_task.rs:359-374`) |
| **`Pending` bookmark stuck after loser pass** (review M3) | Resolved by keeping the cold lease — there is no loser pass under the lease. As a defense-in-depth: Phase C's `mark_pin_ready` already rejects mismatched `(database_branch_id, versionstamp, bookmark)` (`phase_c.rs:114-119`); add a stale-bookmark sweep that flips `Pending` records older than `STALE_MARKER_AGE_MS` to `Failed` |

### 6.5 Hot compactor lease removal

This is the only piece of the original cleanup that survives.

| File | Change |
|---|---|
| `compactor/lease.rs` | Drop hot-related lease helpers; keep cold + eviction lease helpers |
| `compactor/worker.rs` | Drop hot lease take/renew/release wrapper around `compact_default_batch`. Multi-pod hot pass deduplication relies on NATS queue group + tx-level idempotency only |
| `compactor/compact.rs` | No change beyond §6.4 hot fold pin check |
| `pump/keys.rs:26` (`META_COMPACTOR_LEASE_PATH`) | Remove the key constant + helpers |
| `engine/packages/sqlite-storage/CLAUDE.md` | Update the concurrency model section: hot lease removed; cold and eviction leases kept. Add the idempotency invariants (SHARD content determinism, monotonic `cold_drained_txid`, BOOKMARK Pending → Ready/Failed, `bk_pin/desc_pin` ByteMin commutativity, PIDX CompareAndClear) |

### 6.6 Marker forensic-trail enrichment

`pending/{uuid}.marker` body adds `pod_id`, `pass_started_at_ms`, `last_phase` (review M2 / m2). Phase B updates the marker to record `last_phase = "B"` after the rewrite at the start of Phase B. Phase C does not touch the marker (the marker is deleted by the next pass's stale-marker sweep or by Phase C's success path).

### 6.7 Debug-mode runner_id in /META/head

Behind `#[cfg(debug_assertions)]`, add a `runner_id: NodeId` field to `DBHead`. Commit writes it; commit reads compare prior `runner_id` against current and increment `pegboard_exclusivity_violations_total` if they mismatch (review C2).

---

## 7. Operational chapter

### 7.1 Metrics

Cardinality-safe (review M3, m3). All `db_id`/`branch_id` labels are aggregated to `namespace_id` or `tenant_tier`; per-branch metrics are `_total` counters (low cardinality) or sampled to a small fixed set of "hot tenants" via a control-plane allowlist.

| Metric | Type | Labels | Owner | Alert default |
|---|---|---|---|---|
| `sqlite_eviction_tx_aborts_total` | Counter | `pod_id`, `reason` (`hot_pass_advanced`, `pin_advanced`, `tx_too_large`) | eviction | > 100/s sustained for 5m |
| `sqlite_eviction_branch_skipped_total` | Counter | `pod_id`, `reason` | eviction | > 50/min sustained |
| `sqlite_eviction_lag_branches` | Gauge | `namespace_id` | eviction | > 1000 sustained |
| `sqlite_cold_pass_duplicate_total` | Counter | `pod_id` | cold | > 5/min sustained (NATS redelivery storm) |
| `sqlite_cold_pass_duplicate_bytes_total` | Counter | `pod_id` | cold (review M6 — cost attribution) | dashboard only |
| `sqlite_cold_pass_duplicate_phase_a_reads_total` | Counter | `pod_id` | cold | dashboard only |
| `sqlite_cold_pending_markers_total` | Gauge | `pod_id` | cold | > 1000 sustained |
| `sqlite_cold_lag_versionstamps` | Gauge | `namespace_id` | cold | > 10 min materialization lag |
| `sqlite_cold_pass_active_pods` | Gauge | `pod_id` (low-card; counts active passes per pod, not per DB) | cold | > 1 sustained per pod |
| `sqlite_pegboard_exclusivity_violations_total` | Counter | `pod_id` | commit (debug builds) | any sustained — page oncall |
| `sqlite_fork_retries_total` | Counter | `reason` | fork | > 10/min sustained |
| `sqlite_fork_retry_exhausted_total` | Counter | `reason` | fork | any → user-facing 5xx |
| `sqlite_hot_fold_pin_aborts_total` | Counter | `pod_id` | hot | > 50/min (pin-vs-fold contention) |
| `sqlite_shard_versions_per_shard` | Histogram | `pod_id` | hot | p99 > MAX_SHARD_VERSIONS_PER_SHARD-2 |

### 7.2 "Compaction stuck" runbook (replaces "check the lease")

When a customer reports a database not making progress (cold-tier lag growing, `head_txid - cold_drained_txid` rising):

1. Read `/META/manifest/cold_drained_txid` and `/META/head` for the affected branch via UDB tooling. Compute lag.
2. Read `/META/cold_lease/{branch_id}` if present — gives current cold pass holder and TTL. (Cold lease is still kept under this design.)
3. If no cold lease: list `pending/{uuid}.marker` for the branch's S3 prefix. Each marker's body has `pod_id`, `pass_started_at_ms`, `last_phase`. The newest marker is the active or most-recent pass.
4. Query structured logs by `pass_uuid` from the marker — get the holder pod's full pass trace.
5. Query NATS queue depth on the cold compactor topic — if depth is high, triggers are queued behind a slow pod.
6. Query `sqlite_cold_pass_duplicate_total` and `sqlite_cold_pending_markers_total` — are passes being redelivered?
7. Forced retry: clear `/META/cold_lease/{branch_id}` (manual operator lever, replaces "clear `/META/compactor_lease`" today). Next NATS trigger picks up.
8. Forced retrigger: publish a synthetic NATS message via admin endpoint targeted at `branch_id`.

For hot compactor (no lease): step 1 alone tells you. Hot pass is stateless; if NATS triggers are flowing and `materialized_txid` isn't advancing, the bug is in hot pass code, not in coordination.

### 7.3 Eviction backoff under hot pressure

Per-branch retry budget: 3 attempts per sweep. Backoff: exponential 50ms → 200ms → 800ms with 25% jitter. On exhaustion: skip branch, increment `sqlite_eviction_branch_skipped_total{reason="hot_pressure"}`, retry on next sweep cycle (`sweep_interval_ms = 30000`).

Per-sweep retry budget: 32 branches before short-circuiting the sweep. On exhaustion: emit `sqlite_eviction_sweep_short_circuit_total`, complete current iteration, re-trigger via the next sweep tick.

Global escape hatch: control-plane `eviction_paused_until_ms` config flag (read at sweep start). If set and unexpired, sweep returns immediately. Used during incidents.

### 7.4 Fork retry budget & customer SLA

Default retry budget: 5 attempts (matches FDB tx default). Backoff per `calculate_tx_retry_backoff` (`utils/mod.rs:54-62`). Total worst-case latency: ~1.3 s + average tx duration.

On exhaustion: return `SqliteStorageError::ForkContentionRetryExhausted` (new variant). Engine layer translates to a 503 with `Retry-After: 30s`. Customer SLA: fork commits within 5 s p99 under healthy contention; under sustained contention, 503 with retry guidance is preferred over indefinite hang.

### 7.5 Migration plan (review M2)

Two-phase rollout to keep rolling deploys correct:

**Phase 1 — Code-only changes (no behavior change):**
- Land §6.1 (Snapshot → Serializable conversions and explicit `add_conflict_key`).
- Land §6.2 (mutation type fixes for `bk_pin` recompute and `cold_drained_txid` monotonic guard).
- Land §6.3 (eviction per-branch tx restructure).
- Land §6.4 pre-existing bug fixes (SHARD chunking, hot fold pin check, atomic-op simulator).
- Hot lease still active; both old and new pods write/respect it.
- Verify in production for 1 week.

**Phase 2 — Hot lease removal:**
- Land §6.5 (drop hot lease take/renew/release; new pods stop writing/reading hot lease).
- Old pods (still on Phase 1 code) continue taking hot lease; new pods ignore it. Both code paths must be correct standalone — they are, because Phase 1's correctness fixes don't depend on the hot lease.
- Old hot lease keys garbage-collect via TTL (30 s).

**Phase 3 — Cleanup (post-rollout, ≥1 week stable):**
- Remove dead code paths for hot lease in subsequent PR.

### 7.6 Rollback safety (review M3)

Phase 1 has zero schema bump and zero key-format change. Revert is safe.

Phase 2 introduces no schema bump either; reverting reintroduces hot-lease take/renew code paths that find no live lease keys (TTL-expired by the time of revert), take a fresh lease, and proceed. Revert-safe.

Both phases can be reverted independently. No destructive migrations. No `delete_old_lease_keys` startup task that would break revert.

### 7.7 Cardinality-safe metric labeling (review M3)

Never use `branch_id` or `database_id` as a Prometheus label. At 100k tenants, that's a cardinality bomb. Use:

- `namespace_id` for tenant-tier rollups (operators have ≤1k namespaces typically).
- `pod_id` for pod-attribution metrics.
- `reason` enum string for failure-class attribution.

For per-branch debugging, rely on **structured logs** (`branch_id`, `pass_uuid`, `phase`, `pod_id` fields) queried via the log search tool. Logs are higher cardinality but indexed differently.

For "active passes per branch" introspection (review C3), use the `pending/{uuid}.marker` S3 listing, not Prometheus.

---

## 8. Tests

Each test references a conflict pair from §3, the FDB primitive it depends on, and the expected outcome.

1. **Two parallel hot folds on same DB → identical SHARD content** (Hot fold × Hot fold). Depends on F4 (Serializable head read), F8 (deterministic SHARD content). Both write the same SHARD bytes; one tx aborts on F4, surviving SHARD content matches the deterministic encoding.

2. **Hot fold deletes DELTA below desc_pin → abort** (Hot fold × Fork — review C1). Depends on §6.4 hot fold pin check landing. Sequence: fork at txid 50; hot fold attempts to fold txids 1..100; hot fold's tx reads `desc_pin` Serializable, sees `txid 50 < 100`, aborts (does not delete DELTA[1..50]). Replanning skips DELTAs ≤50.

3. **Two parallel cold passes → losing one aborts at OCC fence** (Cold Phase C × Cold Phase C). Depends on F4 (Serializable `cold_drained_txid` read in Phase C). Forces lease loss to allow two Phase Cs; verify the loser aborts and surviving `cold_drained_txid` advances correctly.

4. **Eviction during active hot pass → eviction tx aborts and retries** (Eviction × Hot fold). Depends on F4 (Serializable `last_hot_pass_txid` read in eviction's per-branch tx) and per-branch tx restructure (§6.3). Verify the eviction tx aborts on the first retry and the second retry sees the new `last_hot_pass_txid` and replans.

5. **Fork during GC pin advancement → fork retries** (Fork × GC). Depends on F4 (Serializable `desc_pin` read in fork). GC reads pin, fork writes `ByteMin(desc_pin)`, GC's deletion tx aborts on conflict, retries with new pin floor.

6. **Pinned bookmark cap enforcement under parallel creates** (Pinned bookmark × Pinned bookmark — review M1). Depends on F4 (Serializable `pin_count` read) and F5 (atomic-add doesn't enter conflict set). Two parallel creates near `MAX_PINS_PER_NAMESPACE - 1`; one commits, the other's tx aborts on `pin_count` read-write conflict and on retry sees the cap and returns `TooManyPins`.

7. **Stale-marker sweep does not delete active marker** (Cold Phase B × Stale-marker sweep — review M4). Depends on `STALE_MARKER_AGE_MS > LEASE_TTL_MS + MAX_PHASE_B_MS`. Mock a slow Phase B (5 min); verify sweep at 10 min from pass start does not delete the marker.

8. **Pinned bookmark delete preserves parallel fork's bk_pin** (Pinned bookmark delete × Fork — review-surfaced pre-existing bug). Sequence: delete reads pins, fork lands `ByteMin(bk_pin, V_low)`, delete's tx aborts on `bk_pin` Serializable read-conflict and retries with new pin set including the fork's pin.

9. **Concurrent forks of same source → both succeed; pins commute** (Fork × Fork). Depends on F5 (`Add` and `ByteMin` commutativity, F7 versionstamp uniqueness). Both forks commit; source's `desc_pin` is `min(V_A, V_B)`, refcount = original + 2.

10. **Cold pass with stale plan → defense-in-depth monotonic guard** (Cold Phase C × Cold Phase C, lease loss path). Depends on monotonic guard (§6.2). Pod A's `state_before.cold_drained_txid = 80`; pod B advances to 150 after lease takeover; pod A's Phase C reads `current = 150 > intended new = 100`; A skips the `cold_drained_txid` write but still flips its uploaded BOOKMARK pins to `Ready` (idempotent transition).

11. **SHARD chunked write/read round-trip** (pre-existing bug fix). Verify a SHARD with 64 dirty pages encodes to multiple FDB rows of ≤90 KB and decodes correctly. Verify eviction's clear of a chunked SHARD removes all chunks.

12. **Pegboard exclusivity violation surfaces in metrics** (Pegboard rollback × Commit). Force two writers via test harness; verify second commit's tx aborts and `sqlite_pegboard_exclusivity_violations_total` increments (debug build only).

13. **Versionstamp simulator round-trip** (universaldb pre-existing bug). Verify in-memory atomic-op path substitutes `[0xff..0xff,0,0,0,0,0,0]` placeholders with monotonic versionstamps so tests #1, #3, and #9 don't silently lie.

14. **Eviction per-branch tx fits in 10 MB** (review FDB 1.2). With `MAX_SHARD_VERSIONS_PER_SHARD = 32`, 64 PIDX rows per shard, run a synthetic eviction over a single branch with worst-case version churn; verify the per-branch tx commits without `transaction_too_large`.

15. **Eviction backoff under sustained hot bursts** (review C1 / m2). Drive 100 ms commit cadence on a branch; verify eviction skips the branch after 3 retries and emits `sqlite_eviction_branch_skipped_total{reason="hot_pressure"}`. Subsequent sweep retries.

---

## 9. Open questions / tensions

1. **Eviction lease TTL during long sweeps.** With per-branch txs, a sweep over 1k branches takes minutes. Lease TTL must exceed the sweep duration or renewal must run during the sweep loop. Today's lease is 30 s with renewal — keep that, but `sweep_interval_ms` must be tuned so a single sweep finishes within `lease_ttl_ms`. **Tentative answer:** cap sweep work at `min(batch_size_total, time_budget_ms)`; emit a metric `sqlite_eviction_sweep_truncated_total` when time-budget exits early.

2. **Cold compactor lease vs. ForkWarmup payload.** ForkWarmup passes don't have a clear "branch" in the conventional sense (target_database_branch_id is the new fork). Does the cold lease key for ForkWarmup map to `target_database_branch_id` or `source_database_branch_id`? Today it's target. **Confirm:** target is correct because ForkWarmup writes layers into the target's S3 prefix.

3. **Hot compactor without a lease — duplicate fold rate.** NATS queue group dedup is best-effort. Under healthy steady state (no churn), expected duplicate rate is < 1%. Under deploy churn, duplicates can spike. The bound on duplicate hot folds needs empirical measurement post-rollout. **Plan:** ship the metric `sqlite_hot_fold_duplicate_total` and observe before declaring "OK."

4. **Atomic-op simulator vs. real-FDB versionstamp ordering.** With the simulator fix in §6.4, in-memory tests use a process-local monotonic counter. Cross-process tests require RocksDB driver. Document which test paths are valid under which backend. **Tentative answer:** all versionstamp-ordering tests must opt-in to RocksDB via `test_db()` (already the convention).

5. **Pegboard exclusivity gap window length.** PB's lost-timeout is 30 s by default. During that window, two writers can theoretically commit to the same database before FDB's `/META/head` Serializable conflict catches one. **Acceptance:** PB-leak commits abort cleanly via FDB; the engine sees a transient retryable error. The metric exists to alarm if this becomes frequent. We do not add a per-tx generation fence beyond `#[cfg(debug_assertions)]` because PB is the right layer to fix.

6. **MAX_PHASE_B_MS budget.** Largest active database determines this. For a 1 GB database, Phase B can take minutes. **Tentative answer:** 10 min budget, watchdog abort. `STALE_MARKER_AGE_MS = 30 min`. Revisit if customer DBs grow >10 GB.

7. **NATS JetStream redelivery vs. core NATS.** Reviews note core NATS has no redelivery; JetStream does up to `MaxDeliver`. Cold compactor today uses UPS (a JetStream-backed abstraction). **Tentative answer:** confirm UPS configuration; ensure `MaxDeliver` is bounded so storm cost is bounded.

---

## Summary

**High-level design.** Keep the cold compactor lease (per-branch) and the eviction global lease. Drop the hot compactor lease only. Convert load-bearing reads from Snapshot to Serializable in cold Phase A's handoff, eviction's per-branch plan, and pin recompute. Restructure eviction's plan-and-clear into one tx per branch (within 5 s / 10 MB). Fix two pre-existing correctness bugs that block the cleanup story: SHARD blob chunking (>100 KB violates FDB) and hot fold deleting DELTAs without checking `desc_pin`. Use `pending/{uuid}.marker` enriched with `pod_id` + `pass_started_at_ms` as the durable forensic trail replacing the lease's "who's compacting this DB" answer. Roll out in two phases (code-only correctness fixes; then hot lease drop) for revert safety.

**Resolved review issues.** All correctness blockers (C1 hot fold vs. fork, C2 cold pass plan staleness, M3 loser pin transition orphan, M4 stale-marker vs active pass) are resolved by retaining the cold lease and adding the hot fold pin check. FDB review's central claim — that most "Serializable" reads are actually Snapshot — is addressed by the §6.1 conversions. The 100 KB SHARD violation and 10 MB eviction tx violation get explicit fixes (§6.4 chunking, §6.3 per-branch tx). Operational reviews' demands for runbooks, backoff protocols, fork SLAs, migration plans, rollback safety, and cardinality-safe metrics are all addressed in §7.

**Deferred / accepted.** M1 (pin_count cap) was already correct in code (Serializable read + atomic-add); the proposal mislabeled the protection mechanism. m1 (`desc_pin` multi-fork) is benign as the review acknowledges. Hot compactor's "without lease, who wakes to fold?" is accepted as bounded duplicate work given idempotent SHARD content + commit-rate gating; the metric will tell us if real cost is unacceptable.

**Open questions.** Eviction lease TTL sizing under multi-branch sweeps, MAX_PHASE_B_MS budget for large DBs, and NATS JetStream redelivery configuration are flagged for empirical tuning post-rollout.

# Review: SQLite concurrency cleanup vs FDB realism

Adversarial review of `.agent/specs/sqlite-concurrency-cleanup.md` against actual FDB primitives and the codebase. Findings below are correctness-breaking unless tagged otherwise.

---

## 1. Hard FDB-limit issues

### 1.1 SHARD blob exceeds FDB 100 KB value limit (correctness)

`engine/packages/sqlite-storage/src/compactor/shard.rs` lines 113 and 119: `fold_shard_inner` encodes the LTX V3 blob and writes it under one key with `tx.informal().set(&key, &encoded)`, no chunking. With `SHARD_SIZE = 64` pages and `PAGE_SIZE = 4096` (`pump/keys.rs` 18-19), a fully-populated shard is ~256 KB plus LTX header overhead. FDB's hard value limit is 100 KB (`atomic.rs::apply_append_if_fits` already cites this; the spec acknowledges it on line 94 calling SHARD writes "chunked across multiple FDB rows" — that is wrong for the live code).

The cleanup proposal claims (line 49) "parallel hot folds on the same shard write the same content (deterministic). FDB serializability detects key collisions." Neither helps when `txn.put` rejects a 256 KB value with `value_too_large`. This is a pre-existing bug that the cleanup also leaves unfixed; the eviction "single-tx per database" plan compounds it because eviction reads back the full SHARD value to use as the `expected_value` for `compare_and_clear` (eviction `mod.rs` line 410, value cloned into `EvictableShardVersion.shard_value`), so any DB whose shards exceed 100 KB cannot evict at all. The proposal's "merge plan + commit into one tx per database" assumes shards are small — they aren't.

### 1.2 Eviction "one FDB tx per database" can blow the 10 MB write set (correctness)

`scan_eviction_index` already pages through `batch_size = 256` candidate branches per outer tx (eviction `mod.rs` 244-291) and `plan_evictable_shard_versions` returns every evictable shard version per branch. `clear_evictable_shard_versions` (line 363) batches all of them into one `udb.run` call. Each evicted version is two clears: a `compare_and_clear` against the SHARD blob (up to ~256 KB tracked in conflict-range bytes) plus N PIDX `compare_and_clear`s (one per page in that shard, up to 64). With 256 branches * 32 versions/shard * 64 PIDX/shard, the conflict-range + write-set blows past FDB's `SizeLimit` (default 10 MB). This produces `transaction_too_large`, which is non-retryable, so eviction sweeps stop progressing on busy clusters. The proposal does not mention bounding this.

### 1.3 Cold Phase A snapshot scan can exceed 5s tx age (performance/correctness)

`phase_a.rs::read_snapshot_plan` issues five sequential prefix scans (`load_shard_versions`, `load_delta_chunks`, `load_commit_rows`, `load_vtx_rows`, plus `branch_record`) inside one tx, each `WantAll`. Even with snapshot reads, every fetch counts toward the 5 s `TXN_TIMEOUT` (`universaldb/src/transaction.rs` 18). A branch with 32 shard versions, multi-MB DELTA chunks across 1000s of txids, and proportionate VTX/COMMITS rows trips the `phase_a_read_timeout_ms` guard. The cleanup proposal does nothing to bound this — and removing the cold lease allows multiple pods to issue these scans simultaneously, increasing FDB read pressure.

### 1.4 Hot retention sweep may overflow write set (performance)

`compactor/compact.rs::write_batch` clears DELTA chunk ranges for every selected delta plus runs `sweep_hot_retention` inside the same tx. With `batch_size_deltas` large and many DELTA chunks per delta (10 KB each), the cleared-bytes count can climb. Not as fatal as 1.2 but worth bounding.

---

## 2. FDB primitive misuse

### 2.1 Cold Phase A snapshot reads exclude conflict set (correctness, load-bearing)

The proposal's central claim (line 28-29, 56-57) is "FDB native serializability handles parallel cold passes via `cold_drained_txid`." This requires Phase A's reads to be in the conflict set so a conflicting commit aborts Phase A's commit.

The actual code uses `Snapshot` for every Phase A read inside `read_snapshot_plan` (`phase_a.rs` 308, 314, 322; `scan_prefix` line 512 hard-codes `Snapshot`). FDB snapshot reads do NOT add to the read conflict set. So:

Sequence: Pod A starts Phase A at version V1, reads `cold_drained_txid = 100` (snapshot). Pod B does a full pass (Phase A/B/C) advancing `cold_drained_txid = 150`. Pod A finishes Phase B (S3 PUTs idempotent, fine) and enters Phase C. Phase C uses `Serializable` to re-read `current_state.cold_drained_txid` (`phase_c.rs` line 90) and compares it against `expected_cold_drained_txid = plan.state_before.cold_drained_txid`. The `state_before` field is set from `register_pending_handoff`'s Serializable read (line 250) — but that happened in a *separate* `db.run` block (line 249), so its read conflict range is gone by the time Phase C runs.

The result: Phase A's plan was computed against snapshot data older than the handoff write; Phase C's manual fence reads the current state but compares against the handoff's snapshot. The proposal explicitly drops the bespoke `cold_drained_txid` fence in §22 ("Drop the bespoke OCC fence on cold_drained_txid"). With that gone, Pod A's Phase C — which wrote idempotent S3 layers based on a stale plan — can advance `cold_drained_txid` past Pod B's value if Pod A's `materialized_txid` was higher. Phase A snapshot reads on the SHARD/DELTA/COMMITS/VTX prefixes mean Phase C will not see commits that occurred during Phase B as conflicts.

The cleanup's monotonic-guard ("write only if strictly greater") catches the simple regression but not "Pod A computed against a half-snapshot of state from V1, plus a few mutations Pod B added during Phase B that Pod A never read."

### 2.2 Eviction Phase A reads are also Snapshot (correctness)

`eviction/mod.rs::scan_eviction_index` line 269, `load_branch_shard_versions` 563, `load_branch_pidx_rows` 595 all use `Snapshot`. The "single-tx eviction" claim is stronger than the code: planning and clearing are two `udb.run` blocks (lines 252 and 372). The clear-tx re-reads `last_hot_pass_txid` (line 381-387) with Serializable as the manual fence — exactly the "OCC fence" the proposal wants to drop. If that explicit re-read is removed in favor of pure FDB serializability, the snapshot reads in the planning tx still aren't in any conflict set. Concretely:

Pod 1 plans (snapshot reads SHARD value V1, PIDX P1 → adds to evictable list). Hot compactor commits new SHARD V2 + new PIDX P2 (the snapshot reads recorded no read-conflict on those keys). Pod 1's clear tx fires `compare_and_clear(SHARD, V1)` — no-op because shard is V2 now (fine). But `compare_and_clear(PIDX_key, P1_value)` — PIDX_key now has P2 *but for a NEW page*. Stale `expected_value` clears nothing. The clear tx commits despite hot's intervening commit.

The current `last_hot_pass_txid` re-read is doing real work; removing it leaves the snapshot-planned eviction fully reliant on `compare_and_clear` per-key for safety. That works for atomicity (no wrong key cleared) but not for invariants tying a SHARD version to the PIDX rows that should exit with it: if a fork lands between plan and clear and bumps `bk_pin`, the planning snapshot didn't see it; `filter_now_pinned_versions` (line 434) re-reads pins with Serializable inside the clear tx, but only the *pin keys*, not the SHARD or PIDX, so the clear tx's read-conflict surface is just `{last_hot_pass_txid, branches_desc_pin, branches_bk_pin}`. Hot pass commits to SHARD/PIDX during planning create no conflict.

### 2.3 GC pin reads are Snapshot (correctness, performance-bounded)

`gc/mod.rs` 118, 128, 138, 174 — every GC scan uses Snapshot. The proposal (line 78-80, 140) claims "GC's tx aborts on read-write conflict on `desc_pin` and retries" if a fork lands. Whether that holds depends on whether GC reads `desc_pin` with Serializable (need to check); if it's Snapshot, the fork-vs-GC race is not protected by FDB serializability and the proposal's argument fails. The OCC mention in `CLAUDE.md` line 34 says "fork() regular-reads parent's META/manifest.retention_pin_txid inside the fork tx" — the *fork* uses Serializable (verified `branch.rs::read_versionstamp_pin` line 876). But GC's pin scan, not fork's, is what needs Serializable for the proposal's claim to hold.

### 2.4 `apply_atomic_op` for `SetVersionstampedValue` is broken in the in-process driver (correctness for tests)

`universaldb/src/atomic.rs` lines 20-23: the in-process atomic-op simulator returns `Some(param.to_vec())` for `SetVersionstamped*` with a `// TODO: impl versionstamps` comment. The RocksDB driver (`transaction_task.rs` 359-374) substitutes correctly, but other backends (the in-memory test atomic helper) do not. Tests asserting versionstamp ordering across parallel commits will pass with non-versionstamped writes — masking real bugs in the new model. Tests #1 and #2 in the proposal will silently produce wrong ordering on the in-memory backend.

### 2.5 `apply_min` / atomic-min sentinel hazard (correctness)

`atomic.rs::apply_min` early-returns if `current.is_none()` (line 91): "if no current value, return param." For `bk_pin`/`desc_pin` initialization, the proposal stores empty pin as zero (CLAUDE.md line 96) and uses `0xff..ff` as advanced fence. But `MutationType::Min` does little-endian *integer* comparison (option docs line 244). The pins are 16-byte versionstamps stored big-endian — `Min` would compare them as i64 LE of the first 8 bytes, which is meaningless. The codebase correctly uses `MutationType::ByteMin` instead (`branch.rs` 615, CLAUDE.md line 63). This is correct, but the proposal's table line 92 says "atomic-min" without specifying — any future refactor that picks `Min` over `ByteMin` silently corrupts pin values. Worth pinning down in the proposal.

---

## 3. Realistic failure scenarios

### 3.1 Pegboard exclusivity is not absolute (correctness, load-bearing)

The cleanup leans on pegboard exclusivity as the "only mechanism" for single-writer (line 18). The pegboard model uses `lost_timeout_ts` (`pegboard/src/workflows/actor2/mod.rs` 423, 561) — a wall-clock timeout that releases an actor for re-placement. During graceful migrations, network partitions where the old writer is alive but unreachable, or clock skew between the orchestrator and worker, two pegboard-envoy instances can hold a connection to the same actor for a brief overlap window (multi-second worst case during partition).

The proposal acknowledges this on line 39 ("If pegboard ever leaks (bug), FDB serializability catches it via `/META/head` read-write conflict"). This is true for `/META/head` (commit reads it Serializable on line 86 and writes it; conflict detected). But not for derived state: two parallel commits would both read `branch_meta_compact` Snapshot (line 105), both succeed in advancing `materialized_txid`, both publish compact triggers — and the cold/hot compactor leases that previously serialized this are gone. The "one bad commit, retry, done" intuition does not hold across the whole commit path; only `/META/head` is conflict-protected.

### 3.2 Versionstamp interleaving across DBs (correctness, fine)

FDB's versionstamp is per-cluster (`commit version + 2-byte tx-internal-order`), guaranteed unique and monotonic within the cluster. Parallel commits on different DBs get different versionstamps; VTX index entries do not collide. The driver's `substitute_raw_versionstamp` (`transaction_task.rs` 360) does the right thing. Re-elections produce a new version range that's strictly greater. This claim of the proposal does hold.

### 3.3 NATS queue group dedup `~99%` is optimistic (performance, not correctness)

The proposal (line 33) says NATS queue groups dedup ~99% of compactor triggers. NATS core (no JetStream) queue groups distribute one message to one consumer in the group, full stop — no redelivery on consumer crash. JetStream queue consumers do redeliver on ack timeout (default 30 s). If the cold compactor is JetStream-backed, redelivery rate during pod churn or `AckWait` exhaustion is bounded by `MaxDeliver`, not by 1%. Under deploy churn, every restart causes pending-but-unacked passes to be redelivered to a new pod that picks up the same `in_flight_uuid`. That part is intentional (`Phase A reuses in_flight_uuid on retry`, CLAUDE.md 76). But it means "duplicate cold trigger rate" is closer to "as many redeliveries as `MaxDeliver` allows during a deploy" — not 1%. The S3 cost concern (line 231) is real and the metric is necessary.

### 3.4 Multiple eviction pods on same DB without lease (correctness via 1.2)

The proposal's open question 3 acknowledges this. Concrete failure: pods P1 and P2 both call `sweep_once`. With the global lease removed (per the cleanup), both `scan_eviction_index` reads return the same candidates. Both compute the same plan. P1 commits first, clearing SHARD V1 and PIDX rows. P2 commits second; its `compare_and_clear` calls all no-op because the values changed (good — idempotent). But the planning tx tx-reads ~10 MB of SHARD blobs uselessly, doubling FDB read load and burning cold-shard cache.

Worse: P2's `filter_now_pinned_versions` (line 434) might newly observe a fork's `desc_pin` that landed between P1's clear and P2's clear. So P2 filters out shards P1 already deleted plus a few that should not be deleted. P2's clear-tx has nothing to do (all `compare_and_clear` no-op) — fine. But the "fully evicted index keys" computation (`fully_evicted_index_keys_after_clear` line 477) re-reads the current shards Snapshot post-clear; it can compute "all shards evicted, drop eviction index entry" based on a state that hot-compactor is rebuilding right now. The eviction index drop races the hot pass's re-add: another correctness-flavored concern even if eventually consistent.

---

## 4. Performance concerns

### 4.1 Eviction abort storms under hot bursts (performance)

The proposal's edge case 1 calls this out as a concern. Concrete numbers: hot compactor commits one shard fold per tx. On a hot DB with 32 shards, a hot pass produces ~32 commits in seconds. Eviction's clear-tx reads `last_hot_pass_txid` Serializable; every hot fold writes that key. So during a hot burst, every eviction tx has a guaranteed read-write conflict against at least one of those writes. FDB retry backoff is exponential to a 1 s cap (`utils/mod.rs::calculate_tx_retry_backoff`). Under sustained hot pressure (continuous commits), eviction may livelock — never observing a stable `last_hot_pass_txid` across plan-and-clear. The proposal mentions backoff but doesn't quantify; livelock requires explicit detection (e.g., "after N aborts, skip this branch this sweep").

### 4.2 Cold pass duplicate uploads after a partial Phase B (performance)

NATS redelivery + no lease = pod B picks up the trigger while pod A is mid-Phase-B. Per CLAUDE.md line 76, "Phase A reuses `in_flight_uuid` on retry" — but pod B's `register_pending_handoff` sees the existing `in_flight_uuid` and reuses it (`phase_a.rs` line 271). Both pods now upload the same object keys. S3 PUTs are idempotent (line 119 of proposal), so byte-correctness holds, but every duplicated pass costs S3 PUT requests (real money) plus phase-B CPU. Bounded by NATS `MaxDeliver` * deploy frequency, but worth quantifying.

### 4.3 Read-your-writes within commit tx is fine

`SnapshotRywEnable` is documented as default in `options.rs` 144-147. The current code mixes Serializable and Snapshot reads but does not depend on snapshot-RYW edge cases for correctness.

---

## 5. Claims that do hold

- **Pegboard single-writer for `/META/head` writes.** Commit reads `head_key` Serializable (`commit.rs` 86, 92, 93); two same-DB commits would conflict. The "FDB catches the bug" claim works *for that key* if pegboard ever leaks.
- **Atomic-add for refcount, atomic-min for pins.** `MutationType::Add` and `MutationType::ByteMin` are commutative inside an FDB tx; concurrent fork txs converge correctly on those keys (verified in `apply_min` / `apply_byte_min`).
- **`COMPARE_AND_CLEAR` for PIDX.** It's a real FDB atomic op (`MutationType::CompareAndClear`) — read-then-clear is server-side, no race possible. The cleanup's claim here is fine.
- **`SetVersionstampedValue` ordering across DBs.** FDB cluster-wide versionstamp uniqueness holds across re-elections; parallel commits on different DBs interleave correctly.
- **`clear_range` for DELTA chunks.** FDB `clear_range` is atomic at commit time; no reader sees a half-cleared blob.

---

## Summary of correctness blockers

1. `fold_shard` writes a single >100 KB key (1.1) — pre-existing bug, blocks the cleanup's eviction "single-tx" plan.
2. Eviction's planning + clearing tx aren't actually one tx, and both planning paths use Snapshot reads that don't enter the conflict set (1.2 and 2.2). FDB serializability does not catch hot-vs-eviction interleaving on SHARD/PIDX keys without the `last_hot_pass_txid` re-read the proposal wants to delete.
3. Cold Phase A's snapshot reads do not feed Phase C's conflict set (2.1). The monotonic guard catches direct regression but not "computed against half-stale state." Removing the manual `cold_drained_txid` fence makes this strictly worse.
4. GC's pin scans use Snapshot (2.3); the proposal's "fork-vs-GC OCC handled by FDB serializability" claim depends on that being Serializable.
5. Atomic-op simulator stub for `SetVersionstamped*` (2.4) silently passes versionstamp-ordering tests with wrong values — the new model's tests can't be trusted on the in-memory backend.

The core insight of the cleanup (idempotent + monotonic writes survive duplicate work) is sound. The realization in code does not yet have the conflict-set discipline the proposal assumes — most reads are Snapshot, and the explicit Serializable re-reads being deleted are precisely the keys carrying the cross-tx invariant. Removing them without flipping every relevant read to Serializable inverts the safety story.

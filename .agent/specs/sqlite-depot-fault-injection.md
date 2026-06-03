# SQLite Depot Fault Injection

Spec for a depot-only fault injection system exercised through the real
RivetKit SQLite VFS. The purpose is to prove the new SQLite storage path stays
correct across depot failures, compaction failures, and SQLite database reloads
without relying on mock protocol shims.

## Decision Summary

- **Use one VFS test path:** `SQLite VFS -> DirectStorage -> depot Db -> depot workflows`.
- **Delete the VFS mock transport path.** Remove `MockProtocol`, `SqliteTransport::from_mock`, `SqliteTransportInner::Test`, and mock-backed tests unless rewritten against direct depot.
- **Do not add envoy as a VFS test variant.** Production envoy stays, but VFS correctness tests do not run an envoy/direct/mock matrix.
- **Replace the existing narrow direct transport fault hooks** with a depot fault controller.
- **Keep v1 depot-only.** No UDB driver faults, no global Rivet fault injection, no Gasoline correctness tests.
- **Do not corrupt arbitrary bytes in v1.** Use semantic depot faults: fail, pause, delay, and drop depot-owned artifacts.
- **Disable manager planning timers in fault tests** so compaction only runs when explicitly forced.
- **Use depot's real workflow entry points** for manager, hot compacter, cold compacter, and reclaimer behavior without testing Gasoline itself.

## Goals

- Exercise real SQLite pager/VFS behavior against real depot storage.
- Inject deterministic failures at depot semantic boundaries.
- Force hot/cold/reclaim workflow work without waiting for manager timers.
- Simulate SQLite database reload by clearing VFS and depot in-memory caches.
- Verify results against native SQLite and depot invariants.
- Make failures replayable with seed, workload, checkpoint, and fault metadata.
- Keep production hot paths free of fault-injection branches and latency.

## Non-Goals

- Do not test SQLite core correctness. Native SQLite is the oracle.
- Do not test envoy transport behavior in the fault suite.
- Do not test UDB or Gasoline correctness in v1.
- Do not simulate arbitrary storage corruption or byte flips in v1.
- Do not use retries or longer waits to mask flakes.
- Do not trust DirectStorage mirrors or in-memory VFS state as proof of durability.

## File Layout

### RivetKit SQLite Fault Tests

The VFS-facing tests live under `rivetkit-sqlite` because they need the real
VFS and private direct-storage test harness.

```text
rivetkit-rust/packages/rivetkit-sqlite/tests/inline/fault/
rivetkit-rust/packages/rivetkit-sqlite/tests/inline/fault/mod.rs
rivetkit-rust/packages/rivetkit-sqlite/tests/inline/fault/scenario.rs
rivetkit-rust/packages/rivetkit-sqlite/tests/inline/fault/oracle.rs
rivetkit-rust/packages/rivetkit-sqlite/tests/inline/fault/verify.rs
rivetkit-rust/packages/rivetkit-sqlite/tests/inline/fault/workload.rs
rivetkit-rust/packages/rivetkit-sqlite/tests/inline/fault/simple.rs
rivetkit-rust/packages/rivetkit-sqlite/tests/inline/fault/chaos.rs
```

`simple.rs` is deterministic and suitable for regular CI. `chaos.rs` is
ignored or separately feature-gated.

### Depot Fault Injection API

Depot fault injection lives in depot behind a dev-only Cargo feature:

```text
engine/packages/depot/src/fault/mod.rs
engine/packages/depot/src/fault/controller.rs
engine/packages/depot/src/fault/points.rs
engine/packages/depot/src/fault/actions.rs
engine/packages/depot/src/fault/checkpoint.rs
```

Feature gate:

```toml
[features]
test-faults = []
```

`#[cfg(test)]` alone is not enough because `rivetkit-sqlite` tests depend on
`depot` as another crate. `depot/test-faults` must be enabled only for dev/test
dependencies.

Production-leak guard:

- default and release builds must not compile fault controller symbols
- default and release builds must not compile delay/pause/drop-artifact branches
- default and release builds must not serialize `disable_planning_timers`
- no non-dev dependency may enable `depot/test-faults`
- verification should include `cargo check -p depot --release` without
  `test-faults`

### Depot Workflow Test Driver

Workflow forcing and state waiting lives in depot behind `test-faults`:

```text
engine/packages/depot/src/compaction/test_driver.rs
```

This wraps existing `ForceCompaction` signal behavior and manager result
waiting as a depot control surface.

## Testing API

High-level scenario shape:

```rust
FaultScenario::new("hot_install_failure_survives_reload")
	.seed(123)
	.profile(FaultProfile::Simple)
	.setup(|ctx| async move {
		ctx.sql("CREATE TABLE kv (k TEXT PRIMARY KEY, v BLOB)").await?;
		Ok(())
	})
	.workload(|ctx| async move {
		ctx.exec(LogicalOp::Put {
			key: "a".into(),
			value: vec![1, 2, 3],
		}).await?;
		ctx.checkpoint("after_first_write").await;

		let result = ctx.force_hot_compaction().await?;
		result.assert_success()?;

		ctx.reload_database().await?;

		ctx.exec(LogicalOp::Put {
			key: "b".into(),
			value: vec![4, 5, 6],
		}).await?;
		Ok(())
	})
	.faults(|faults| {
		faults
			.at(DepotFaultPoint::HotCompaction(
				HotCompactionFaultPoint::InstallBeforeRootUpdate,
			))
			.once()
			.fail("injected hot install failure");
	})
	.verify(|ctx| async move {
		ctx.verify_sqlite_integrity().await?;
		ctx.verify_against_native_oracle().await?;
		ctx.verify_depot_invariants().await?;
		Ok(())
	});
```

`FaultScenarioCtx` exposes:

```rust
ctx.sql(sql)
ctx.query(sql)
ctx.exec(LogicalOp)
ctx.checkpoint(name)

ctx.force_hot_compaction()
ctx.force_cold_compaction()
ctx.force_reclaim()
ctx.force_compaction(ForceCompactionWork)

ctx.reload_database()

ctx.verify_sqlite_integrity()
ctx.verify_against_native_oracle()
ctx.verify_depot_invariants()
ctx.replay_record()
```

`LogicalOp` applies the same operation to the Rivet SQLite connection and the
native SQLite oracle using explicit commit-result semantics. Raw SQL helpers
are still available for targeted regression tests.

## DirectStorage Changes

`DirectStorage` becomes the single test transport for the VFS suite.

Required changes:

- Remove `MockProtocol` from `tests/inline/vfs_support.rs`.
- Remove `SqliteTransport::from_mock` and `SqliteTransportInner::Test` from
  `src/vfs.rs`.
- Keep production `SqliteTransport::from_envoy` for runtime use, but do not use
  it as a VFS test variant.
- Add strict direct mode for fault tests.
- Make mirror reads impossible in strict mode. Any call to `read_mirror`,
  `fill_from_mirror`, or mirror-backed cache seeding must fail the test.
- Add strict-mode counters or sentinels proving mirror fallback and mirror seed
  paths were not touched.
- Keep the page mirror only as a diagnostic/oracle helper.
- Add `DirectStorage::evict_actor_db(actor_id)` so database reload can drop depot
  `Db` caches.
- Add workflow harness ownership so direct storage can start the depot manager
  and companion workflows for the database branch.

Fault test direct path:

```text
SQLite VFS
  -> SqliteTransport::from_direct
  -> DirectStorage strict mode
  -> depot::conveyer::Db
  -> depot compaction workflows
  -> depot cold tier
```

Required anti-mirror smoke test:

- poison the DirectStorage mirror with impossible page bytes
- reopen SQLite in strict mode
- assert reads come from depot or fail loudly
- assert the poisoned mirror bytes are never returned

Required strict-mode read evidence:

- first post-reload read increments a real depot read counter
- cold-covered post-reload reads increment a cold-tier get counter
- strict-mode tests fail if these counters do not move when expected

## Fault Boundary Classification

Every fault point must declare one boundary class:

```rust
pub enum FaultBoundary {
	PreDurableCommit,
	AmbiguousAfterDurableCommit,
	PostDurableNonData,
	ReadOnly,
	WorkflowOnly,
}
```

Rules:

- `PreDurableCommit` faults must compare against the old oracle state.
- `AmbiguousAfterDurableCommit` faults must compare a fresh post-reload canonical
  dump against exactly the old or fully-new native SQLite state before any
  oracle resync.
- `PostDurableNonData` faults, such as failed compaction wake delivery, must
  preserve the new committed state.
- `ReadOnly` faults must not change durable state.
- `WorkflowOnly` faults must not change foreground commit semantics unless they
  publish or reclaim reader-visible artifacts.

Replay records must include:

- fault boundary class
- branch head txid before and after the operation
- commit row presence for candidate txids
- whether the oracle advanced
- whether the result was old, new, or an error

## Database Reload Semantics

`ctx.reload_database()` must approximate the relevant storage effects of a clean
SQLite database unload and reload:

1. Flush and drop `NativeDatabase`.
2. Drop VFS state and page cache by construction.
3. Reopen through a fresh `NativeDatabase`.
4. Reopen through a fresh `SqliteTransport::from_direct`.
5. Evict the actor's cached depot `Db` from `DirectStorage`.
6. Keep only persisted depot state, cold-tier objects, and native oracle state.

The reopened SQLite database uses the same actor id and same depot state. The
first read after reload must come from depot, not a page mirror.

Tests should run database reload after important failure points:

- after failed commit
- after ambiguous post-commit failure
- after failed hot stage/install
- after failed cold upload/publish
- after forced compaction succeeds
- before final verification

## Depot Fault Injection API

Public API, only compiled with `depot/test-faults`:

```rust
pub struct DepotFaultController;

pub enum DepotFaultPoint {
	Commit(CommitFaultPoint),
	Read(ReadFaultPoint),
	HotCompaction(HotCompactionFaultPoint),
	ColdCompaction(ColdCompactionFaultPoint),
	Reclaim(ReclaimFaultPoint),
	ColdTier(ColdTierFaultPoint),
	ShardCacheFill(ShardCacheFillFaultPoint),
}

pub enum DepotFaultAction {
	Fail { message: String },
	Pause { checkpoint: String },
	Delay { duration: Duration },
	DropArtifact,
}
```

Rule API:

```rust
faults
	.at(DepotFaultPoint::Commit(CommitFaultPoint::BeforeHeadWrite))
	.once()
	.fail("injected commit failure");

faults
	.at(DepotFaultPoint::ColdTier(ColdTierFaultPoint::GetObject))
	.nth(3)
	.drop_artifact();

faults
	.at(DepotFaultPoint::HotCompaction(
		HotCompactionFaultPoint::AfterStageBeforeFinishSignal,
	))
	.pause("hot_staged");
```

Matching dimensions:

- fault point
- actor/database id where available for depot scoping
- database branch id where available
- checkpoint name
- invocation count
- optional page number or shard id for read/compaction points
- seed

Every fired fault appends an event to the scenario replay log.

## Fault Actions

### `Fail`

Returns an explicit error from the depot operation or workflow activity. This
is the highest-value v1 fault.

### `Pause`

Notifies the scenario that a checkpoint was reached and waits for the scenario
to release it. This is for deterministic races, not for arbitrary sleeps.

### `Delay`

Adds bounded latency at a point to expose timeout and cancellation behavior.
Use sparingly in simple CI. Chaos tests can use it more heavily.

Delays must live only inside `test-faults` fault-point dispatch. Every delay
must have a maximum duration and a replay entry. Simple CI should prefer
pause/release checkpoints except for explicit timeout tests. Timeout tests must
assert elapsed bounds and error classification, not just eventual success or
failure.

### `DropArtifact`

Drops a depot-owned semantic artifact only when the code would naturally have
to handle that artifact being absent:

- staged hot shard missing before install
- cold object missing on get
- shard cache fill skipped
- compaction wake not delivered

Do not use `DropArtifact` as raw storage mutation deletion in v1.

## Fault Points

### Commit

Hook sites in `engine/packages/depot/src/conveyer/commit/apply.rs`:

- `BeforeTx`
- `AfterBranchResolution`
- `AfterHeadRead`
- `AfterTruncateCleanup`
- `AfterLtxEncode`
- `BeforeDeltaWrites`
- `BeforePidxWrites`
- `BeforeHeadWrite`
- `BeforeCommitRows`
- `BeforeQuotaMutation`
- `AfterUdbCommit`
- `BeforeCompactionSignal`
- `AfterCompactionSignal`

Commit invariants:

- Dirty pages are validated before storage work.
- Page 0, short pages, and duplicate dirty pages are rejected by depot.
- Failed pre-commit faults leave the old oracle state.
- Post-durable-commit failures are explicitly ambiguous and must compare against
  old-or-new oracle state.
- Failed compaction wakeups do not invalidate the committed database.
- Timeout before storage work leaves the old oracle state.
- Timeout while the depot commit may be durably committing is an explicitly
  ambiguous fault point.
- Timeout after the depot commit is durable is an explicitly ambiguous fault
  point.

### Read

Hook sites in `engine/packages/depot/src/conveyer/read.rs` and read submodules:

- `BeforeScopeResolve`
- `AfterScopeResolve`
- `AfterPidxScan`
- `DeltaBlobMissing`
- `AfterDeltaBlobLoad`
- `AfterShardBlobLoad`
- `ColdRefSelected`
- `ColdObjectMissing`
- `BeforeReturnPages`
- `ShardCacheFillEnqueue`

Read invariants:

- Page 0 is rejected by depot.
- Missing delta can fall back only to valid shard/cold coverage.
- Cold object missing produces a loud error unless another valid source covers
  the page.
- Returned pages are exactly `SQLITE_PAGE_SIZE` when present.
- Reopened SQLite state matches oracle after any read fault.
- Delta chunks must be contiguous from chunk index `0..n`.
- A missing first, middle, or last delta chunk must fail loudly or fall back to
  valid proven coverage.

### Hot Compaction

Hook sites in `db_hot_compacter.rs` and manager install logic:

- `StageBeforeInputRead`
- `StageAfterInputRead`
- `StageAfterShardWrite`
- `AfterStageBeforeFinishSignal`
- `InstallBeforeStagedRead`
- `InstallAfterStagedRead`
- `InstallBeforeShardPublish`
- `InstallAfterShardPublishBeforePidxClear`
- `InstallBeforeRootUpdate`
- `InstallAfterRootUpdate`

Hot compaction invariants:

- Reader-visible `SHARD` rows are not trusted until install validates staged
  output hash and size.
- Staged hot output is never reader-visible before install.
- PIDX rows are compare-cleared only after replacement coverage exists.
- `CMP/root` advances only after publish and PIDX cleanup rules are satisfied.
- Failed/stale hot output can be retried or cleaned without losing data.
- Every SHARD blob's pages belong to the shard id in its key.
- No SHARD blob contains pages above the branch head database size for its
  `as_of_txid`.

### Cold Compaction

Hook sites in `db_cold_compacter.rs`:

- `UploadBeforeInputRead`
- `UploadAfterInputRead`
- `UploadBeforePutObject`
- `UploadAfterPutObject`
- `PublishBeforeInputRead`
- `PublishAfterInputRead`
- `PublishBeforeColdRefWrite`
- `PublishAfterColdRefWriteBeforeRootUpdate`
- `PublishAfterRootUpdate`

Cold compaction invariants:

- Cold refs are written only for objects that were uploaded and match expected
  hash/size metadata.
- No cold ref exists without verified uploaded object metadata.
- Failed publish after upload schedules cleanup.
- Missing cold objects are not silently treated as zero pages.
- Cold watermark advances only after refs are durable.
- `cold_watermark <= hot_watermark <= head_txid`.
- A put-object failure after bytes are written but before acknowledgement is an
  explicit ambiguous cold-tier artifact case. It must either be cleaned as an
  orphan or safely reused by matching hash/size.

### Reclaim

Hook sites in `db_reclaimer.rs`:

- `PlanBeforeSnapshot`
- `PlanAfterSnapshot`
- `BeforeHotDelete`
- `AfterHotDelete`
- `BeforeColdRetire`
- `AfterColdRetire`
- `BeforeColdDelete`
- `AfterColdDelete`
- `BeforeCleanupRows`

Reclaim invariants:

- Hot deltas are deleted only after replacement shard or cold coverage exists.
- Cold objects are deleted only after retire/grace rules pass.
- Restore points, forks, and PITR intervals pin required history.
- Reclaim never makes a previously readable committed page unreadable.
- Reclaim must not delete the only proven coverage for any page at or below the
  branch head database size.
- Retired cold-object fences prevent republishing deleted keys.

### Cold Tier

Use and extend the existing `FaultyColdTier` under `depot/test-faults`:

- `PutObject`
- `GetObject`
- `DeleteObjects`
- `ListPrefix`

Support `Fail` and `Delay` for all operations. Support `DropArtifact` for
`GetObject` and semantic put-after-write-before-ack cases. Keep keys as
relative object keys and preserve existing path validation.

Cold-tier fault invariants:

- failed delete leaves retired records retryable
- failed list does not permit republish of retired keys
- orphan uploads are cleaned or safely ignored by deterministic object key and
  hash checks
- missing get cannot be converted into a zero page

## Workflow Control

Add a `test-faults`-gated manager input field:

```rust
pub struct DbManagerInput {
	pub database_branch_id: DatabaseBranchId,
	#[serde(default)]
	pub actor_id: Option<String>,
	#[cfg(feature = "test-faults")]
	#[serde(default)]
	pub disable_planning_timers: bool,
}
```

When `disable_planning_timers` is true:

- manager uses `listen_n::<DbManagerSignal>(...)`
- manager does not use `listen_n_until(...)`
- no autonomous hot/cold/reclaim refresh occurs
- `ForceCompaction` still wakes the manager immediately

Forced compaction API:

```rust
pub struct DepotCompactionTestDriver;

impl DepotCompactionTestDriver {
	pub async fn start_manager(
		&self,
		database_branch_id: DatabaseBranchId,
		actor_id: Option<String>,
		disable_planning_timers: bool,
	) -> Result<Id>;

	pub async fn force_compaction(
		&self,
		manager_workflow_id: Id,
		database_branch_id: DatabaseBranchId,
		work: ForceCompactionWork,
	) -> Result<ForceCompactionResult>;
}
```

`force_compaction` sends `ForceCompaction`, waits for signal ack, then waits for
`DbManagerState.force_compactions.recent_results` to contain the request id.
This is used only as depot's compaction control surface. The fault suite does
not test Gasoline scheduling, replay, or worker durability.

Forced compaction tests must assert depot-level results:

- requested work
- attempted job kinds
- completed job ids
- skipped noop reasons
- terminal error
- resulting depot invariants

Cold-object delete grace should have a depot-local `test-faults` override so
reclaim can be forced end-to-end without waiting on real grace timers.

## Verification API

### SQLite Integrity

Run after faulted operations, after database reload, and at final verification:

```sql
PRAGMA quick_check;
PRAGMA integrity_check;
PRAGMA foreign_key_check;
```

`integrity_check` must return `ok`. `foreign_key_check` must return no rows
when the workload enables foreign keys.

### Native SQLite Oracle

The oracle is native SQLite using the same logical workload. It must not use
Depot, DirectStorage, or the VFS under test.

Commit semantics:

- Pre-commit faults compare Rivet SQLite against old oracle state.
- Successful commits apply to both Rivet SQLite and native SQLite.
- Explicitly ambiguous post-durable-commit faults compare against old or new oracle
  state, then resync the oracle to the observed Rivet state.
- Every ambiguous fault target must declare ambiguity in the fault point enum or
  metadata. Ambiguity is not inferred from test failure.
- Resync is allowed only after a fresh post-reload canonical dump exactly matches
  either the old or fully-new oracle state.

Canonical dump:

- `sqlite_schema` ordered by type, name, SQL.
- User tables ordered by primary key when available.
- Tables without primary keys ordered by all columns and stable `rowid` only
  when `rowid` is expected to be stable for the workload.
- Blobs encoded as lowercase hex.
- Values encoded with explicit type tags.
- Internal SQLite tables excluded unless explicitly targeted.

### Depot Invariant Scanner

The invariant scanner decodes depot rows directly and does not call the VFS.

Minimum checks:

- database pointer resolves to a live database branch
- branch head exists when database has committed content
- commit rows are contiguous from branch start to head
- head txid has a commit row
- PIDX rows point to existing DELTA chunks or are covered by valid SHARD/cold
  refs
- DELTA chunks decode and contain only valid page numbers and page sizes
- DELTA chunk indexes are contiguous for each txid
- SHARD blobs decode and match expected hash/size when referenced
- every SHARD row's pages belong to its key shard
- no SHARD row covers pages above the database size at its `as_of_txid`
- cold refs point to existing objects with matching size/hash when cold tier is
  enabled
- compaction root watermarks never exceed proven coverage
- `cold_watermark_txid <= hot_watermark_txid <= head_txid`
- dirty marker state agrees with head/root lag
- staged hot shards are referenced only by active jobs or cleanup paths
- cold refs do not point beyond the committed head/root
- no duplicate conflicting page coverage exists at the same txid
- retired cold object rows fence deleted keys from republish
- restore point, fork, and PITR pins prevent reclaiming required history

### Anti-Bullshit Requirements

The harness must prove it is not passing through fake shims:

- Fault tests must fail if strict direct mode is accidentally disabled.
- Fault tests must fail if VFS reads are served from DirectStorage mirror.
- Fault tests must fail if the depot `Db` cache is not evicted during
  `reload_database`.
- Fault tests must fail if the first post-reload read does not hit depot.
- Fault tests that read cold-covered pages must fail if cold-tier get counters
  do not move.
- Native oracle comparison must run in a separate SQLite connection that does
  not use the Rivet VFS.
- At least one smoke test should intentionally inject a fault that must fail
  the SQL operation and assert the expected error path.
- At least one smoke test should intentionally inject a post-commit ambiguous
  failure and verify old-or-new handling.
- Every forced compaction test must assert `ForceCompactionResult` contents:
  attempted job kinds, completed job ids, skipped noop reasons, and terminal
  error.
- Every fault rule must assert whether it fired. Unfired expected faults fail
  the test.
- Replay records must include all fired and expected-but-unfired faults.
- A production-leak check must prove `depot/test-faults` is absent from normal
  release builds.

## Edge Cases To Cover

### SQLite/VFS Boundaries

- page 1 database header creation and reopen
- empty database reopen
- explicit transaction rollback
- savepoint rollback
- schema change inside transaction
- index creation and deletion
- foreign keys with deferred constraints
- blob rows crossing many pages
- large row near page boundary
- many small commits followed by one large read
- transaction that shrinks database size
- pages above EOF return `None`
- dirty-page flush failure poisons VFS
- commit timeout before storage work
- commit timeout while depot commit may be durable
- commit timeout after depot commit is durable
- aux/temp file create/delete failures if still relevant after mock deletion

### Depot Commit/Read Boundaries

- dirty page count 0 with db size change
- duplicate dirty pages
- page 0 request
- short dirty page
- dirty page above `db_size_pages` if SQLite can produce it. If not, assert
  depot rejects it.
- LTX chunk split boundary around `DELTA_CHUNK_BYTES`
- encoded LTX delta sizes `10_000 - 1`, `10_000`, `10_000 + 1`, and multi-chunk
- missing first, middle, and last delta chunk
- multiple pages in the same shard and across shard boundary
- exact shard-boundary pages `63`, `64`, `65`, `127`, `128`, `129`
- truncate sizes `63`, `64`, and `65`
- PIDX stale but shard fallback valid
- missing delta with no shard/cold fallback fails loudly
- cold cache fill failure does not change foreground correctness
- bounded shard-cache fill queue with fill in flight during database reload
- late shard-cache fill after actor `Db` eviction does not corrupt fresh reads

### Workflow/Compaction

- forced hot compaction with no work records noop
- forced cold compaction before hot compaction records noop
- forced hot then cold then reclaim
- compaction repeated until noop across batch limits
- hot/reclaim/cold workloads crossing `CMP_FDB_BATCH_MAX_KEYS`
- hot/reclaim/cold workloads crossing `CMP_FDB_BATCH_MAX_VALUE_BYTES`
- cold workloads crossing `CMP_S3_UPLOAD_MAX_OBJECTS`
- watermarks advance monotonically across partial batches
- dirty markers are not cleared while residual work remains
- hot stage succeeds but install is delayed while the database reloads
- hot install fails after shard publish before root update
- cold upload succeeds but publish fails
- cold object missing during read after hot rows were reclaimed
- reclaim is blocked by restore point or PITR interval
- reclaim after cold publish with shortened delete grace
- stale hot/cold job finished signal is cleaned up

### Timeouts And Delays

- depot commit delay exceeds VFS call timeout where one exists
- cold tier get delay during read
- cold tier put delay during cold upload
- cold tier delete/list delay and failure during reclaim/cleanup
- flush during `Drop` times out and logs while process continues
- pause at compaction checkpoint, run read workload, then release checkpoint

## Test Classification

### Simple CI

- deterministic seeds
- no wall-clock sleeps except bounded timeouts
- one fault per test
- forced compaction only
- strict direct mode only
- native oracle and depot invariant scanner enabled
- database reload after fault and at final verification

### Chaos

- ignored by default or separate feature
- random workload generation
- random checkpoint fault schedule
- repeated database reload cycles
- delays and pauses
- forced hot/cold/reclaim sequences
- seed replay output

## Implementation Phases

### Phase 1: Single Direct Path Cleanup

- Delete `MockProtocol` and mock transport tests.
- Remove `SqliteTransport::from_mock` and `SqliteTransportInner::Test`.
- Keep production envoy runtime path but remove envoy test variance for VFS.
- Add strict DirectStorage mode.
- Make mirror fallback/cache seeding fail loudly in strict mode.
- Add the poisoned-mirror smoke test.

### Phase 2: Workflow Harness

- Add `depot/test-faults`.
- Add manager `disable_planning_timers`.
- Add `DepotCompactionTestDriver`.
- Start manager/companions from the VFS fault scenario.
- Force hot/cold/reclaim and wait on `ForceCompactionResult`.
- Assert depot-level `ForceCompactionResult` contents.

### Phase 3: Fault Controller

- Add depot fault controller and point enums.
- Add fault boundary classification.
- Add commit/read semantic hooks.
- Extend cold tier faults.
- Add pause/checkpoint support.
- Add bounded delay support for explicit timeout tests.
- Add replay records.

### Phase 4: Verification

- Add native SQLite oracle.
- Add canonical dump.
- Add depot invariant scanner.
- Add anti-mirror smoke tests.
- Add production-leak checks for `depot/test-faults`.
- Add fresh post-reload depot/cold-tier read counters.

### Phase 5: Coverage Expansion

- Add simple CI tests for highest-value fault points.
- Add chaos tests with workload generation.
- Add selected sqllogictest-style replay inputs if useful.
- Add compaction batch-limit and shard-boundary coverage.

## Open Questions

- Should `Db::commit` reject dirty pages with `pgno > db_size_pages`, or does
  SQLite ever send this during truncate/journal cleanup?
- Which existing mock-backed VFS tests represent real VFS behavior and should
  be rewritten instead of deleted?

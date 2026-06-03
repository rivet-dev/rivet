# Adversarial Review: `04-29-feat_sqlite_pitr_forking`

Five adversarial subagents reviewed the PR (119 files / ~27k LOC vs the parent stack branch). Each was scoped to one high-risk seam and asked to surface concrete bugs (file:line). 74 distinct findings, **17 BLOCKERs**.

Reviewer scopes:
1. Concurrency & OCC (cold compactor phases, eviction OCC, leases, head_txid races, takeover)
2. PITR / forking semantics (fork_actor, fork_namespace, rollback, bookmarks, ancestry cache)
3. Cold tier & DR (phases A-D orchestration, S3 publish, follow-up sweep, DR replay, fork warmup)
4. GC, eviction, retention (pin formula, 3-gate eviction, MAX_SHARD_VERSIONS_PER_SHARD, hot-tier retention sweep)
5. Engine integration & wire format (envoy-protocol v3 vbare, pegboard-envoy lifecycle, NAPI/core layering, migration tests)

---

## Three structural problems (cross-cutting)

1. **Cold tier and PITR are wired up but not actually plumbed in.** `Db::new` is used everywhere instead of `Db::new_with_cold_tier`, the cold compactor in `run_config` defaults to `DisabledColdTier`, and `handle_namespace_warmup` is a tracing-only no-op. The end-to-end PITR / restore-from-cold path is not exercised by this PR's runtime configuration.
2. **Several "concurrent race" tests are falsely-named.** `gc_pin_recompute_under_bookmark_delete_race.rs`, `eviction_during_active_read.rs`, and `concurrent_fork_during_eviction.rs` all run strictly serially. The races they claim to cover are uncovered.
3. **There is no production DR-replay code.** `dr_replay_from_s3_alone.rs` hand-rebuilds FDB inside the test using bytes the test itself wrote, then asserts it can read them. It establishes nothing about S3-alone recovery.

---

## BLOCKERs (data loss / data corruption / silent wrong reads)

| # | File | Bug |
|---|---|---|
| B1 | `compactor/cold/phase_c.rs:38-82` | Phase C never deletes its own `pending/{uuid}.marker` after success. |
| B2 | `compactor/cold/phase_b.rs:382-388` | Stale-marker sweep deletes `marker.planned_object_keys` after 10min. Those keys are the **live manifest chunk, index, pointer snapshot, and image layers of a successful pass**. Successful data is wiped 10 minutes later. |
| B3 | `compactor/cold/phase_a.rs:196-219` | Pending marker uploaded AFTER the FDB handoff (HWM invariant inverted; spec requires marker before upload). |
| B4 | `compactor/cold/phase_a.rs:271` | `register_pending_handoff` reuses prior `in_flight_uuid` after lease loss. Second compactor overwrites first's marker, orphans S3 objects. |
| B5 | `compactor/cold/phase_c.rs:39-66` | OCC fence checks `cold_drained_txid` but not `in_flight_uuid == plan.pass_uuid`. Two compactors can both commit. |
| B6 | `pump/branch.rs:612-616`, `gc/mod.rs:155-197` | `desc_pin` only ever lowered via `ByteMin`. **No path raises it back when descendants delete.** Permanently shrinks retention to dead descendants' fork points. Spec (`sqlite-pitr-fork.md:11`, line 569) explicitly forbids monotonic ratchet. |
| B7 | `compactor/compact.rs:532-559` | Universal hot retention sweep deletes `COMMITS/{txid}` + `VTX/{versionstamp}` after 7d **regardless of pins**. Breaks pinned-bookmark fork even though cold tier kept them. |
| B8 | `compactor/compact.rs:321-324` | `last_hot_pass_txid` written via plain `set` not atomic-max. Out-of-order tx commits regress it, silently re-opening the eviction OCC window. |
| B9 | `compactor/eviction/mod.rs:681-707` | `is_pinned` collapses `VERSIONSTAMP_INFINITY` and `VERSIONSTAMP_ZERO` to "unpinned"; GC code (`gc/mod.rs:226-242`) treats INFINITY as "pin everything". Eviction will evict pin-everything branches. |
| B10 | `pump/branch.rs:625,639` | `BranchState::Frozen` is **dead metadata** — repo-wide grep finds zero readers. After rollback, stale connections keep committing to the frozen branch via `cached_branch_id`. |
| B11 | `pump/commit.rs:417-442` | **Namespace fork does not COW inherited databases on write.** A commit in a forked namespace silently mutates the source namespace's database. Fork only isolates the pointer, not the data. |
| B12 | `pump/branch.rs:322-327`, `keys.rs:233/242` | `delete_database` writes a UUID-keyed tombstone; DBPTR/bookmark resolvers only check the string-keyed name tombstone. Deleted databases remain resolvable. The two tombstone schemes share a prefix, allowing decode garbage. |
| B13 | `pegboard/src/actor_sqlite.rs:30-52` | `clear_v2_storage_for_destroy` clears only legacy keys; orphans branch-scoped state, DBPTR, bookmarks, branch refcounts, and cold-tier S3 blobs. No GC pin recompute on destroy. |
| B14 | `envoy-protocol v3.bare:123-124,145-146` + `ws_to_tunnel_task.rs:625,654` | `expectedGeneration`/`expectedHeadTxid` are in the wire schema but never validated on the engine side. Stale post-rollback commits silently apply on the new branch. |
| B15 | `pegboard/src/actor_sqlite.rs:127-133` | `migrate_v1_to_v2` constructs `Db` with fresh in-process `MemoryDriver` UPS and `NodeId::new()`. Hot-compactor wakeup signals go to a pubsub no real subscriber listens to. Migrations land megabytes of delta with zero compaction trigger. |
| B16 | `pump/branch.rs:551-554` | `derive_branch_at` retention check uses `bk_pin` only, not full `gc_pin = min(root, desc, bk)`. A fork can succeed at a point whose layers have already been cold-deleted. |
| B17 | `pump/bookmark.rs:694,302` | `recompute_database_branch_bk_pin` writes `[0;16]` (ZERO) when no pins remain. Future `create_pinned_bookmark` does `ByteMin([0;16], new)` which keeps `[0;16]`. **New pin is silently lost.** |

---

## HIGH — sharp edges that will bite under load

### Concurrency / OCC

- **Phase A reads its plan with `Snapshot` isolation; Phase C never re-checks `materialized_txid` against hot-compactor advance.** Concurrent hot pass between A and C ships a stale layer to S3. (`phase_a.rs:308-334, 512`; `phase_c.rs:39-82`)
- **Phase D fork-vs-GC race.** Deletes S3 layers outside any FDB fence, with TOCTOU between `gc_pin` re-read (tx#2) and the actual `delete_objects`. `desc_pin` is written via `ByteMin` which doesn't add a read-conflict, so fork has no OCC armor against an in-flight Phase D delete. (`phase_d.rs:121-150`; `branch.rs:613-616`)
- **No checksum validation on cold-tier read fall-through.** Corrupted S3 objects silently serve wrong page bytes to SQLite. (`pump/read.rs:446-507`)
- **404 detected by `err.to_string().contains("NoSuchKey")`** — brittle across SDK versions. (`cold_tier/mod.rs:311`)
- **S3 PUT has no `If-None-Match`, no Content-MD5, no SSE.** Late-arriving retry can clobber a freshly republished key. (`cold_tier/mod.rs:286-298`)
- **Filesystem `put_object` does no `sync_all` and no rename-from-temp.** Power loss can drop or zero a layer; breaks DR claim. (`cold_tier/mod.rs:165-184`)
- **Eviction compactor has no lease renewal task.** Sweeps that exceed 30s split-brain. CLAUDE.md mandates the renewal pattern. (`eviction/mod.rs:117-209`)
- **Phase D rewrites chunk + updates index PUT, then deletes layer objects.** A crash between index PUT and DELETE leaves unreferenced orphans with no record. No list-vs-manifest reconciliation pass. (`phase_d.rs:136-150`)
- **Worker reaps lease without aborting in-flight S3 PUTs.** Cancel checks are between PUTs only; a single PUT in flight when cancel fires completes. UUID-scoped marker + pointer_snapshot orphan. (`worker.rs:518-541`)
- **Cold Phase C marks pin "Ready" by re-reading `bookmark_pinned_key`.** If `delete_pinned_bookmark` lands between Phase B upload and Phase C, the pinned record is gone and Phase C silently no-ops without cleaning the uploaded S3 pin object. (`phase_c.rs:114-118`)
- **Phase B's `versionstamp_for_txid` returns `[0;16]` when txid not in `plan.commit_rows`**, written into `LayerEntry.min/max_versionstamp`. Phase D treats zero as "unknown, do not delete" → permanent retention. (`phase_b.rs:61-66, 81-83`; `phase_d.rs:74-76`)
- **`pass_versionstamp` returns `[0;16]` when `plan.vtx_rows` is empty** (no-op pass). All fork-warmup chunks inherit the bogus zero. (`phase_b.rs:425-430`; `phase_warmup.rs:96-100`)

### Retention / GC / pins

- **`gc_pin` formula includes a `root_pin` term not in the spec.** `root_versionstamp` never decreases, so `sweep_branch_hot_history_tx` is effectively a no-op for any live branch. (`gc/mod.rs:81-88`; spec `sqlite-pitr-fork.md:565-569`)
- **`sweep_unreferenced_branch_tx` blocks deletion until `desc_pin == INFINITY`**, but nothing ever raises it to INFINITY. Combined with B6, branch metadata + hot-tier data leak forever. (`gc/mod.rs:163-165`)
- **`delete_database` does not recompute parent's `desc_pin`.** Parent's retention bounded by long-deleted forks. Subsequent `derive_branch_at` rejects forks with `ForkOutOfRetention`. (`pump/branch.rs:306-337`)
- **`recompute_database_branch_bk_pin` uses non-atomic `set` for an atomic-min field.** Saved by prefix-scan conflict range, but fragile and the `set([0;16])` semantics produce B17. (`pump/bookmark.rs:301-302, 665-694`)
- **Tombstone visibility check ignores namespace fork cap.** Tombstone written in parent *after* the fork retroactively masks the database in every child; catalog list path applies the cap correctly, so resolution disagrees with listing. (`pump/branch.rs:181-191`; `pump/bookmark.rs:510-520` vs `pump/branch.rs:710-725`)
- **`enforce_shard_version_cap` aborts entire compaction `write_batch` when one shard saturates.** A single saturated shard wedges all compaction progress for the database. (`compact.rs:439-455`)

### Forking / PITR semantics

- **`restore_to_bookmark` undo bookmark window is unpinned.** Between rollback (advances `desc_pin`) and undo `bk_pin` write, GC can clear the COMMITS/VTX rows the undo was meant to pin. (`pump/bookmark.rs:245-262`)
- **Cross-branch bookmark resolution forges via txid alone.** `ts_ms` is parsed but never compared against `commit.wall_clock_ms`. Combined with B11, **cross-tenant data resolution is plausible**. (`pump/bookmark.rs:582-600`)
- **Pinned-bookmark UPS publish is post-commit.** Process crash between commit and publish leaves the pin permanently `Pending` with no recovery sweep. (`pump/bookmark.rs:155-243`)
- **`handle_namespace_warmup` is a tracing-only no-op.** `fork_namespace` enqueues warmup but the cold compactor just logs it. (`compactor/cold/worker.rs:462-480`; `pump/branch.rs:384-392`)
- **`rollback_database` has no fence preventing GC from deleting under in-flight readers on the frozen branch.** (`pump/branch.rs:621-633`; `gc/mod.rs:155-197`)
- **First-commit on a fork doesn't validate `head_at_fork` is still present when branch has a parent.** Silently restarts at txid=1 if cleared. (`pump/commit.rs:99-120`)
- **Flattened ancestry cache not invalidated when sibling fork rotates `parent_versionstamp` cap.** `load_branch_read_plan` returns stale `max_txid` on cached lookup. (`pump/db.rs:80, 103-105`; `pump/read.rs:766`)
- **Versionstamp timeline mixing.** Forked branch's `root_versionstamp` is from parent's VTX; eviction `read_pin_txid` looks up `branch_vtx_key(child, parent_versionstamp)` which doesn't exist → `Some(0)` over-pin (fail-safe but semantically wrong). (`branch.rs:587-597`; `eviction/mod.rs:697`; `gc/mod.rs:211`)

### Engine integration / wire format

- **v2→v3 silently drops `sqlite_startup_data`** in the envoy-protocol converter. (`envoy-protocol/src/versioned.rs:827-837`)
- **WS inbound decode failures log+drop instead of closing the conn.** Violates fail-by-default rule and the `envoy↔pegboard-envoy` trust boundary. (`ws_to_tunnel_task.rs:120-126`; `tunnel_to_ws_task.rs:90-96`)
- **`now_ms` from envoy is treated as authoritative without bounding.** Propagates into `created_at_ms` and retention math; trust-boundary violation. (`ws_to_tunnel_task.rs:670`; `pump/commit.rs:158, 459, 486`)
- **`actor_dbs` cache survives DBPTR swap on other live envoy conns.** `storage_used` and `last_access_bucket` accounting continues against the wrong branch after PITR. (`pegboard-envoy/src/conn.rs:39`)
- **Typed sqlite errors stuffed into free-form `SqliteErrorResponse{message: str}`.** Runner cannot machine-discriminate fence mismatch vs quota vs internal; violates RivetError-everywhere rule. (`envoy-protocol v3.bare:131-138,151-154`; `ws_to_tunnel_task.rs:676-686`)
- **TS envoy-protocol mirror lost ~729 lines.** Verify no consumer lost types. (`engine/sdks/typescript/envoy-protocol/src/index.ts`)

---

## MEDIUM

- **Burst-mode lag computed against in-flight commit's `txid`** (should be `txid - 1`). Lag permanently ≥ 1; harmless because threshold is 1024 but the formula is wrong. (`pump/commit.rs:206-216`)
- **`MAX_PINS_PER_NAMESPACE` not pre-checked in `restore_to_bookmark`.** A concurrent pin between rollback and undo-pin write surfaces `TooManyPins` *after* the rollback already mutated the branch pointer. (`pump/bookmark.rs:245-262, 417`)
- **`recompute_database_branch_bk_pin` scans all bookmarks for the database, not just the branch.** O(database) walk per delete. Verify `bookmark_key` length-prefixes `database_id` to avoid prefix collision. (`pump/bookmark.rs:665-694`)
- **Eviction sweep tx bundles many branches into one FDB tx with O(branch × pidx) reads.** A single high-traffic commit on any covered branch rolls back the entire batch. (`eviction/mod.rs:252-291, 398-414`)
- **`dr_replay_from_s3_alone.rs` does not actually replay** — manually writes branch record + one image into a fresh FDB, then asserts it reads back. (`tests/dr_replay_from_s3_alone.rs:67-124`)
- **`cold_compactor_5xx_phase_b.rs` does not exercise 5xx** — synthetic client-side bail with no HTTP semantics; phase B has no retry path. (`tests/cold_compactor_5xx_phase_b.rs:29-37`)
- **Cold-tier read fall-through has no negative cache.** Missing layer re-fetches on every call.
- **`clear_v2_storage_for_destroy` runs after `tx.clear_subspace_range(&subspace)`** that already cleared the legacy storage rows; redundant in the legacy direction, while branch-scoped keys escape both passes. (`workflows/actor/destroy.rs:190-194`; `workflows/actor2/mod.rs:1174-1177`)
- **`migrate_v1_to_v2` no longer deletes v1 rows post-migration.** Idempotent re-entry via `load_v2_head` short-circuit, but v1 chunks live forever in UDB. (`pegboard/src/actor_sqlite.rs:95-98, 116-148`)
- **Conn-init missed-commands replay only handles `CommandStopActor` via `if let`.** Future Command variants silently no-op. CLAUDE.md requires exhaustive matches. (`pegboard-envoy/src/conn.rs:328-332`)
- **`head_at_fork` write/clear bypasses quota accounting in both directions.** Cancels by accident; any future change that adds write-side accounting without clear-side leaks quota. (`branch.rs:572-585`; `commit.rs:196-202, 233-235`)
- **`derive_branch_at` retention margin (`GC_FORK_MARGIN_TXIDS`) is absent**; only `bk_pin` strict-greater check. (`pump/branch.rs:551-554`)
- **`delete_pinned_bookmark` clears `bookmark_key(...)` (unpinned variant) which is dead code** — pinned creation only writes the pinned key. (`bookmark.rs:289-291`)
- **`envoy-client/src/envoy.rs:503-508` indentation is double-indented.** Cosmetic; reflects an incomplete merge.

---

## Triage clusters

- **B1 + B2 together are a data-loss pattern** that fires 10 minutes after every successful cold pass.
- **B6 + B7 together** mean retention/PITR is broken in two independent ways (pin can't recover from descendant delete; hot sweep ignores pins).
- **B10 + B11 + B12** mean rollback, namespace fork, and database delete all have semantic holes — the PITR primitives are not yet correct.
- **B13 + B14 + B15** mean the engine integration layer has its own correctness holes independent of the storage layer.
- **B3 + B4 + B5** are the cold-compactor-vs-cold-compactor concurrency failure modes — they compound: B5 lets two passes both commit, B4 lets the second's marker reuse the first's UUID, B3 means the sequence can crash with no marker as a safety net.

## Suggested next steps

1. Stop merging cold-compactor work until B1, B2, B3, B4, B5 are fixed and a real two-compactor-race test exists. The "live data deleted 10 minutes later" failure mode means even a single successful pass in production loses data.
2. Fix the `desc_pin` recompute path (B6) before any further work that depends on retention being correct — eviction, GC, fork-out-of-retention checks all read from a value that monotonically shrinks but never grows.
3. Decide whether `BranchState::Frozen` is a required mechanism or dead code (B10). Either remove the field and document that rollback relies on DBPTR swap + connection re-resolution, or wire enforcement into commit.
4. Decide whether namespace fork is supposed to COW data (B11). Spec implies yes; implementation says no. Test coverage for "write to forked namespace doesn't mutate parent" is missing.
5. Replace the falsely-named race tests with deterministic concurrent harnesses or remove them.
6. Either ship a real DR-replay code path or remove `dr_replay_from_s3_alone.rs` and document DR as unimplemented.

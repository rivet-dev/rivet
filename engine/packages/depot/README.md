# depot

Per-database storage engine for Rivet's SQLite-on-FDB system. Depot owns FDB-backed durability, branch/fork metadata, restore points, PITR interval bookkeeping, hot compaction, and FDB cleanup.

OSS Depot does not include S3-backed cold storage. Configs that still specify SQLite workflow cold storage should be treated as unsupported instead of silently downgrading.

## Layout

```text
src/
  conveyer/           commit/read paths, FDB keys, branch metadata, quotas
  workflows/          DB manager, hot compacter, reclaimer
  compaction/         shared planning, payloads, workflow helpers
  gc/                 branch/refcount/restore-point pin calculations
  doctor.rs           storage diagnostics
  debug.rs            historical debug reads and metadata dumps
  inspect.rs          raw FDB inspection helpers
```

## Storage Model

Commits are durable after the FDB transaction commits. Depot stores dirty pages as LTX V3 DELTA chunks plus PIDX owner rows, then wakes workflow compaction when hot lag crosses thresholds.

Reads resolve pages in this order:

1. PIDX-owned DELTA chunks.
2. Reader-visible FDB SHARD rows written by hot compaction.
3. Zero-fill only for valid gaps inside the database size.

Missing DELTA/SHARD coverage is a storage error. Reads do not fall through to object storage in OSS.

## Workflows

Each active database branch has:

- `DbManagerWorkflow` — owns compaction state and dispatches manager-authorized work.
- `DbHotCompacterWorkflow` — stages compacted FDB SHARD output and reports completion.
- `DbReclaimerWorkflow` — deletes manager-authorized FDB rows and stale staged hot output.

Hot compaction is signal-driven by `DeltasAvailable` and explicit `ForceCompaction { hot: true }`. Reclaim runs from its own manager deadline. The manager never dispatches cold upload work in OSS.

## Important Invariants

- No local SQLite database files. The VFS talks to Depot; Depot talks to FDB.
- FDB remains the authoritative store for live and retained OSS history.
- Hot compaction output is staged first, then installed by the manager after revalidation.
- Reclaim deletes only rows that manager planning proved safe against branch pins, restore-point pins, PITR coverage, and current branch state.
- `CompactionRoot` keeps legacy cold watermark fields for persisted decode compatibility, but OSS code does not update or act on them.
- Burst mode no longer grants quota relief from cold lag; quota checks use the base hot quota cap.

## Reference Docs

- `docs-internal/engine/depot.md` — system overview.
- `docs-internal/engine/sqlite/storage-structure.md` — FDB key layout.
- `docs-internal/engine/sqlite/components.md` — component responsibilities.
- `docs-internal/engine/sqlite/constraints-and-design-decisions.md` — design constraints and rationale.

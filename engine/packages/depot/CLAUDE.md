# Depot Package Notes

The per-database Depot engine. FDB is the authoritative store for OSS SQLite state. LTX V3 file format is used throughout commit and compaction payloads.

## Hard Constraints

- **No local SQLite files. Ever.** Not on disk, not on tmpfs, not as a hydrated cache file. The VFS speaks to Depot, and Depot speaks to FDB.
- **Lazy reads only.** Do not bulk pre-load database pages at open. Fetch pages on demand through PIDX/DELTA and FDB SHARD coverage.
- **No OSS cold tier.** Do not reintroduce `cold_tier`, S3 object storage, cold manifests, cold compacter workflows, cold read fallback, or shard-cache fill workers in this package.
- **Unsupported cold config must fail clearly.** Do not silently disable old `workflow_cold_storage` config shapes.
- **Workflow compaction is the only compaction authority.** Do not reintroduce standalone compactor modules or tests.
- **Manager owns publish/delete authority.** Hot and reclaim companions stage or delete only manager-authorized work.

## Read Path

- PIDX/DELTA wins.
- FDB SHARD fallback is next.
- Valid database gaps are zero-filled.
- Missing required source coverage returns a storage error.
- `debug::read_at` cannot trust PIDX alone; it scans DELTA history up to the target txid before falling through to SHARD rows and zero-fill.

## Workflow Compaction

- Keep one module per workflow: `DbManagerWorkflow`, `DbHotCompacterWorkflow`, and `DbReclaimerWorkflow`.
- `RefreshManagerOutput` carries concrete hot/reclaim planned jobs.
- `DbManagerState.active_jobs` stores concrete hot/reclaim active jobs.
- `ForceCompactionWork` supports hot, reclaim, and final settle work.
- `CompactionRoot` retains cold watermark fields for legacy persisted compatibility only.
- Reclaimer planning owns deletion eligibility from current manifest, pins, PIDX, SHARD, and staged hot output.

## Keys And Types

- Conveyer type domains live behind the `conveyer/types.rs` facade.
- Keep branch, restore point, compaction, history-pin, storage, page, and id payloads in focused `conveyer/types/*.rs` files.
- Do not add raw `serde_bare` persisted encodings; use versioned BARE (`vbare`) for persisted/wire-format data.
- When changing FDB key layout, branch metadata, or compactor responsibilities, update `docs-internal/engine/sqlite/{storage-structure,components,constraints-and-design-decisions}.md` in the same change.

## Tests

- Put Rust tests under `tests/`, not inline `#[cfg(test)] mod tests` in `src/`, unless private-module access is truly required.
- Keep test fixtures FDB-backed. Do not add filesystem object-storage stand-ins for OSS Depot.
- Fault tests should use surviving commit, read, hot compaction, and reclaim fault points.

## Reference Docs

- `docs-internal/engine/depot.md`
- `docs-internal/engine/sqlite/storage-structure.md`
- `docs-internal/engine/sqlite/components.md`
- `docs-internal/engine/sqlite/constraints-and-design-decisions.md`

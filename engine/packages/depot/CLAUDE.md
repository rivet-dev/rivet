# Depot Package Notes

The per-database Depot engine. UDB is the authoritative store for OSS SQLite state. LTX V3 file format is used throughout commit and compaction payloads.

## Hard Constraints

- **No local SQLite files. Ever.** Not on disk, not on tmpfs, not as a hydrated cache file. The VFS speaks to Depot, and Depot speaks to UDB.
- **Lazy reads only.** Do not bulk pre-load database pages at open. Fetch pages on demand through PIDX/DELTA and UDB SHARD coverage.
- **No OSS cold tier.** Do not reintroduce `cold_tier`, S3 object storage, cold manifests, cold compacter workflows, cold read fallback, or shard-cache fill workers in this package.
- **Unsupported cold config must fail clearly.** Do not silently disable old `workflow_cold_storage` config shapes.
- **Workflow compaction is the only compaction authority.** Do not reintroduce standalone compactor modules or tests.
- **Manager owns publish/delete authority.** Hot and reclaim companions stage or delete only manager-authorized work.

## Read Path

- PIDX/DELTA wins.
- UDB SHARD fallback is next.
- Valid database gaps are zero-filled.
- Missing required source coverage returns a storage error.
- `debug::read_at` cannot trust PIDX alone; it scans DELTA history up to the target txid before falling through to SHARD rows and zero-fill.

## Workflow Compaction

- Keep one module per workflow: `DbManagerWorkflow`, `DbHotCompacterWorkflow`, and `DbReclaimerWorkflow`.
- `RefreshManagerOutput` carries concrete hot/reclaim planned jobs.
- `DbManagerState.active_jobs` stores concrete hot/reclaim active jobs.
- `ForceCompactionWork` supports hot, reclaim, and final settle work.
- `CompactionRoot` retains cold watermark fields for legacy persisted compatibility only.
- The install that advances `hot_watermark_txid` is the coverage proof: DELTA rows at or below the watermark are reclaimable with no per-shard or PIDX proof.
- COMMITS/VTX below the watermark survive only as keep-set islands (pins plus retained PITR interval representatives); superseded SHARD versions die once no covered txid reads through them.
- Snapshot targets (pins, forks, restores) must be covered or above the watermark; creation paths fence on `CMP/root` serializably and snap versionstamp targets down to the newest covered point.
- Ambiguous bucket fork proofs fail-safe only commit/VTX and shard-version deletes; they never block installs or delta reclaim.
- Truncate publishes pruned SHARD versions at the truncating txid and never deletes or rewrites historical versions.

## Keys And Types

- Conveyer type domains live behind the `conveyer/types.rs` facade.
- Keep branch, restore point, compaction, history-pin, storage, page, and id payloads in focused `conveyer/types/*.rs` files.
- Do not add raw `serde_bare` persisted encodings; use versioned BARE (`vbare`) for persisted/wire-format data.
- When changing UDB key layout, branch metadata, or compactor responsibilities, update `docs-internal/engine/depot/{storage-structure,components,constraints-and-design-decisions}.md` in the same change.

## Tests

- Put Rust tests under `tests/`, not inline `#[cfg(test)] mod tests` in `src/`, unless private-module access is truly required.
- Keep test fixtures UDB-backed. Do not add filesystem object-storage stand-ins for OSS Depot.
- Fault tests should use surviving commit, read, hot compaction, and reclaim fault points.

## Reference Docs

- `docs-internal/engine/depot/overview.md` (start here: high-level system overview)
- `docs-internal/engine/depot/storage-structure.md`
- `docs-internal/engine/depot/components.md`
- `docs-internal/engine/depot/constraints-and-design-decisions.md`
- `docs-internal/engine/depot/comparison-to-other-systems.md`

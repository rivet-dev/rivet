# SQLite Requirements (v3)

Brief. Supersedes any assumption in earlier docs that contradicts these
three constraints.

## Hard constraints

1. **Single writer per database.** One actor owns one SQLite database at a
   time. There is never concurrent writing from multiple actors,
   connections, or processes. MVCC, optimistic conflict detection,
   page-versioned storage, and content-addressed dedup are unnecessary.

2. **No local SQLite files.** Ever. Not on disk, not on tmpfs, not as a
   hydrated cache file. The authoritative page store is the distributed
   KV layer, and the VFS must speak to it directly. Any design that puts
   a real SQLite file on the pegboard node is out of scope.

3. **Lazy read only.** The database does not fit in memory and we cannot
   eagerly download it at actor open. Pages are fetched on demand from
   the KV layer. Caching and prefetch amortize the per-fetch round-trip,
   but there is no bulk pre-load phase.

## What this rules out

- Local-file designs: LiteFS, libSQL embedded replicas, Turso embedded
  replicas, any plan that hydrates to a file.
- Bulk "hydrate whole database at resume" — the earlier Option F Piece 1.
- mvSQLite's MVCC, PLCC, DLCC, MPC, versionstamps, commit-intent logs,
  and content-addressed dedup. All dead weight under single-writer.
- Any plan that assumes the actor has enough RAM to hold its whole
  database.

## What this leaves on the table

- Bounded client-side page cache keyed by `(file_tag, chunk_index)`.
- Predictive prefetch at the VFS read layer: stride detection,
  sequential-scan detection, B-tree-hint-based fetches.
- Batched page fetch server op (`sqlite_read_many`) so one round-trip
  carries many pages.
- Write-path fast batching (already shipped, US-008 through US-014).
- VFS commit-boundary merging so one SQLite transaction produces one
  server write batch regardless of how many `xSync` callbacks fire.

## Drift from existing docs

`.agent/specs/sqlite-vfs-single-writer-plan.md` still lists "hydrate at
open" as Piece 1 and a 64 MiB hydration budget. Both violate constraint
3 and must be reframed as lazy-fill + bounded cache + prefetch.

`scripts/ralph/prd.json` US-025 is titled "Hydrate the actor SQLite page
cache at resume time" and its acceptance criteria describe a bulk
parallel fetch. Same drift. Needs to be rewritten to describe lazy
fill-on-miss with prefetch instead of a resume-time bulk load.

Everything else in US-020 through US-028 still holds.

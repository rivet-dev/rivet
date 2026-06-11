# SQLite PITR Comparison To Other Systems

This design borrows proven ideas from adjacent systems, but the constraints are different: Rivet has single-writer database ownership, no local SQLite files, UDB as the source of truth, and storage-level fork primitives instead of storage-level rollback.

| System | What We Share | What We Diverge On | Why |
|---|---|---|---|
| Neon | Layer model, branching, dependency-graph GC. | Rough PITR by default instead of exact PITR everywhere; UDB is the durable page store instead of a pageserver. | Exact PITR is valuable for Postgres workloads but too expensive as the default for these database databases. |
| Cloudflare Durable Objects SQLite | RestorePoint-like time tokens and the idea that snapshots can be built from log state. | Durable Objects use a follower quorum and do not expose fork primitives. | UDB replaces the multi-replica WAL quorum. Forking and bucket cloning are first-class goals here. |
| Snowflake | Time travel and zero-copy clone by metadata. | Snowflake is OLAP/table-oriented; this storage layer is per-SQLite-database and exposes lower-level primitives to the engine. | The metadata-only clone idea carries over, but the unit of identity is a database branch, not a warehouse/table abstraction. |
| LiteFS | LTX file format and high-water-mark pending markers. | LiteFS uses local SQLite files and WAL replication. This design forbids local database files and builds PITR around branches. | Stateless database hosting cannot depend on local files. Branchable storage needs graph retention, not only replica catch-up. |
| Litestream | LTX-style incremental backup and rolling post-apply checksum. | Litestream backs up one SQLite database stream. It has no branch graph, bucket fork, or UDB tier. | Litestream answers "can I restore this database?" This design answers "can I fork this database or bucket cheaply?" |
| mvSQLite | Versionstamp awareness as a concept. | mvSQLite's multi-writer PLCC/DLCC/MPC machinery and content-addressed dedup are deliberately not adopted. | Pegboard already guarantees a single writer per database. Multi-writer conflict machinery would add cost without buying correctness. |
| Turso/libSQL | Point-in-time fork/branch as a user-facing primitive. | Turso uses local SQLite files with replication and treats rollback as a storage operation. This design pushes rollback to the engine layer and exposes only fork/delete/restore_point primitives. | Keeping rollback out of storage removes mutable pointer swaps, pointer history, frozen states, and commit-vs-rollback races. |

## Rollback Ownership

Cloudflare Durable Objects, Turso, and Neon expose rollback semantics in storage. Rivet storage does not. The engine owns database lifecycle and the database-to-current-database mapping, so rollback is implemented by:

1. Resolve a restore_point or AS-OF versionstamp.
2. Call `fork_database`.
3. Point the engine's database mapping at the new database id.
4. Restart or reconnect the database against that database.

This leaves SQLite storage with one simple rule: branch ids are immutable for life.

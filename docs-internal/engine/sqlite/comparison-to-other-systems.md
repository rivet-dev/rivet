# SQLite PITR Comparison To Other Systems

This design borrows proven ideas from adjacent systems, but the constraints are different: Rivet has single-writer database ownership, no local SQLite files, FDB as the hot tier, and storage-level fork primitives instead of storage-level rollback.

| System | What We Share | What We Diverge On | Why |
|---|---|---|---|
| Neon | Layer model with image and delta layers, branching, dependency-graph GC. | Rough PITR by default instead of exact PITR everywhere; FDB is the hot source of truth instead of a pageserver; branch records are immutable and deleted by refcount/pin GC. | Exact PITR is valuable for Postgres workloads but too expensive as the default for these actor databases. FDB already provides the hot durable tier. |
| Cloudflare Durable Objects SQLite | Bookmark-like time tokens and the idea that snapshots can be built from log state. | Durable Objects use a follower quorum and do not expose fork primitives. This design uses FDB plus S3, and exposes `fork_database` and `fork_namespace`. | FDB replaces the multi-replica WAL quorum. Forking and namespace cloning are first-class goals here. |
| Snowflake | Time travel and zero-copy clone by metadata. | Snowflake is OLAP/table-oriented; this storage layer is per-SQLite-database and exposes lower-level primitives to the engine. | The metadata-only clone idea carries over, but the unit of identity is a database branch, not a warehouse/table abstraction. |
| LiteFS | LTX file format and high-water-mark pending markers. | LiteFS uses local SQLite files and WAL replication. This design forbids local database files and builds PITR around branches. | Stateless actor hosting cannot depend on local files. Branchable storage needs graph retention, not only replica catch-up. |
| Litestream | LTX-style incremental backup, rolling post-apply checksum, and S3 retention. | Litestream backs up one SQLite database stream. It has no branch graph, namespace fork, or hot FDB tier. | Litestream answers "can I restore this database?" This design answers "can I fork this database or namespace cheaply?" |
| mvSQLite | Versionstamp awareness as a concept. | mvSQLite's multi-writer PLCC/DLCC/MPC machinery and content-addressed dedup are deliberately not adopted. | Pegboard already guarantees a single writer per database. Multi-writer conflict machinery would add cost without buying correctness. |
| Turso/libSQL | Point-in-time fork/branch as a user-facing primitive. | Turso uses local SQLite files with replication and treats rollback as a storage operation. This design pushes rollback to the engine layer and exposes only fork/delete/bookmark primitives. | Keeping rollback out of storage removes mutable pointer swaps, pointer history, frozen states, and commit-vs-rollback races. |

## Rollback Ownership

Cloudflare Durable Objects, Turso, and Neon expose rollback semantics in storage. Rivet storage does not. The engine owns actor lifecycle and the actor-to-current-database mapping, so rollback is implemented by:

1. Resolve a bookmark or AS-OF versionstamp.
2. Call `fork_database`.
3. Point the engine's actor mapping at the new database id.
4. Restart or reconnect the actor against that database.

This leaves SQLite storage with one simple rule: branch ids are immutable for life.

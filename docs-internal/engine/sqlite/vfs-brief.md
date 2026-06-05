# SQLite VFS Brief

This page is intentionally brief. Full VFS rules live in [../sqlite-vfs.md](../sqlite-vfs.md), and the storage backend crash course lives in [../depot.md](../depot.md).

## Boundary

The VFS presents SQLite page reads and commits to the storage conveyer. It does not own PITR, fork metadata, or FDB compaction/reclaim. Those are storage-layer responsibilities under `engine/packages/depot/`.

## Read Shape

For page reads, the VFS asks storage for pages by database id and generation. Storage:

1. Resolves the database branch ancestry.
2. Checks database size and the current head.
3. Uses PIDX to find recent DELTA owners.
4. Falls through to the latest SHARD version at or below the read txid.
5. Zero-fills valid database gaps.

The VFS should treat missing pages above EOF differently from storage misses below EOF.

## Commit Shape

For commits, the VFS passes dirty pages to storage. Storage encodes the pages into LTX chunks, writes DELTA/PIDX rows, updates `COMMITS` and `VTX`, and advances `META/head` in one FDB transaction.

The VFS does not write local SQLite database files. Local files would break the stateless storage invariant and bypass the branch machinery.

## Reference Links

- [SQLite VFS](../sqlite-vfs.md)
- [Depot crash course](../depot.md)
- VFS source: `engine/packages/depot-client/src/`

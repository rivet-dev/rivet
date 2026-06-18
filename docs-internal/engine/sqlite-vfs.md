# SQLite VFS

Rules for the SQLite VFS implementation.

## Package boundaries

- Depot client owns native SQLite VFS and query execution in `engine/packages/depot-client/`. Core owns lifecycle, and NAPI only marshals JS types.
- RivetKit TypeScript SQLite is exposed through `@rivetkit/rivetkit-napi`, but runtime behavior stays in `engine/packages/depot-client/` and `rivetkit-core`.
- The Rust KV-backed SQLite implementation lives in `engine/packages/depot-client/src/`. When changing its on-disk or KV layout, update the internal data-channel spec in the same change.
- The VFS uses a 4 KiB chunk layout for page storage. PRAGMAs are pinned at open: `journal_mode = DELETE`, `locking_mode = EXCLUSIVE`, `auto_vacuum = NONE`. Source: `engine/packages/depot-client/src/vfs.rs`.

## VFS implementation notes

- SQLite VFS aux-file create/open paths mutate `BTreeMap` state under one write lock with `entry(...).or_insert_with(...)`. Avoid read-then-write upgrade patterns.
- SQLite VFS v2 storage keys use literal ASCII path segments under the `0x02` subspace prefix with big-endian numeric suffixes so `scan_prefix` and `BTreeMap` ordering stay numerically correct.
- SQLite v2 large commits keep legacy DELTA keys for small commits, but publish large commits through `DELTA_OBJ`, `DELTA_MANIFEST`, and `DELTA_PAGEIDX`. Tests and recovery code must treat both encodings as committed delta artifacts.

## Read-mode/write-mode connection manager

- The native connection manager is the SQLite read/write routing policy boundary. TypeScript and NAPI wrappers forward calls to native execution and must not decide routing from SQL text.
- Read mode may hold multiple read-only SQLite connections against one shared VFS context. Write mode must hold exactly one writable SQLite connection and no reader connections.
- Entering write mode stops admitting new readers, waits for active readers to release, closes idle readers, then opens or reuses the single writable connection.
- Read-pool v1 intentionally does not let readers continue during writes and does not pin per-reader head txids or snapshots. Any future design that overlaps readers with writers must add explicit snapshot fencing.

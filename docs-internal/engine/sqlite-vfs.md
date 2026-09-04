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
- SQLite v2 slow-path staging writes encoded LTX bytes directly under DELTA chunk keys. Do not expect `/STAGE` keys or a fixed one-chunk-per-page mapping in tests or recovery code.

## Deferred commits

- Deferred mode stages locally committed pages in the VFS overlay and exposes them to reads until the single background flusher receives a durable acknowledgement.
- Every flush batch carries the durable head fence captured when the batch forms. Retries reuse the same bytes and fence. A lost acknowledgement or divergent head breaks the database because durability is indeterminate.
- Flush waiters check the terminal error before sequence progress. Once broken, no sequence wait may report success and the core failure monitor stops the actor generation once.
- Close first stops new work, rolls back an open lease, drains the overlay within the retry deadline, then shuts down the flusher before releasing the VFS.
- Keep the named structures, algorithms, and invariants synchronized with the [deferred commits specification](sqlite/deferred-commits/SPEC.md).

## Read-mode/write-mode connection manager

- The native connection manager is the SQLite read/write routing policy boundary. TypeScript and NAPI wrappers forward calls to native execution and must not decide routing from SQL text.
- Read mode may hold multiple read-only SQLite connections against one shared VFS context. Write mode must hold exactly one writable SQLite connection and no reader connections.
- Entering write mode stops admitting new readers, waits for active readers to release, closes idle readers, then opens or reuses the single writable connection.
- Read-pool v1 intentionally does not let readers continue during writes and does not pin per-reader head txids or snapshots. Any future design that overlaps readers with writers must add explicit snapshot fencing.

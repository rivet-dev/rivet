# SQLite VFS parity

Rules for the SQLite VFS implementations.

## Package boundaries

- RivetKit SQLite is native-only. VFS and query execution live in `rivetkit-rust/packages/rivetkit-sqlite/`, core owns lifecycle, and NAPI only marshals JS types.
- RivetKit TypeScript SQLite is exposed through `@rivetkit/rivetkit-napi`, but runtime behavior stays in `rivetkit-rust/packages/rivetkit-sqlite/` and `rivetkit-core`.
- The Rust KV-backed SQLite implementation lives in `rivetkit-rust/packages/rivetkit-sqlite/src/`. When changing its on-disk or KV layout, update the internal data-channel spec in the same change.

## Native VFS ↔ WASM VFS parity

**The native Rust VFS and the WASM TypeScript VFS must match 1:1.** This includes:

- KV key layout and encoding
- Chunk size
- PRAGMA settings
- VFS callback-to-KV-operation mapping
- Delete/truncate strategy (both must use `deleteRange`)
- Journal mode

When changing any VFS behavior in one implementation, update the other.

- Native: `rivetkit-rust/packages/rivetkit-sqlite/src/vfs.rs`, `kv.rs`
- WASM: `rivetkit-typescript/packages/sqlite-wasm/src/vfs.ts`, `kv.ts`

The native VFS uses the same 4 KiB chunk layout and KV key encoding as the WASM VFS. Data is compatible between backends.

## VFS implementation notes

- SQLite VFS aux-file create/open paths mutate `BTreeMap` state under one write lock with `entry(...).or_insert_with(...)`. Avoid read-then-write upgrade patterns.
- SQLite VFS v2 storage keys use literal ASCII path segments under the `0x02` subspace prefix with big-endian numeric suffixes so `scan_prefix` and `BTreeMap` ordering stay numerically correct.
- SQLite v2 slow-path staging writes encoded LTX bytes directly under DELTA chunk keys. Do not expect `/STAGE` keys or a fixed one-chunk-per-page mapping in tests or recovery code.

## Read-mode/write-mode connection manager

- The native connection manager is the SQLite read/write routing policy boundary. TypeScript and NAPI wrappers forward calls to native execution and must not decide routing from SQL text.
- Read mode may hold multiple read-only SQLite connections against one shared VFS context. Write mode must hold exactly one writable SQLite connection and no reader connections.
- Entering write mode stops admitting new readers, waits for active readers to release, closes idle readers, then opens or reuses the single writable connection.
- Read-pool v1 intentionally does not let readers continue during writes and does not pin per-reader head txids or snapshots. Any future design that overlaps readers with writers must add explicit snapshot fencing.

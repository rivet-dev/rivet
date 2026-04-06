# SQLite VFS (`@rivetkit/sqlite-vfs`)

## How It Works
- SQLite issues byte-range reads/writes; VFS translates to chunked KV operations
- `CHUNK_SIZE = 4096` — each chunk is one KV key
- `xWrite`: computes touched chunks, read-modify-write for partial updates, `putBatch`
- `xRead`: fetches chunk range, copies bytes, zero-fills gaps
- Metadata (file size) stored alongside chunks via `metaKey`

## Single-Writer Model
- Actors are single-writer, so `xLock`/`xUnlock` are no-ops
- No need for WAL (its benefit is concurrent readers/writer)
- Double mutex exists: `db/mod.ts` + `vfs.ts` — redundant under single-writer

## Current Journal/WAL Status
- Actor KV path: DELETE journal mode (SQLite default), no WAL
- File-system driver: uses WAL (standard WAL, not WAL2)
- WAL not recommended for KV-backed VFS due to checkpoint traffic on high-latency KV

## Caching
- SQLite has its own page cache; VFS-level chunk cache would mostly duplicate it
- VFS cache only helps if KV RTT is very high and pages churn — treat as benchmark-driven, not default

## Pending TODOs
- Measure `xAccess` KV round-trip overhead during DB open
- Benchmark `journal_mode=PERSIST` + `journal_size_limit` (fewer KV deletes per txn)
- Fast-path delete-on-close: reuse in-memory `file.size` instead of extra `metaKey` read
- Add per-method metrics for `xOpen`/`xAccess`/`xRead`/`xWrite`/`xTruncate`/`xDelete` and KV call counts/latency
- Measure journal-file traffic volume (create/write/delete) before any IOCAP or PRAGMA changes
- Implement `xSectorSize = CHUNK_SIZE` (4096) and benchmark impact
- Reduce `xTruncate` round trips by batching last-chunk rewrite + metadata update in one `putBatch` where possible
- Validate and document page-size alignment expectations (`page_size = CHUNK_SIZE`)

## Decisions Made
- Do NOT defer metadata writes to `xSync`/`xClose` — crash risk outweighs minimal gain (metadata already batched with chunk data in `putBatch`)
- Do NOT enable `journal_mode=MEMORY`, `journal_mode=OFF`, or `synchronous=OFF`
- `journal_mode=PERSIST` is safe to switch to later (no migration needed)

## Native SQLite Backend

The WASM VFS described above has a native Rust counterpart (`@rivetkit/sqlite-native`) that statically links SQLite via napi-rs and routes VFS callbacks over a WebSocket-based KV channel protocol. The native backend shares one SQLite library across all actors (vs. one WASM module instance per actor), reducing memory overhead and removing JS from the I/O hot path. Data is fully compatible between backends. An actor can switch between WASM and native without migration.

Key implementation files:

- `rivetkit-typescript/packages/sqlite-native/` — napi-rs addon (Rust): `vfs.rs`, `kv.rs`, `channel.rs`, `protocol.rs`, `lib.rs`
- `engine/sdks/schemas/kv-channel-protocol/` — BARE schema and TypeScript codec
- `engine/packages/pegboard-kv-channel/` — engine-side KV channel WebSocket server
- `rivetkit-typescript/packages/rivetkit/src/manager/kv-channel.ts` — manager-side KV channel handler
- `rivetkit-typescript/packages/rivetkit/src/db/native-sqlite.ts` — integration and WASM fallback logic

## Future Work
- **PITR / fork**: implement at KV layer (immutable chunk versions, manifests, branch heads, GC) with SQLite layer providing snapshot boundary coordination
- **Remove double mutex** once profiled
- **IOCAP exploration (guarded)**:
  - Do not set `SQLITE_IOCAP_BATCH_ATOMIC` unless actor KV `putBatch` atomicity is proven and `xFileControl` atomic-write opcodes are handled correctly.
  - `SQLITE_IOCAP_ATOMIC4K` is only safe if single-key KV writes are crash-atomic at 4 KiB granularity.
  - Prioritize metrics and correctness proof before enabling either flag.

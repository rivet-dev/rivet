# Rust SQLite VFS With Native SQLite + Wasm Fallback

## Summary

Replace `@rivetkit/sqlite-vfs` with a Rust-backed implementation that uses embedded, official SQLite in the native dylib build and a wasm build as a fallback. The package must load a native dylib in Node.js and Bun when available, and fall back to wasm on other platforms. The public JS API remains stable so existing callers in `rivetkit/db` continue to work unchanged.

## Goals

- Provide a Rust implementation of the SQLite VFS logic while keeping the existing JS API surface.
- Use embedded, official SQLite in the native dylib build.
- Use SQLite as wasm for fallback platforms.
- Support Node.js and Bun via a native addon (dylib) that internally hosts the sqlite-wasm runtime.
- Provide a wasm fallback path for unsupported platforms.
- Preserve the current KV key format, chunking, and metadata schema so existing data remains valid.
- Allow `CHUNK_SIZE` to be configured, defaulting to 4096.
- Provide a dedicated test wrapper package for running vitest tests against an in-memory KV driver.

## Non-goals

- Changing the public `SqliteVfs` API or `KvVfsOptions` shape in a breaking way.
- Introducing new storage formats or automatic migrations.
- Adding full multi-process WAL or shared-memory support beyond what exists today.
- Replacing or changing actor KV semantics outside the VFS boundary.

## Current Behavior to Preserve

- KV key format and constants in `rivetkit-typescript/packages/sqlite-vfs/src/kv.ts`.
- Chunk size is 4096 bytes.
- File metadata schema is `FileMeta { size: u64 }`.
- WAL and shared-memory operations are not implemented in the current VFS. Behavior should remain equivalent.
- The JS API exposes `SqliteVfs.open(fileName, options)` returning a `Database` with `exec` and `close`.

## Requirements

- Must embed SQLite as wasm.
- Must support native dylib loading in Node.js and Bun.
- Must provide wasm fallback for other platforms.
- Must preserve deterministic KV behavior across runtimes.
- Must keep `@rivetkit/sqlite-vfs` as the published entrypoint for callers.

## Proposed Architecture

### High-Level Components

- `rivetkit-sqlite-vfs-core` (Rust crate)
  - Implements the KV-backed VFS logic and SQLite host bindings.
  - Defines a Rust trait for KV operations that mirrors `KvVfsOptions`.
  - Owns key formatting, chunking, and metadata encode/decode.

- `rivetkit-sqlite-vfs-wasm` (Rust crate)
  - Builds the SQLite wasm module with async host call support.
  - Exposes a wasm-friendly interface for KV callbacks.

- `rivetkit-sqlite-vfs-native` (Rust crate)
  - N-API addon that embeds the official SQLite amalgamation.
  - Exposes the JS API and bridges to KV callbacks from JS.

- `@rivetkit/sqlite-vfs` (JS package)
  - Minimal wrappers for native and wasm bindings.
  - Exposes `./native` and `./wasm` entrypoints.
  - The runtime selection and KV binding logic lives in `rivetkit/db` to reduce glue code.

- `@rivetkit/sqlite-vfs-test` (JS package)
  - Vitest wrapper package that runs the sqlite-vfs test suite against an in-memory KV driver.
  - Can target either native or wasm backend via environment selection.

### Runtime Flow

- `rivetkit/db` imports the sqlite-vfs native binding when available, otherwise imports the wasm wrapper directly.
- Both bindings expose the same minimal JS API that accepts `KvVfsOptions` and returns a `Database`.
- Runtime selection is centralized in `rivetkit/db` to minimize glue and keep the integration next to actor KV wiring.
- Optional debug override: allow an environment variable to force backend selection (see "Runtime Selection").

### SQLite Build

- Native:
  - Embed the official SQLite amalgamation directly in the native Rust crate.
  - Do not link against system SQLite to preserve feature parity.
- Wasm:
  - Build SQLite to wasm with async host call support to allow `getBatch` and `putBatch` to be awaited.
  - Follow the same feature set as the @rivetkit/sqlite build for compatibility.
  - The sqlite-wasm module must expose the VFS registration and open/exec APIs used today.

## API and ABI Design

### JavaScript API (Unchanged)

- `class SqliteVfs`
  - `open(fileName: string, options: KvVfsOptions): Promise<Database>`

- `interface KvVfsOptions`
  - `get(key: Uint8Array): Promise<Uint8Array | null>`
  - `getBatch(keys: Uint8Array[]): Promise<(Uint8Array | null)[]>`
  - `put(key: Uint8Array, value: Uint8Array): Promise<void>`
  - `putBatch(entries: [Uint8Array, Uint8Array][]): Promise<void>`
  - `deleteBatch(keys: Uint8Array[]): Promise<void>`

- `class Database`
  - `exec(sql: string, callback?: (row: unknown[], columns: string[]) => void): Promise<void>`
  - `close(): Promise<void>`

### Rust KV Trait

- `trait KvVfs`
  - `get(&self, key: &[u8]) -> Future<Option<Vec<u8>>>`
  - `get_batch(&self, keys: Vec<Vec<u8>>) -> Future<Vec<Option<Vec<u8>>>>`
  - `put(&self, key: Vec<u8>, value: Vec<u8>) -> Future<()>`
  - `put_batch(&self, entries: Vec<(Vec<u8>, Vec<u8>)>) -> Future<()>`
  - `delete_batch(&self, keys: Vec<Vec<u8>>) -> Future<()>`

### Key Encoding

- Must match `rivetkit-typescript/packages/sqlite-vfs/src/kv.ts`.
- `SQLITE_PREFIX = 9`, `META_PREFIX = 0`, `CHUNK_PREFIX = 1`.
- Chunk index encoded as big-endian `u32`.

### Chunk Size Configuration

- Default `CHUNK_SIZE` is 4096 bytes (unchanged).
- Add an optional configuration parameter (e.g. `SqliteVfsConfig`) to allow overrides.
- The override affects new databases only. Existing databases remain on 4096 unless migrated intentionally.

## Storage Strategy

- Use a snapshot-based persistence model instead of page-level VFS hooks.
- On `open`, load the full database file bytes from KV (chunked by the existing key scheme).
- On `exec` and `close`, export the full database bytes and write them back to KV in chunks.
- This preserves the exact database file bytes and keeps compatibility with existing data.

## VFS Semantics

- The VFS surface is implemented at the package boundary, but storage is snapshot-based.
- WAL and shared-memory are not used.
- Journal mode is forced to `DELETE` in native mode to ensure the main database file contains all data.

## Packaging and Distribution

### Biome-Style Native Packaging

- Follow the same pattern as Biome: a thin base package with `optionalDependencies` on platform-specific binary packages.
- Each platform-specific package contains the prebuilt `.node` addon and a tiny JS wrapper.
- The base package exposes a stable JS API and relies on Node/Bun module resolution to pull in the correct optional dependency.

- Native addon:
  - Built with N-API to support Node.js 20+ and Bun.
  - Ship per-platform binaries for macOS, Linux, and Windows.
- Wasm fallback:
  - Bundle sqlite-wasm module and JS loader.
  - Provide the same API surface.
- Provide both ESM and CJS entrypoints for Node and Bun.

## Runtime Selection

- Default: attempt native binding first, fall back to wasm.
- Optional debug override (environment variable) to force `native` or `wasm`, used for CI and troubleshooting.
  - Example name: `RIVETKIT_SQLITE_BACKEND=native|wasm`.

## Testing Strategy

- Add a new wrapper package `@rivetkit/sqlite-vfs-test` that depends on `@rivetkit/sqlite-vfs`.
- The wrapper package provides:
  - An in-memory KV driver identical to the current `sqlite-vfs.test.ts` setup.
  - A vitest runner that can target `native` or `wasm` backends via `RIVETKIT_SQLITE_BACKEND`.
- Port existing test `sqlite-vfs.test.ts` into the wrapper package and run against both native and wasm backends.
- Add new tests:
  - Data persistence across instances.
  - Reopen and schema migration smoke test.
  - Chunk boundary read/write tests.
  - Concurrent open serialization behavior.
- Add a parity test that runs the same suite against native and wasm paths.

## Migration Plan

- Keep the `@rivetkit/sqlite-vfs` package name and exports stable.
- Replace the implementation under the same entrypoint.
- Verify that `rivetkit/db` and `rivetkit/db/drizzle` continue to work unchanged.

## Risks and Mitigations

- Async host call support in sqlite-wasm:
  - Ensure asyncify or equivalent is part of the wasm build and used consistently in both runtimes.
- JS to Rust callback overhead:
  - Maintain `getBatch` and `putBatch` usage to minimize round trips.
- Locking and WAL semantics:
  - Preserve current behavior and document limits. Avoid introducing WAL unless fully implemented.

## Open Questions

- Which compile-time SQLite flags are enabled in @rivetkit/sqlite. These must be mirrored in both native and wasm builds and documented here once identified.
- Whether to expose additional debug logging flags consistent with `VFS_DEBUG`.

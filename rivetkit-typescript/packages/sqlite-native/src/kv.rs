//! KV key layout for SQLite-over-KV storage.
//!
//! This module must produce byte-identical keys to the TypeScript implementation
//! in `rivetkit-typescript/packages/sqlite-vfs/src/kv.ts`.
//!
//! Key layout:
//!   Meta key:  [SQLITE_PREFIX, SCHEMA_VERSION, META_PREFIX, file_tag]       (4 bytes)
//!   Chunk key: [SQLITE_PREFIX, SCHEMA_VERSION, CHUNK_PREFIX, file_tag, chunk_index_u32_be] (8 bytes)

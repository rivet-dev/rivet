//! Custom SQLite VFS backed by KV operations via the channel.
//!
//! Maps SQLite VFS callbacks (xRead, xWrite, xTruncate, xDelete, etc.)
//! to KV get/put/delete/deleteRange operations. Uses the same 4 KiB chunk
//! layout and key encoding as the WASM VFS (`rivetkit-typescript/packages/sqlite-vfs/src/vfs.ts`).
//!
//! End-to-end tests are in the driver test suite:
//! `rivetkit-typescript/packages/rivetkit/src/driver-test-suite/`

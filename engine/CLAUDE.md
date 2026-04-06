# Engine Notes

## VBARE migrations

When changing a versioned VBARE schema, follow the existing migration pattern.

1. Never edit an existing published `*.bare` schema in place. Add a new versioned schema instead.
2. Update the matching `versioned.rs` like this:
   - If the bytes did not change, deserialize both versions into the new wrapper variant:

   ```rust
   6 | 7 => Ok(ToClientMk2::V7(serde_bare::from_slice(payload)?))
   ```

   - If the bytes did change, write the conversion field by field.

   - Do not do this:

   ```rust
   let bytes = serde_bare::to_vec(&x)?;
   serde_bare::from_slice(&bytes)?
   ```
3. Verify the affected Rust crate still builds.
4. For the runner protocol specifically:
   - Bump both protocol constants together:
     - `engine/sdks/rust/runner-protocol/src/lib.rs` `PROTOCOL_MK2_VERSION`
     - `engine/sdks/typescript/runner/src/mod.ts` `PROTOCOL_VERSION`
   - Update the Rust latest re-export in `engine/sdks/rust/runner-protocol/src/lib.rs` to the new generated module.

## Epoxy durable keys

- All epoxy durable state lives under per-replica subspaces (`keys::subspace(replica_id)` for v2, `keys::legacy_subspace(replica_id)` for read-only legacy data). Shared key types (`KvValueKey`, `KvBallotKey`, etc.) live in `engine/packages/epoxy/src/keys/keys.rs` and new tuple segment constants go in `engine/packages/universaldb/src/utils/keys.rs`.
- When adding fields to epoxy workflow state structs, mark them `#[serde(default)]` so Gasoline can replay older serialized state.
- Epoxy integration tests that spin up `tests/common::TestCtx` must call `shutdown()` before returning.

## Concurrent containers

Never use `Mutex<HashMap<K, V>>` or `RwLock<HashMap<K, V>>`. They serialize all access behind a single lock and are extremely slow under contention. Use lock-free concurrent maps instead:

- `scc::HashMap` for general concurrent key-value storage. Use `entry_async`, `get_async`, `insert_async`, `remove_async` for async contexts. Be aware that `scc::HashMap` does not hold entries locked across `.await` points. Each async method acquires and releases its lock atomically. If you need read-then-write atomicity, use `entry_async` which holds the bucket lock for the duration of the closure, but the closure itself must be synchronous.
- `moka::Cache` when you need TTL-based expiration or bounded capacity.
- `DashMap` is also acceptable but `scc::HashMap` is preferred in this codebase.

The same applies to `Mutex<HashSet<T>>`. Use `scc::HashSet` instead.

## Test snapshots

Use `test-snapshot-gen` to generate and load RocksDB snapshots of the full UDB KV store for migration and integration tests. Scenarios produce per-replica RocksDB checkpoints stored under `engine/packages/test-snapshot-gen/snapshots/` (git LFS tracked). In tests, use `test_snapshot::SnapshotTestCtx::from_snapshot("scenario-name")` to boot a cluster from snapshot data. See `docs-internal/engine/TEST_SNAPSHOTS.md` for the full guide.

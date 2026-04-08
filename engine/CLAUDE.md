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
     - `engine/packages/runner-protocol/src/lib.rs` `PROTOCOL_MK2_VERSION`
     - `rivetkit-typescript/packages/engine-runner/src/mod.ts` `PROTOCOL_VERSION`
   - Update the Rust latest re-export in `engine/packages/runner-protocol/src/lib.rs` to the new generated module.

## Epoxy durable keys

- All epoxy durable state lives under per-replica subspaces (`keys::subspace(replica_id)` for v2, `keys::legacy_subspace(replica_id)` for read-only legacy data). Shared key types (`KvValueKey`, `KvBallotKey`, etc.) live in `engine/packages/epoxy/src/keys/keys.rs` and new tuple segment constants go in `engine/packages/universaldb/src/utils/keys.rs`.
- When adding fields to epoxy workflow state structs, mark them `#[serde(default)]` so Gasoline can replay older serialized state.
- Epoxy integration tests that spin up `tests/common::TestCtx` must call `shutdown()` before returning.

## Test snapshots

Use `test-snapshot-gen` to generate and load RocksDB snapshots of the full UDB KV store for migration and integration tests. Scenarios produce per-replica RocksDB checkpoints stored under `engine/packages/test-snapshot-gen/snapshots/` (git LFS tracked). In tests, use `test_snapshot::SnapshotTestCtx::from_snapshot("scenario-name")` to boot a cluster from snapshot data. See `docs-internal/engine/TEST_SNAPSHOTS.md` for the full guide.

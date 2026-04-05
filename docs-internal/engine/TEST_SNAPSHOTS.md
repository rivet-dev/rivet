# Test Snapshots (test-snapshot-gen)

Generate and load RocksDB snapshots of the full UniversalDB KV store for integration and migration tests.

## Overview

The `test-snapshot-gen` crate (`engine/packages/test-snapshot-gen/`) provides:

1. **A binary** (`test-snapshot-gen`) that runs scenarios to generate RocksDB snapshots.
2. **A library** (`test_snapshot`) that loads those snapshots into test infrastructure.

Snapshots capture the entire UDB state (epoxy, gasoline, pegboard, etc.) for each replica in a multi-node cluster. They are stored as raw RocksDB checkpoint directories tracked by git LFS.

## Generating Snapshots

### Running the generator

```bash
# List available scenarios
cargo run -p test-snapshot-gen -- list

# Build a specific scenario
cargo run -p test-snapshot-gen -- build epoxy-v1
```

### Snapshot storage

Each scenario produces a single snapshot directory:

```
engine/packages/test-snapshot-gen/snapshots/{scenario}/
  metadata.json      # commit, branch, timestamp
  replica-1/         # RocksDB checkpoint
  replica-2/         # RocksDB checkpoint
```

Regenerating a scenario overwrites the previous snapshot in place.

### Adding a new scenario

1. Create a new file in `engine/packages/test-snapshot-gen/src/scenarios/`.
2. Implement the `Scenario` trait:
   - `name()` - unique scenario name (used as directory name, e.g. `"epoxy-v1"`)
   - `replica_count()` - number of replicas in the cluster
   - `populate()` - write state through normal APIs (epoxy propose, UDB transactions, etc.)
3. Register it in `scenarios::all()`.
4. Run `cargo run -p test-snapshot-gen -- build <name>` and commit the result.

### Cross-version snapshots (e.g. v1 to v2 migration)

To generate a snapshot that captures state from a different code version, you need to run the generator on a branch where that code version exists. The scenario code itself must also exist on that branch.

The typical workflow:

1. Write the scenario on your feature branch first (e.g. `epoxy_keys.rs`).
2. Create a worktree from the target branch and copy the scenario code into it.
3. Build and run the scenario in the worktree.
4. Copy the generated snapshot back to your feature branch.

```bash
# Create a worktree from the branch with the target code version
git worktree add /tmp/rivet-main main

# Copy the test-snapshot-gen crate into the worktree
cp -r engine/packages/test-snapshot-gen /tmp/rivet-main/engine/packages/test-snapshot-gen
# Add it to the worktree's Cargo.toml workspace members and dependencies

# Build and run the scenario in the worktree
cd /tmp/rivet-main
cargo run -p test-snapshot-gen -- build epoxy-v1

# Copy the snapshot back to your feature branch
cp -r engine/packages/test-snapshot-gen/snapshots/epoxy-v1 \
      /path/to/feature-branch/engine/packages/test-snapshot-gen/snapshots/epoxy-v1

# Clean up
git worktree remove /tmp/rivet-main
```

If your scenario only writes data through stable APIs that haven't changed between versions (e.g. `propose::Input`), you can generate the snapshot directly on your feature branch instead.

## Loading Snapshots in Tests

Add `test-snapshot-gen` as a dev-dependency:

```toml
[dev-dependencies]
test-snapshot-gen.workspace = true
```

### Using SnapshotTestCtx

The simplest way to load a snapshot is with `SnapshotTestCtx`, which boots a full multi-replica cluster from snapshot data:

```rust
use test_snapshot::SnapshotTestCtx;

#[tokio::test(flavor = "multi_thread")]
async fn my_migration_test() {
    // Load snapshot and start replicas (no coordinator).
    let mut test_ctx = SnapshotTestCtx::from_snapshot("epoxy-v1")
        .await
        .unwrap();

    let replica_id = test_ctx.leader_id;
    let ctx = test_ctx.get_ctx(replica_id);

    // Run your workflow, read data, assert results...

    test_ctx.shutdown().await.unwrap();
}
```

If your test also needs the epoxy coordinator running:

```rust
let mut test_ctx = SnapshotTestCtx::from_snapshot_with_coordinator("epoxy-v1")
    .await
    .unwrap();
```

### Lower-level API

For custom setups, use `load_snapshot` directly:

```rust
use test_snapshot::load_snapshot;

let test_id = uuid::Uuid::new_v4();
let replica_paths = load_snapshot("epoxy-v1", test_id).unwrap();
// replica_paths: HashMap<ReplicaId, PathBuf>
// Each path is a temp copy of the snapshot RocksDB, ready for setup_single_datacenter.
```

## How It Works

1. The generator boots a `TestCluster` (same infrastructure as epoxy integration tests).
2. The scenario's `populate()` writes state through normal APIs.
3. Each replica's RocksDB is checkpointed via `universaldb::Database::checkpoint()`.
4. A `metadata.json` file is written with the commit hash, branch name, and timestamp.
5. The test loader copies the checkpoint directory to `$TMPDIR/rivet-test-{test_id}-{dc_label}`, which is the same path that `rivet_test_deps::setup_single_datacenter` creates. Since the directory already exists with data, the RocksDB driver opens it and finds the pre-populated state.

## Git LFS

All files under `engine/packages/test-snapshot-gen/snapshots/` are tracked by git LFS (configured in `.gitattributes`). Make sure git LFS is installed before committing snapshots.

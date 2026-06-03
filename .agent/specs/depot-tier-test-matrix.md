# Depot Cold-Tier Test Matrix Helper

## Problem

Most depot tests are tier-blind: they exercise commit, branch, fork,
restore-point, and read paths that should behave identically whether the cold
tier is configured or not. Today they're not actually verified that way —
`fork_database.rs`, `fork_bucket.rs`, `restore_points.rs` have **zero** cold
coverage, and `conveyer_branch.rs` / `conveyer_restore_point.rs` / `gc.rs` are
under half. Cold-disabled is a real code path (CLAUDE.md: "cold-disabled reads
must fail on cold-only coverage" with `ShardCoverageMissing`), so single-mode
tests can mask regressions.

We want one test body that runs against both `TierMode::Disabled` and
`TierMode::Filesystem` without proc macros, without new dependencies, and
without requiring each call site to write a for-loop.

## Helper

Add to `engine/packages/depot/tests/common/mod.rs`:

```rust
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use anyhow::{Context, Result};
use tempfile::TempDir;

use depot::{
    cold_tier::{ColdTier, FilesystemColdTier},
    conveyer::Db,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TierMode {
    Disabled,
    Filesystem,
}

impl TierMode {
    pub fn label(self) -> &'static str {
        match self {
            TierMode::Disabled => "cold_disabled",
            TierMode::Filesystem => "cold_filesystem",
        }
    }
}

pub struct TestDb {
    pub db: Db,
    pub udb: Arc<universaldb::Database>,
    pub bucket_id: Id,
    pub database_id: String,
    pub cold_tier: Option<Arc<dyn ColdTier>>,
    _udb_dir: TempDir,
    _cold_dir: Option<TempDir>,
}

pub async fn build_test_db(prefix: &str, tier: TierMode) -> Result<TestDb> {
    let (udb, udb_dir) = test_db_with_dir(prefix).await?;
    let bucket_id = Id::new_v1(NodeId::new());
    let database_id = format!("{prefix}-db");

    let (db, cold_tier, cold_dir) = match tier {
        TierMode::Disabled => (
            Db::new(udb.clone(), bucket_id, database_id.clone(), NodeId::new()),
            None,
            None,
        ),
        TierMode::Filesystem => {
            let dir = tempfile::tempdir()?;
            let tier: Arc<dyn ColdTier> =
                Arc::new(FilesystemColdTier::new(dir.path(), NodeId::new())?);
            let db = Db::new_with_cold_tier(
                udb.clone(),
                bucket_id,
                database_id.clone(),
                NodeId::new(),
                tier.clone(),
            );
            (db, Some(tier), Some(dir))
        }
    };

    Ok(TestDb {
        db,
        udb,
        bucket_id,
        database_id,
        cold_tier,
        _udb_dir: udb_dir,
        _cold_dir: cold_dir,
    })
}

/// Runs `body` once per cold-tier mode against a fresh `TestDb`. Failures are
/// wrapped with the tier label so test output names the failing case.
pub async fn test_matrix<F>(prefix: &str, body: F) -> Result<()>
where
    F: Fn(TierMode, TestDb) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>,
{
    for tier in [TierMode::Disabled, TierMode::Filesystem] {
        let ctx = build_test_db(prefix, tier)
            .await
            .with_context(|| format!("[{}] failed to build TestDb", tier.label()))?;
        body(tier, ctx)
            .await
            .with_context(|| format!("[{}] body failed", tier.label()))?;
    }
    Ok(())
}
```

The closure signature `F: Fn(TierMode, TestDb) -> Pin<Box<dyn Future + Send>>`
matches the existing convention at
`engine/packages/engine/tests/common/test_runner.rs:1293-1302`. No proc macros,
no new deps.

## Test usage

```rust
#[tokio::test]
async fn fork_preserves_committed_rows() -> Result<()> {
    test_matrix("fork-preserve", |_tier, ctx| Box::pin(async move {
        // body uses ctx.db, ctx.cold_tier, ctx.udb, etc.
        Ok(())
    }))
    .await
}
```

The test runs twice in one `#[tokio::test]` invocation. A failure surfaces as
`fork_preserves_committed_rows: [cold_disabled] body failed: …` (or
`[cold_filesystem]`), so one test name in cargo output covers both modes and
the failure context names the case.

## When to bypass the matrix

- **Cold-tier-specific tests** (S3 retire, cold manifest, object upload):
  call `build_test_db(prefix, TierMode::Filesystem)` directly. Skip the
  matrix.
- **Disabled-only tests** (e.g. "cold-disabled returns
  `ShardCoverageMissing`"): call `build_test_db(prefix, TierMode::Disabled)`
  directly.
- **Tier-conditional behavior** ("cold tier writes object iff enabled"): keep
  outside the matrix and write two distinct tests with descriptive names.

Use the matrix only for tests that should be tier-agnostic — commit, branch,
fork, restore-point CRUD, pidx page lookup, GC pin computation. Wherever the
cold tier should be invisible to the assertion, run both modes.

## Migration order

1. **`fork_database.rs`** (12 tests, zero cold coverage), **`fork_bucket.rs`**
   (8 tests, zero), **`restore_points.rs`** (10 tests, zero). Convert each test
   to `test_matrix(...)`.
2. **`conveyer_branch.rs`** (~44%), **`conveyer_restore_point.rs`** (~43%),
   **`gc.rs`** (~42%). Audit each test — matrix the tier-agnostic ones, leave
   tier-specific ones alone.
3. **`workflow_compaction_skeletons.rs`** (152 tests, ~18% cold today). Most
   are inherently cold-related. Identify the subset that should pass with
   `Disabled` (commit, dirty-marker, manager planning without cold work) and
   matrix only those.

## Out of scope

- Removing `rstest`. It was previously declared in `engine/packages/engine/Cargo.toml`
  and the workspace Cargo.toml but never imported in any `.rs` file; cleanup is
  a separate change.
- Cold-tier matrix coverage in non-depot crates (rivetkit-sqlite VFS, pegboard).
  Those test transports, not depot directly.

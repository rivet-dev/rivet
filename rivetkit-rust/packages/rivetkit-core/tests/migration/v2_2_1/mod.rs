#![cfg(feature = "sqlite")]

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use tokio::process::Command;

use crate::common::ctx::IntegrationCtx;

const ACTOR_NAME: &str = "actor-v2-2-1-baseline";
const BENCH_ACTOR_NAME: &str = "actor-v2-2-1-migration-bench";

#[tokio::test(flavor = "multi_thread")]
async fn actor_v2_2_1_snapshot_starts_in_current_rivetkit_core() -> Result<()> {
	// This Rust test is the engine and snapshot harness. The current RivetKit actor
	// implementation lives in this module's scripts/current-verify.ts fixture.
	let mut ctx = IntegrationCtx::builder()
		.import_snapshot(module_dir().join("snapshot"))
		.start()
		.await?;

	ctx.actor_by_name(ACTOR_NAME).await?;
	run_current_rivetkit_verifier(&ctx).await?;
	ctx.restart_engine().await?;
	ctx.actor_by_name(ACTOR_NAME).await?;
	run_current_rivetkit_verifier(&ctx).await?;
	ctx.shutdown().await?;

	Ok(())
}

async fn run_current_rivetkit_verifier(ctx: &IntegrationCtx) -> Result<()> {
	let script_path = module_dir().join("scripts/current-verify.ts");
	let output = tokio::time::timeout(
		Duration::from_secs(90),
		Command::new("pnpm")
			.arg("exec")
			.arg("tsx")
			.arg(&script_path)
			.current_dir(workspace_root().join("rivetkit-typescript/packages/rivetkit"))
			.env("RIVET_ENDPOINT", ctx.endpoint())
			.env("RIVET_TOKEN", "dev")
			.env("RIVET_NAMESPACE", "default")
			.stdin(Stdio::null())
			.output(),
	)
	.await
	.context("timed out running current RivetKit v2.2.1 verifier")?
	.context("run current RivetKit v2.2.1 verifier")?;

	if !output.status.success() {
		bail!(
			"current RivetKit v2.2.1 verifier failed with {}\n\nverifier stdout:\n{}\n\nverifier stderr:\n{}\n\nengine stdout:\n{}\n\nengine stderr:\n{}",
			output.status,
			String::from_utf8_lossy(&output.stdout),
			String::from_utf8_lossy(&output.stderr),
			ctx.engine_stdout_tail(),
			ctx.engine_stderr_tail()
		);
	}

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "manual large migration benchmark"]
async fn benchmark_large_v2_2_1_snapshot_migration() -> Result<()> {
	let snapshot = std::env::var("RIVETKIT_MIGRATION_BENCH_SNAPSHOT")
		.context("RIVETKIT_MIGRATION_BENCH_SNAPSHOT is required")?;
	let rows = std::env::var("RIVETKIT_MIGRATION_BENCH_ROWS")
		.context("RIVETKIT_MIGRATION_BENCH_ROWS is required")?;
	let value_bytes = std::env::var("RIVETKIT_MIGRATION_BENCH_VALUE_BYTES")
		.context("RIVETKIT_MIGRATION_BENCH_VALUE_BYTES is required")?;

	let ctx = IntegrationCtx::builder()
		.import_snapshot(snapshot)
		.actor_start_threshold_ms(15 * 60 * 1_000)
		.start()
		.await?;
	ctx.actor_by_name(BENCH_ACTOR_NAME).await?;
	let first_start = run_migration_benchmark(&ctx, &rows, &value_bytes).await?;
	println!("migration_bench_engine phase=first_start {first_start}");
	ctx.shutdown().await?;

	Ok(())
}

async fn run_migration_benchmark(
	ctx: &IntegrationCtx,
	rows: &str,
	value_bytes: &str,
) -> Result<String> {
	let script_path = module_dir().join("scripts/bench-current.ts");
	let output = tokio::time::timeout(
		Duration::from_secs(15 * 60),
		Command::new("pnpm")
			.arg("exec")
			.arg("tsx")
			.arg(&script_path)
			.current_dir(workspace_root().join("rivetkit-typescript/packages/rivetkit"))
			.env("RIVET_ENDPOINT", ctx.endpoint())
			.env("RIVET_TOKEN", "dev")
			.env("RIVET_NAMESPACE", "default")
			.env("RIVETKIT_MIGRATION_BENCH_ROWS", rows)
			.env("RIVETKIT_MIGRATION_BENCH_VALUE_BYTES", value_bytes)
			.stdin(Stdio::null())
			.output(),
	)
	.await
	.context("timed out running large migration benchmark")?
	.context("run large migration benchmark")?;

	if !output.status.success() {
		bail!(
			"large migration benchmark failed with {}\n\nbenchmark stdout:\n{}\n\nbenchmark stderr:\n{}\n\nengine stdout:\n{}\n\nengine stderr:\n{}",
			output.status,
			String::from_utf8_lossy(&output.stdout),
			String::from_utf8_lossy(&output.stderr),
			ctx.engine_stdout_tail(),
			ctx.engine_stderr_tail()
		);
	}

	Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn module_dir() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/migration/v2_2_1")
}

fn workspace_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.ancestors()
		.nth(3)
		.expect("rivetkit-core should live under the workspace root")
		.to_path_buf()
}

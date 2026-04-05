use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::Serialize;

mod scenarios;
mod test_cluster;

/// Generate UDB snapshots for integration and migration tests.
///
/// Each scenario sets up a multi-replica cluster, writes state through
/// normal APIs, then checkpoints each replica's RocksDB into a snapshot
/// directory that can be loaded by tests.
#[derive(Parser)]
#[command(name = "test-snapshot-gen")]
struct Cli {
	#[command(subcommand)]
	command: Command,
}

#[derive(Subcommand)]
enum Command {
	/// Build a single scenario's snapshot.
	Build {
		/// Scenario name (e.g. "epoxy-v1").
		scenario: String,
	},
	/// List available scenarios.
	List,
}

#[derive(Serialize)]
struct SnapshotMetadata {
	commit: String,
	branch: String,
	generated_at: String,
}

fn snapshot_base_dir() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("snapshots")
}

fn get_commit_hash() -> Result<String> {
	let output = std::process::Command::new("git")
		.args(["rev-parse", "--short", "HEAD"])
		.output()
		.context("failed to run git")?;
	Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn get_branch_name() -> Result<String> {
	let output = std::process::Command::new("git")
		.args(["rev-parse", "--abbrev-ref", "HEAD"])
		.output()
		.context("failed to run git")?;
	Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn get_iso_timestamp() -> String {
	let secs = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_secs();
	// Simple ISO-ish format from epoch. Not worth adding chrono for this.
	format!("{secs}")
}

#[tokio::main]
async fn main() -> Result<()> {
	gas::ctx::test::setup_logging();

	let cli = Cli::parse();
	let all_scenarios = scenarios::all();

	match cli.command {
		Command::List => {
			for s in &all_scenarios {
				println!(
					"{} ({} replica{})",
					s.name(),
					s.replica_count(),
					if s.replica_count() == 1 { "" } else { "s" }
				);
			}
		}
		Command::Build { scenario } => {
			let s = all_scenarios
				.iter()
				.find(|s| s.name() == scenario)
				.with_context(|| format!("unknown scenario: {scenario}"))?;
			run_scenario(s.as_ref()).await?;
		}
	}

	Ok(())
}

async fn run_scenario(scenario: &dyn scenarios::Scenario) -> Result<()> {
	let name = scenario.name();
	tracing::info!(%name, "running scenario");

	let scenario_dir = snapshot_base_dir().join(name);

	// Clean previous snapshot if it exists.
	if scenario_dir.exists() {
		std::fs::remove_dir_all(&scenario_dir).context("failed to remove old snapshot")?;
	}
	std::fs::create_dir_all(&scenario_dir).context("failed to create snapshot directory")?;

	// Build the cluster.
	let replica_ids: Vec<u64> = (1..=scenario.replica_count() as u64).collect();
	let mut cluster = test_cluster::TestCluster::new(&replica_ids).await?;

	// Run the scenario to populate state.
	scenario.populate(&cluster).await?;

	// Checkpoint each replica.
	for &replica_id in &replica_ids {
		let checkpoint_path = scenario_dir.join(format!("replica-{replica_id}"));
		tracing::info!(%replica_id, path = %checkpoint_path.display(), "checkpointing replica");

		let ctx = cluster.get_ctx(replica_id);
		ctx.udb()?
			.checkpoint(&checkpoint_path)
			.context("failed to checkpoint")?;
	}

	cluster.shutdown().await?;

	// Write metadata.
	let metadata = SnapshotMetadata {
		commit: get_commit_hash()?,
		branch: get_branch_name()?,
		generated_at: get_iso_timestamp(),
	};
	let metadata_path = scenario_dir.join("metadata.json");
	std::fs::write(&metadata_path, serde_json::to_string_pretty(&metadata)?)
		.context("failed to write metadata")?;

	tracing::info!(path = %scenario_dir.display(), "snapshot saved");
	println!("snapshot: {}", scenario_dir.display());

	Ok(())
}

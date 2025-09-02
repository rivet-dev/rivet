use std::{path::PathBuf, sync::Arc};

use anyhow::*;
use clap::Parser;
use rivet_engine::{SubCommand, run_config};

#[derive(Parser)]
#[command(name = "Rivet", version, about)]
struct Cli {
	#[command(subcommand)]
	command: SubCommand,

	/// Path to the config file or directory of config files
	#[clap(long, global = true)]
	config: Vec<PathBuf>,
}

fn main() -> Result<()> {
	rivet_runtime::run(main_inner()).transpose()?;
	Ok(())
}

async fn main_inner() -> Result<()> {
	let cli = Cli::parse();

	// Load config
	let config = rivet_config::Config::load(&cli.config).await?;
	tracing::info!(config = ?*config, "loaded config");

	// Build run config
	let run_config = Arc::new(run_config::config(config.clone())?);

	// Execute command
	cli.command.execute(config, run_config).await
}

use std::{
	path::{Path, PathBuf},
	sync::Arc,
	time::Duration,
};

use anyhow::*;
use clap::Parser;
use rivet_edge_server::run_config;
use rivet_server_cli::SubCommand;

// 7 day logs retention
const LOGS_RETENTION: Duration = Duration::from_secs(7 * 24 * 60 * 60);

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
	rivet_runtime::run(async { main_inner().await })??;
	Ok(())
}

async fn main_inner() -> Result<()> {
	let cli = Cli::parse();

	// Load config
	let config = rivet_config::Config::load(&cli.config)
		.await
		.map_err(|err| anyhow!("{err:?}"))?;

	// TODO: Remove, hardcoded for testing
	std::fs::create_dir_all("/var/lib/rivet-sqlite")?;

	// Setup logs
	if config
		.server()
		.ok()
		.and_then(|x| x.rivet.edge.as_ref())
		.and_then(|x| x.redirect_logs)
		.unwrap_or_default()
	{
		let logs_path = Path::new("/var/log/rivet-edge-server");
		std::fs::create_dir_all(logs_path)?;
		rivet_logs::Logs::new(logs_path.to_path_buf(), LOGS_RETENTION)
			.start()
			.await?;
	}

	// Build run config
	let run_config = Arc::new(run_config::config(config.clone())?);

	// Execute command
	cli.command.execute(config, run_config).await
}



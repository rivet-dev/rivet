use anyhow::*;
use clap::Parser;
use rivet_cli::SubCommand;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "Rivet", version, about)]
struct Cli {
	#[command(subcommand)]
	command: SubCommand,
}

fn main() -> Result<()> {
	rivet_runtime::run(async { main_inner().await })??;
	Ok(())
}

async fn main_inner() -> Result<()> {
	let run_config = Arc::new(rivet_cli::run_config::default_config()?);

	let cli = Cli::parse();
	cli.command.execute(run_config).await
}

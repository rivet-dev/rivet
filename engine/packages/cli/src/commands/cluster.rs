use anyhow::Result;
use clap::{Parser, Subcommand};

mod logs;

#[derive(Parser)]
pub struct Opts {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// View a cluster's engine and runner logs.
	Logs(logs::Opts),
}

impl Opts {
	pub async fn execute(self) -> Result<()> {
		match self.command {
			Commands::Logs(opts) => opts.execute().await,
		}
	}
}

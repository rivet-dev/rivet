use anyhow::Result;
use clap::{Parser, Subcommand};

mod env;
mod logs;
mod secret;

#[derive(Parser)]
pub struct Opts {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// View a cluster's engine and runner logs.
	Logs(logs::Opts),
	/// Manage the cluster's secrets, which runner pools' env can name.
	Secret(secret::Opts),
	/// Manage a runner pool's environment variables.
	Env(env::Opts),
}

impl Opts {
	pub async fn execute(self) -> Result<()> {
		match self.command {
			Commands::Logs(opts) => opts.execute().await,
			Commands::Secret(opts) => opts.execute().await,
			Commands::Env(opts) => opts.execute().await,
		}
	}
}

use anyhow::*;
use clap::Parser;
use commands::*;

pub mod commands;
pub mod run_config;
pub mod util;

#[derive(Parser)]
pub enum SubCommand {
	/// Starts the Rivet server
	Server(server::Opts),
	/// Provisions all of the required resources to run Rivet.
	///
	/// If you need to provision specific parts, use the `rivet db migrate up` and `rivet storage
	/// provision` commands.
	Provision(provision::Opts),
	/// Manages databases
	#[clap(alias = "db")]
	Database {
		#[clap(subcommand)]
		command: db::SubCommand,
	},
	/// Manages buckets
	Storage {
		#[clap(subcommand)]
		command: storage::SubCommand,
	},
	/// Manages workflows
	#[clap(alias = "wf")]
	Workflow {
		#[clap(subcommand)]
		command: wf::SubCommand,
	},
}

impl SubCommand {
	pub async fn execute(self, run_config: run_config::RunConfig) -> Result<()> {
		match self {
			SubCommand::Server(opts) => opts.execute(&run_config).await,
			SubCommand::Provision(opts) => opts.execute(&run_config).await,
			SubCommand::Database { command } => command.execute(&run_config).await,
			SubCommand::Storage { command } => command.execute(&run_config).await,
			SubCommand::Workflow { command } => command.execute().await,
		}
	}
}

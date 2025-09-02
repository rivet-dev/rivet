use anyhow::*;
use clap::Parser;
use commands::*;
use rivet_service_manager::RunConfig;

pub mod commands;
pub mod run_config;
pub mod util;

#[derive(Parser)]
pub enum SubCommand {
	/// Starts the Rivet server
	Start(start::Opts),
	/// Manages databases
	#[clap(alias = "db")]
	Database {
		#[clap(subcommand)]
		command: db::SubCommand,
	},
	/// Manages workflows
	#[clap(alias = "wf")]
	Workflow {
		#[clap(subcommand)]
		command: wf::SubCommand,
	},
	/// Manage the Rivet config
	Config {
		#[clap(subcommand)]
		command: config::SubCommand,
	},
	/// Allows inspection of UDB data
	Udb(udb::Opts),
}

impl SubCommand {
	pub async fn execute(self, config: rivet_config::Config, run_config: RunConfig) -> Result<()> {
		match self {
			SubCommand::Start(opts) => opts.execute(config, &run_config).await,
			SubCommand::Database { command } => command.execute(config).await,
			SubCommand::Workflow { command } => command.execute(config).await,
			SubCommand::Config { command } => command.execute(config).await,
			SubCommand::Udb(opts) => opts.execute(config).await,
		}
	}
}

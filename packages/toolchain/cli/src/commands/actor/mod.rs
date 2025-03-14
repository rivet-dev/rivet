pub mod create;
pub mod destroy;
pub mod get;
pub mod list;
pub mod logs;

use anyhow::*;
use clap::Subcommand;

/// Commands for managing actors
#[derive(Subcommand)]
pub enum SubCommand {
	Create(create::Opts),
	Get(get::Opts),
	Destroy(destroy::Opts),
	List(list::Opts),
	Logs(logs::Opts),
}

impl SubCommand {
	pub async fn execute(&self) -> Result<()> {
		match &self {
			SubCommand::Create(opts) => opts.execute().await,
			SubCommand::Get(opts) => opts.execute().await,
			SubCommand::Destroy(opts) => opts.execute().await,
			SubCommand::List(opts) => opts.execute().await,
			SubCommand::Logs(opts) => opts.execute().await,
		}
	}
}

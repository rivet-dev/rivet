use anyhow::*;
use clap::Parser;

pub mod endpoint;
pub mod list;

/// Commands for managing routes
#[derive(Parser)]
pub enum SubCommand {
	/// List all routes
	List(list::Opts),
	/// Create or update an endpoint (route)
	#[clap(alias = "ep")]
	Endpoint(endpoint::Opts),
}

impl SubCommand {
	pub async fn execute(&self) -> Result<()> {
		match self {
			SubCommand::List(opts) => opts.execute().await,
			SubCommand::Endpoint(opts) => opts.execute().await,
		}
	}
}

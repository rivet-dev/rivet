use anyhow::*;
use clap::Parser;

pub mod endpoint;
pub mod list;

/// Commands for managing routes
#[derive(Parser)]
pub enum SubCommand {
	/// List all routes
	List(list::Opts),
	/// Get information about a specific route endpoint
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

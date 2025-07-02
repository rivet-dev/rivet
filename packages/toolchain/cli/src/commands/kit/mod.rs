use anyhow::*;
use clap::Parser;

pub mod endpoint;

/// Commands for managing RivetKit
#[derive(Parser)]
pub enum SubCommand {
	/// Get the RivetKit endpoint
    Endpoint(endpoint::Opts),
}

impl SubCommand {
    pub async fn execute(&self) -> Result<()> {
        match self {
            SubCommand::Endpoint(opts) => opts.execute().await,
        }
    }
}

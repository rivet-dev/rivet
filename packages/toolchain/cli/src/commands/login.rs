use anyhow::*;
use clap::Parser;

/// Login to a project
#[derive(Parser)]
pub struct Opts {
	/// Specify a custom API endpoint (defaults to production API)
	#[clap(long)]
	api_endpoint: Option<String>,
}

impl Opts {
	pub async fn execute(&self) -> Result<()> {
		crate::util::login::login(self.api_endpoint.clone()).await?;

		Ok(())
	}
}

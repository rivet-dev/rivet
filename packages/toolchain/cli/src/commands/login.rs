use anyhow::*;
use clap::Parser;

/// Login to a project
#[derive(Parser)]
pub struct Opts {
	#[clap(long)]
	api_endpoint: Option<String>,
}

impl Opts {
	pub async fn execute(&self) -> Result<()> {
		let api_endpoint = if let Some(e) = &self.api_endpoint {
			Some(e.clone())
		} else {
			tokio::task::spawn_blocking(|| crate::util::login::inquire_self_hosting()).await??
		};

		crate::util::login::login(api_endpoint).await?;

		Ok(())
	}
}

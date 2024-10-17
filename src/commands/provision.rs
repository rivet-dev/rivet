use anyhow::*;
use clap::Parser;

#[derive(Parser)]
pub struct Opts {}

impl Opts {
	pub async fn execute(self) -> Result<()> {
		tracing::info!("provisioning s3");
		s3_util::provision().await?;

		tracing::info!("migrating database");
		rivet_migrate::up_all().await?;

		Ok(())
	}
}

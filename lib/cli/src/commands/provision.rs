use anyhow::*;
use clap::Parser;

use crate::run_config::RunConfig;

#[derive(Parser)]
pub struct Opts {}

impl Opts {
	pub async fn execute(self, run_config: &RunConfig) -> Result<()> {
		tracing::info!("provisioning s3");
		s3_util::provision(&run_config.s3_buckets).await?;

		tracing::info!("migrating database");
		rivet_migrate::up(&run_config.sql_services).await?;

		Ok(())
	}
}

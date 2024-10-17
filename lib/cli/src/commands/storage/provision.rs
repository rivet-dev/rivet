use anyhow::*;
use clap::Parser;

use crate::run_config::RunConfig;

#[derive(Parser)]
pub struct Opts {}

impl Opts {
	pub async fn execute(self, run_config: &RunConfig) -> Result<()> {
		s3_util::provision(&run_config.s3_buckets).await?;
		Ok(())
	}
}

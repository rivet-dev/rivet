use anyhow::*;
use clap::Parser;

#[derive(Parser)]
pub struct Opts {}

impl Opts {
	pub async fn execute(self) -> Result<()> {
		s3_util::provision().await?;
		Ok(())
	}
}

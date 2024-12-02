use anyhow::*;
use clap::Parser;

#[derive(Parser)]
pub struct Opts {}

impl Opts {
	pub async fn execute(&self) -> Result<()> {
		let ctx = toolchain::toolchain_ctx::load().await?;
		crate::util::env::select(&ctx, true).await?;
		Ok(())
	}
}

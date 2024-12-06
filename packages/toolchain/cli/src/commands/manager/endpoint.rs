use anyhow::*;
use clap::Parser;
use toolchain::tasks::manager::get_endpoint;

use crate::util::{
	task::{run_task, TaskOutputStyle},
};

#[derive(Parser)]
pub struct Opts {
	#[clap(long, alias = "env", short = 'e')]
	environment: Option<String>,
}

impl Opts {
	pub async fn execute(&self) -> Result<()> {
		let ctx = toolchain::toolchain_ctx::load().await?;
		let env = crate::util::env::get_or_select(&ctx, self.environment.as_ref()).await?;
		let res =
			run_task::<get_endpoint::Task>(TaskOutputStyle::None, get_endpoint::Input { env_slug: env })
			.await?;
		println!("{}", res.endpoint);
		Ok(())
	}
}

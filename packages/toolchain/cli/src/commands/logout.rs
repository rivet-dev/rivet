use anyhow::*;
use clap::Parser;
use toolchain::tasks;

use crate::util::task::{run_task, TaskOutputStyle};

/// Logout from a game
#[derive(Parser)]
pub struct Opts {}

impl Opts {
	pub async fn execute(&self) -> Result<()> {
		run_task::<tasks::auth::sign_out::Task>(
			TaskOutputStyle::None,
			tasks::auth::sign_out::Input {},
		)
		.await?;
		eprintln!("Logged out");
		Ok(())
	}
}

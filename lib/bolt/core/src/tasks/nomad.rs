use std::process::Command;

use anyhow::*;

use crate::{context::ProjectContext, utils::command_helper::CommandHelper};

pub async fn logs(_ctx: &ProjectContext, service_name: &str, region: &str) -> Result<()> {
	let mut cmd = Command::new("nomad");
	cmd.arg("alloc")
		.arg("logs")
		.arg("-f")
		.arg("-job")
		.arg(format!("rivet-{}:{}", service_name, region));

	cmd.exec().await
}

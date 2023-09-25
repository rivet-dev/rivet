use anyhow::*;
use std::process::Command;

use crate::{context::ProjectContext, utils::command_helper::CommandHelper};

pub struct LogsOpts {
	pub follow: bool,
	pub stream: LogStream,
}

pub enum LogStream {
	StdOut,
	StdErr,
}

pub async fn logs(ctx: &ProjectContext, service_name: &str, opts: &LogsOpts) -> Result<()> {
	let mut cmd = Command::new("kubectl");
	cmd.arg("logs")
		.arg("-n")
		.arg("rivet-service")
		.arg(format!("deployment/rivet-{service_name}"));
	if opts.follow {
		cmd.arg("-f");
	}
	cmd.env("KUBECONFIG", ctx.gen_kubeconfig_path());

	cmd.exec().await
}

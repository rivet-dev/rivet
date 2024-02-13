use std::process::Command;

use crate::utils::command_helper::CommandHelper;

async fn build_command() -> Command {
	Command::new("docker")
}

pub async fn push(tag: &str, quiet: bool) {
	let mut cmd = build_command().await;
	cmd.arg("push").arg(tag);
	cmd.exec_quiet(quiet, quiet).await.unwrap();
}

pub async fn tag(src_tag: &str, target_tag: &str) {
	let mut cmd = build_command().await;
	cmd.arg("tag").arg(src_tag).arg(target_tag);
	cmd.exec().await.unwrap();
}

/// Gets the SHA hash for an image.
///
/// Returns `None` if image does not exist.
pub async fn container_exists(tag: &str) -> bool {
	let mut cmd = build_command().await;
	cmd.arg("manifest").arg("inspect").arg(tag);

	tokio::task::spawn_blocking(move || cmd.output().unwrap().status.success())
		.await
		.expect("blocking")
}

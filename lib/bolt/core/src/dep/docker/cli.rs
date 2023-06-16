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
pub async fn inspect_sha_tag(tag: &str) -> Option<String> {
	let mut cmd = build_command().await;
	cmd.arg("inspect")
		.args(&["--format", r#"{{index .RepoDigests 0}}"#])
		.arg(tag);

	tokio::task::spawn_blocking(move || {
		let sha_cmd = cmd.output().unwrap();
		if sha_cmd.status.success() {
			Some(String::from_utf8(sha_cmd.stdout).unwrap().trim().to_owned())
		} else {
			None
		}
	})
	.await
	.expect("blocking")
}

pub async fn inspect_sha_tag_pull(tag: &str) -> Option<String> {
	// Optimistically inspect if the image is already there
	if let Some(tag) = inspect_sha_tag(tag).await {
		return Some(tag);
	}

	// If not, then pull the image
	let mut pull_cmd = build_command().await;
	pull_cmd.arg("pull").arg(tag);
	let pull_success = tokio::task::spawn_blocking(move || {
		let out = pull_cmd.output().unwrap();
		out.status.success()
	})
	.await
	.expect("blocking");
	if !pull_success {
		return None;
	}

	// Now resolve the tag
	inspect_sha_tag(tag).await
}

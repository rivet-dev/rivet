use std::{
	os::unix::process::CommandExt,
	path::PathBuf,
	process::{Command, Stdio},
};

use anyhow::*;
use nix::{
	sys::wait::{waitpid, WaitStatus},
	unistd::{fork, ForkResult},
};
use pegboard::protocol;
use url::Url;
use uuid::Uuid;

use crate::{ctx::Ctx, utils};

const VECTOR_SOCKET_ADDR: &str = "127.0.0.1:5021";

pub struct Container {
	pub container_id: Uuid,
	pub image_artifact_url: String,
	pub container_runner_binary_url: String,
	pub root_user_enabled: bool,
	pub stakeholder: protocol::Stakeholder,
}

impl Container {
	pub fn new(
		container_id: Uuid,
		image_artifact_url: String,
		container_runner_binary_url: String,
		stakeholder: protocol::Stakeholder,
	) -> Self {
		Container {
			container_id,
			image_artifact_url,
			container_runner_binary_url,
			root_user_enabled: todo!(),
			stakeholder,
		}
	}

	pub async fn start(self, ctx: &Ctx) -> Result<()> {
		// Get local path of container runner
		let container_runner_path = ctx
			.fetch_container_runner(&self.container_runner_binary_url)
			.await?;

		let url = Url::parse(&self.image_artifact_url)?;
		let path_stub = utils::get_s3_path_stub(&url, false)?;
		let path = ctx.container_path(self.container_id).join(path_stub);

		// Download container image
		utils::download_file(&self.image_artifact_url, &path).await?;

		// Spawn container
		self.spawn(&ctx, container_runner_path)
	}

	fn spawn(&self, ctx: &Ctx, container_runner_path: PathBuf) -> Result<()> {
		let container_path = ctx.container_path(self.container_id);

		let mut env = vec![
			(
				"PEGBOARD_META_root_user_enabled",
				self.root_user_enabled.to_string(),
			),
			(
				"PEGBOARD_META_vector_socket_addr",
				VECTOR_SOCKET_ADDR.to_string(),
			),
		];
		env.extend(self.stakeholder.env());

		spawn_orphaned_container_runner(container_runner_path, container_path, &env)?;

		Ok(())
	}
}

fn spawn_orphaned_container_runner(
	container_runner_path: PathBuf,
	container_path: PathBuf,
	env: &[(&str, String)],
) -> Result<()> {
	// Prepare the arguments for the container runner
	let runner_args = vec![container_path.to_str().unwrap()];

	// NOTE: This is why we fork the process twice: https://stackoverflow.com/a/5386753
	match unsafe { fork() }.context("process first fork failed")? {
		ForkResult::Parent { child } => {
			// Ensure that the child process spawned successfully
			match waitpid(child, None).context("waitpid failed")? {
				WaitStatus::Exited(_, 0) => Ok(()),
				WaitStatus::Exited(_, status) => {
					bail!("Child process exited with status {}", status)
				}
				_ => bail!("Unexpected wait status for child process"),
			}
		}
		ForkResult::Child => {
			// Child process
			match unsafe { fork() } {
				Result::Ok(ForkResult::Parent { .. }) => {
					// Exit the intermediate child
					std::process::exit(0);
				}
				Result::Ok(ForkResult::Child) => {
					// Exit immediately on fail in order to not leak process
					let err = Command::new(&container_runner_path)
						.args(&runner_args)
						.envs(env.iter().cloned())
						.stdin(Stdio::null())
						.stdout(Stdio::null())
						.stderr(Stdio::null())
						.exec();
					eprintln!("exec failed: {err:?}");
					std::process::exit(1);
				}
				Err(err) => {
					// Exit immediately in order to not leak child process.
					//
					// The first fork doesn't need to exit on error since it
					eprintln!("process second fork failed: {err:?}");
					std::process::exit(1);
				}
			}
		}
	}
}

use std::{
	os::unix::process::CommandExt,
	path::{Path, PathBuf},
	process::{Command, Stdio},
	sync::Arc,
	time::{Duration, Instant},
};

use anyhow::*;
use futures_util::{stream::FuturesUnordered, FutureExt, StreamExt};
use indoc::indoc;
use nix::{
	sys::{
		signal::{kill, Signal},
		wait::{waitpid, WaitStatus},
	},
	unistd::{fork, pipe, read, write, ForkResult, Pid},
};
use pegboard::protocol;
use tokio::{fs, sync::Mutex};
use url::Url;
use uuid::Uuid;

use crate::{ctx::Ctx, utils};

/// How often to check for a PID when one is not present and a stop command was received.
const STOP_PID_INTERVAL: Duration = std::time::Duration::from_millis(250);
/// How long to wait until no longer waiting for a PID when a stop command was received.
const STOP_PID_TIMEOUT: Duration = std::time::Duration::from_secs(30);
/// How often to check that a PID is still running when observing container state.
const PID_POLL_INTERVAL: Duration = std::time::Duration::from_millis(1000);
const VECTOR_SOCKET_ADDR: &str = "127.0.0.1:5021";

#[derive(Debug)]
enum ObservationState {
	Exited,
	Running,
	Dead,
}

pub struct Container {
	pub container_id: Uuid,
	pub image_artifact_url: String,
	pub container_runner_binary_url: String,
	pub root_user_enabled: bool,
	pub stakeholder: protocol::Stakeholder,

	pub pid: Mutex<Option<Pid>>,
}

impl Container {
	pub fn new(
		container_id: Uuid,
		image_artifact_url: String,
		container_runner_binary_url: String,
		root_user_enabled: bool,
		stakeholder: protocol::Stakeholder,
	) -> Arc<Self> {
		Arc::new(Container {
			container_id,
			image_artifact_url,
			container_runner_binary_url,
			root_user_enabled,
			stakeholder,

			pid: Mutex::new(None),
		})
	}

	pub async fn start(self: &Arc<Self>, ctx: &Arc<Ctx>) -> Result<()> {
		tracing::info!(container_id=?self.container_id, "starting container");

		// Write container to DB
		utils::query(|| async {
			// NOTE: On conflict here in case this query runs but the command is not acknowledged
			sqlx::query(indoc!(
				"
				INSERT INTO containers (
					container_id,
					create_ts
				)
				VALUES (?1, ?2)
				ON CONFLICT (container_id) DO NOTHING
				",
			))
			.bind(self.container_id)
			.bind(utils::now())
			.execute(&mut *ctx.sql().await?)
			.await
		})
		.await?;

		ctx.event(protocol::Event::ContainerStateUpdate {
			container_id: self.container_id,
			state: protocol::ContainerState::Starting,
		})
		.await?;

		let self2 = self.clone();
		let ctx2 = ctx.clone();
		tokio::spawn(async {
			if let Err(err) = self2.run(ctx2).await {
				tracing::error!(?err, "container run failed");
			}
		});

		Ok(())
	}

	async fn run(self: Arc<Self>, ctx: Arc<Ctx>) -> Result<()> {
		let container_path = ctx.container_path(self.container_id);

		fs::create_dir(&container_path).await?;

		// Get local path of container runner
		let container_runner_path = ctx
			.fetch_container_runner(&self.container_runner_binary_url)
			.await?;

		let url = Url::parse(&self.image_artifact_url)?;
		let path_stub = utils::get_s3_path_stub(&url, false)?;
		let path = container_path.join(path_stub);

		// Download container image
		utils::download_file(&self.image_artifact_url, &path).await?;

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

		tracing::info!(container_id=?self.container_id, "spawning");

		// Spawn runner which spawns the container
		let pid = spawn_orphaned_container_runner(container_runner_path, container_path, &env)?;

		tracing::info!(container_id=?self.container_id, ?pid, "pid received");

		{
			*self.pid.lock().await = Some(pid);
		}

		ctx.event(protocol::Event::ContainerStateUpdate {
			container_id: self.container_id,
			state: protocol::ContainerState::Running {
				pid: pid.as_raw().try_into()?,
			},
		})
		.await?;

		self.observe(ctx, pid).await?;

		Ok(())
	}

	// Watch container for updates
	async fn observe(&self, ctx: Arc<Ctx>, pid: Pid) -> Result<()> {
		let exit_code_path = ctx.container_path(self.container_id).join("exit-code");
		let proc_path = Path::new("/proc").join(pid.to_string());

		let mut futs = FuturesUnordered::new();

		// Watch for exit code file being written
		futs.push(
			async {
				utils::wait_for_creation(&exit_code_path).await?;

				Ok(ObservationState::Exited)
			}
			.boxed(),
		);

		// Polling interval to check that the pid still exists
		futs.push(
			async {
				tokio::time::sleep(PID_POLL_INTERVAL).await;

				if fs::metadata(&proc_path).await.is_ok() {
					Ok(ObservationState::Running)
				} else {
					Ok(ObservationState::Dead)
				}
			}
			.boxed(),
		);

		let state = loop {
			// Get next complete future
			if let Some(state) = futs.next().await {
				let state = state?;

				// If still running, add poll future back to list
				if let ObservationState::Running = state {
					futs.push(
						async {
							tokio::time::sleep(PID_POLL_INTERVAL).await;

							if fs::metadata(&proc_path).await.is_ok() {
								Ok(ObservationState::Running)
							} else {
								Ok(ObservationState::Dead)
							}
						}
						.boxed(),
					);
				} else {
					break state;
				}
			} else {
				bail!("observation failed, developer error");
			}
		};

		let exit_code = if let ObservationState::Exited = state {
			Some(
				fs::read_to_string(&exit_code_path)
					.await?
					.trim()
					.parse::<i32>()?,
			)
		} else {
			None
		};

		tracing::info!(container_id=?self.container_id, ?exit_code, "received exit code");

		utils::query(|| async {
			sqlx::query(indoc!(
				"
				UPDATE containers
				SET
					exit_ts = ?2 AND
					exit_code = ?3
				WHERE container_id = ?1
				",
			))
			.bind(self.container_id)
			.bind(utils::now())
			.bind(exit_code)
			.execute(&mut *ctx.sql().await?)
			.await
		})
		.await?;

		ctx.event(protocol::Event::ContainerStateUpdate {
			container_id: self.container_id,
			state: protocol::ContainerState::Exited { exit_code },
		})
		.await?;

		tracing::info!(container_id=?self.container_id, "container complete");

		Ok(())
	}

	pub async fn stop(self: &Arc<Self>, ctx: &Arc<Ctx>) -> Result<()> {
		let self2 = self.clone();
		let ctx2 = ctx.clone();
		tokio::spawn(async {
			if let Err(err) = self2.stop_inner(ctx2).await {
				tracing::error!(?err, "container stop failed");
			}
		});

		Ok(())
	}

	async fn stop_inner(self: Arc<Self>, ctx: Arc<Ctx>) -> Result<()> {
		let now = Instant::now();

		let pid = loop {
			if let Some(pid) = *self.pid.lock().await {
				break Some(pid);
			}

			tracing::warn!(container_id=?self.container_id, "waiting for pid to stop workflow");

			if now.elapsed() > STOP_PID_TIMEOUT {
				tracing::error!(
					container_id=?self.container_id,
					"timed out waiting for container to get PID, considering container stopped",
				);

				break None;
			}

			tokio::time::sleep(STOP_PID_INTERVAL).await;
		};

		// Kill if PID found
		if let Some(pid) = pid {
			kill(pid, Signal::SIGTERM)?;
		}

		utils::query(|| async {
			sqlx::query(indoc!(
				"
				UPDATE containers
				SET stop_ts = ?2
				container_id = ?1
				",
			))
			.bind(self.container_id)
			.bind(utils::now())
			.execute(&mut *ctx.sql().await?)
			.await
		})
		.await?;

		ctx.event(protocol::Event::ContainerStateUpdate {
			container_id: self.container_id,
			state: protocol::ContainerState::Stopping,
		})
		.await?;

		Ok(())
	}
}

fn spawn_orphaned_container_runner(
	container_runner_path: PathBuf,
	container_path: PathBuf,
	env: &[(&str, String)],
) -> Result<Pid> {
	// Prepare the arguments for the container runner
	let runner_args = vec![container_path.to_str().context("bad path")?];

	// Pipe communication between processes
	let (pipe_read, pipe_write) = pipe()?;

	// NOTE: This is why we fork the process twice: https://stackoverflow.com/a/5386753
	match unsafe { fork() }.context("process first fork failed")? {
		ForkResult::Parent { child } => {
			// Close the writing end of the pipe in the parent
			nix::unistd::close(pipe_write)?;

			// Ensure that the child process spawned successfully
			match waitpid(child, None).context("waitpid failed")? {
				WaitStatus::Exited(_, 0) => {
					// Read the second child's PID from the pipe
					let mut buf = [0u8; 4];
					read(pipe_read, &mut buf)?;
					let second_child_pid = Pid::from_raw(i32::from_le_bytes(buf));

					Ok(second_child_pid)
				}
				WaitStatus::Exited(_, status) => {
					bail!("Child process exited with status {}", status)
				}
				_ => bail!("Unexpected wait status for child process"),
			}
		}
		ForkResult::Child => {
			// Child process
			match unsafe { fork() } {
				Result::Ok(ForkResult::Parent { child }) => {
					// Write the second child's PID to the pipe
					let child_pid_bytes = child.as_raw().to_le_bytes();
					write(pipe_write, &child_pid_bytes)?;

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

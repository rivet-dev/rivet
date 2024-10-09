use std::{
	path::{Path, PathBuf},
	sync::Arc,
	time::{Duration, Instant},
};

use anyhow::*;
use futures_util::{stream::FuturesUnordered, FutureExt, StreamExt};
use indoc::indoc;
use nix::{
	sys::signal::{kill, Signal},
	unistd::Pid,
};
use pegboard::protocol;
use tokio::{fs, sync::Mutex};
use uuid::Uuid;

use crate::{ctx::Ctx, utils};

mod oci_config;
mod seccomp;
mod setup;

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
	container_id: Uuid,

	pid: Mutex<Option<Pid>>,
}

impl Container {
	pub fn new(container_id: Uuid) -> Arc<Self> {
		Arc::new(Container {
			container_id,

			pid: Mutex::new(None),
		})
	}

	pub async fn start(
		self: &Arc<Self>,
		ctx: &Arc<Ctx>,
		config: protocol::ContainerConfig,
	) -> Result<()> {
		tracing::info!(container_id=?self.container_id, "starting container");

		// Write container to DB
		utils::query(|| async {
			// NOTE: On conflict here in case this query runs but the command is not acknowledged
			sqlx::query(indoc!(
				"
				INSERT INTO containers (
					container_id,
					start_ts
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

		{
			let s = self.clone();
			let ctx = ctx.clone();

			tokio::spawn(async move {
				if let Err(err) = s.setup(&ctx, config).await {
					tracing::error!(?err, "container run failed");

					// Cleanup
					let mut containers = ctx.containers.write().await;
					containers.remove(&s.container_id);
				}
			});
		}

		Ok(())
	}

	async fn setup(
		self: &Arc<Self>,
		ctx: &Arc<Ctx>,
		config: protocol::ContainerConfig,
	) -> Result<()> {
		let container_path = ctx.container_path(self.container_id);

		fs::create_dir(&container_path).await?;

		// Download container runner
		let container_runner_path = ctx
			.fetch_container_runner(&config.container_runner_binary_url)
			.await?;

		setup::cni_bundle(self.container_id, &config, &ctx).await?;

		// Run CNI setup script
		if let protocol::NetworkMode::Bridge = config.network_mode {
			setup::cni_network(self.container_id, &config, &ctx).await?;
		}

		let mut runner_env = vec![
			(
				"PEGBOARD_META_root_user_enabled",
				config.root_user_enabled.to_string(),
			),
			(
				"PEGBOARD_META_vector_socket_addr",
				VECTOR_SOCKET_ADDR.to_string(),
			),
		];
		runner_env.extend(config.stakeholder.env());

		self.run(ctx, container_runner_path, &runner_env).await?;

		Ok(())
	}

	async fn run(
		self: &Arc<Self>,
		ctx: &Arc<Ctx>,
		container_runner_path: PathBuf,
		env: &[(&str, String)],
	) -> Result<()> {
		tracing::info!(container_id=?self.container_id, "spawning");

		// Spawn runner which spawns the container
		let pid = setup::spawn_orphaned_container_runner(
			container_runner_path,
			ctx.container_path(self.container_id),
			&env,
		)?;

		tracing::info!(container_id=?self.container_id, ?pid, "pid received");

		// Store PID
		{
			*self.pid.lock().await = Some(pid);
		}

		// Update DB
		utils::query(|| async {
			sqlx::query(indoc!(
				"
				UPDATE containers
				SET
					running_ts = ?2
				WHERE container_id = ?1
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
			state: protocol::ContainerState::Running {
				pid: pid.as_raw().try_into()?,
			},
		})
		.await?;

		self.observe(ctx, pid).await?;

		Ok(())
	}

	// Watch container for updates
	async fn observe(&self, ctx: &Arc<Ctx>, pid: Pid) -> Result<()> {
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

		// Update DB
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
		tokio::spawn(async move {
			if let Err(err) = self2.stop_inner(&ctx2).await {
				tracing::error!(?err, "container stop failed");
			}

			// Cleanup regardless
			let mut containers = ctx2.containers.write().await;
			containers.remove(&self2.container_id);
		});

		Ok(())
	}

	async fn stop_inner(self: &Arc<Self>, ctx: &Arc<Ctx>) -> Result<()> {
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

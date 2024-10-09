use std::{
	path::{Path, PathBuf},
	sync::{
		atomic::{AtomicBool, Ordering},
		Arc,
	},
	time::Duration,
};

use anyhow::*;
use futures_util::{stream::FuturesUnordered, FutureExt, StreamExt};
use indoc::indoc;
use nix::{
	errno::Errno,
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
/// How many times to check for a PID when a stop command was received.
const STOP_PID_RETRIES: usize = 32;
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
	config: protocol::ContainerConfig,

	pid: Mutex<Option<Pid>>,
	exited: AtomicBool,
}

impl Container {
	pub fn new(container_id: Uuid, config: protocol::ContainerConfig) -> Arc<Self> {
		Arc::new(Container {
			container_id,
			config,

			pid: Mutex::new(None),
			exited: AtomicBool::new(false),
		})
	}

	pub fn with_pid(container_id: Uuid, config: protocol::ContainerConfig, pid: Pid) -> Arc<Self> {
		Arc::new(Container {
			container_id,
			config,

			pid: Mutex::new(Some(pid)),
			exited: AtomicBool::new(false),
		})
	}

	pub async fn start(self: &Arc<Self>, ctx: &Arc<Ctx>) -> Result<()> {
		tracing::info!(container_id=?self.container_id, "starting");

		// Write container to DB
		let config_json = serde_json::to_vec(&self.config)?;
		utils::query(|| async {
			// NOTE: On conflict here in case this query runs but the command is not acknowledged
			sqlx::query(indoc!(
				"
				INSERT INTO containers (
					container_id,
					config,
					start_ts
				)
				VALUES (?1, ?2, ?3)
				ON CONFLICT (container_id) DO NOTHING
				",
			))
			.bind(self.container_id)
			.bind(&config_json)
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

		// Lifecycle
		let self2 = self.clone();
		let ctx2 = ctx.clone();
		tokio::spawn(async move {
			use std::result::Result::{Err, Ok};

			match self2.setup(&ctx2).await {
				Ok((container_runner_path, proxied_ports)) => {
					match self2.run(&ctx2, container_runner_path, proxied_ports).await {
						Ok(pid) => {
							if let Err(err) = self2.observe(&ctx2, pid).await {
								tracing::error!(container_id=?self2.container_id, ?err, "observe failed");
							}
						}
						Err(err) => {
							tracing::error!(container_id=?self2.container_id, ?err, "run failed")
						}
					}
				}
				Err(err) => tracing::error!(container_id=?self2.container_id, ?err, "setup failed"),
			}

			// Cleanup afterwards
			self2.cleanup(&ctx2).await
		});

		Ok(())
	}

	async fn setup(
		self: &Arc<Self>,
		ctx: &Arc<Ctx>,
	) -> Result<(
		PathBuf,
		protocol::HashableMap<String, protocol::ProxiedPort>,
	)> {
		tracing::info!(container_id=?self.container_id, "setting up");

		let container_path = ctx.container_path(self.container_id);

		// Create container working dir
		fs::create_dir(&container_path).await?;

		// Download container runner
		let container_runner_path = ctx
			.fetch_container_runner(&self.config.container_runner_binary_url)
			.await?;

		self.setup_oci_bundle(&ctx).await?;

		// Run CNI setup script
		let proxied_ports = if let protocol::NetworkMode::Bridge = self.config.network_mode {
			let proxied_ports = self.bind_ports(ctx).await?;
			self.setup_cni_network(&ctx, &proxied_ports).await?;

			proxied_ports
		} else {
			Default::default()
		};

		Ok((container_runner_path, proxied_ports))
	}

	async fn run(
		self: &Arc<Self>,
		ctx: &Arc<Ctx>,
		container_runner_path: PathBuf,
		proxied_ports: protocol::HashableMap<String, protocol::ProxiedPort>,
	) -> Result<Pid> {
		tracing::info!(container_id=?self.container_id, "spawning");

		let mut runner_env = vec![
			(
				"PEGBOARD_META_root_user_enabled",
				self.config.root_user_enabled.to_string(),
			),
			(
				"PEGBOARD_META_vector_socket_addr",
				VECTOR_SOCKET_ADDR.to_string(),
			),
		];
		runner_env.extend(self.config.stakeholder.env());

		// Spawn runner which spawns the container
		let pid = setup::spawn_orphaned_container_runner(
			container_runner_path,
			ctx.container_path(self.container_id),
			&runner_env,
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
					running_ts = ?2,
					pid = ?3
				WHERE container_id = ?1
				",
			))
			.bind(self.container_id)
			.bind(utils::now())
			.bind(pid.as_raw())
			.execute(&mut *ctx.sql().await?)
			.await
		})
		.await?;

		ctx.event(protocol::Event::ContainerStateUpdate {
			container_id: self.container_id,
			state: protocol::ContainerState::Running {
				pid: pid.as_raw().try_into()?,
				proxied_ports,
			},
		})
		.await?;

		Ok(pid)
	}

	// Watch container for updates
	pub(crate) async fn observe(&self, ctx: &Arc<Ctx>, pid: Pid) -> Result<()> {
		tracing::info!(container_id=?self.container_id, ?pid, "observing");

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
			use std::result::Result::Ok;
			match fs::read_to_string(&exit_code_path).await {
				Ok(contents) => match contents.trim().parse::<i32>() {
					Ok(x) => Some(x),
					Err(err) => {
						tracing::error!(?err, "failed to parse exit code file");

						None
					}
				},
				Err(err) => {
					tracing::error!(?err, "failed to read exit code file");

					None
				}
			}
		} else {
			tracing::warn!(?pid, "process died before exit code file was written");

			None
		};

		tracing::info!(container_id=?self.container_id, ?exit_code, "exited");

		self.set_exit_code(ctx, exit_code).await?;

		tracing::info!(container_id=?self.container_id, "complete");

		Ok(())
	}

	pub async fn signal(self: &Arc<Self>, ctx: &Arc<Ctx>, signal: Signal) -> Result<()> {
		tracing::info!(container_id=?self.container_id, ?signal, "sending signal");

		let self2 = self.clone();
		let ctx2 = ctx.clone();
		tokio::spawn(async move {
			if let Err(err) = self2.signal_inner(&ctx2, signal).await {
				tracing::error!(?err, "container signal failed");
			}
		});

		Ok(())
	}

	async fn signal_inner(self: &Arc<Self>, ctx: &Arc<Ctx>, signal: Signal) -> Result<()> {
		let mut i = 0;

		// Signal command might be sent before the container has a valid PID. This loop waits for the PID to
		// be set
		let pid = loop {
			if let Some(pid) = *self.pid.lock().await {
				break Some(pid);
			}

			tracing::warn!(container_id=?self.container_id, "waiting for pid to signal container");

			if i > STOP_PID_RETRIES {
				tracing::error!(
					container_id=?self.container_id,
					"timed out waiting for container to get PID, considering container stopped",
				);

				break None;
			}
			i += 1;

			tokio::time::sleep(STOP_PID_INTERVAL).await;
		};

		// Kill if PID set
		if let Some(pid) = pid {
			use std::result::Result::{Err, Ok};

			match kill(pid, signal) {
				Ok(_) => {}
				Err(Errno::ESRCH) => {
					tracing::warn!(container_id=?self.container_id, ?pid, "pid not found for signalling")
				}
				Err(err) => return Err(err.into()),
			}
		}

		// Update stop_ts
		if matches!(signal, Signal::SIGTERM | Signal::SIGKILL) || pid.is_none() {
			let stop_ts_set = utils::query(|| async {
				sqlx::query_as::<_, (bool,)>(indoc!(
					"
					UPDATE containers
					SET stop_ts = ?2
					WHERE
						container_id = ?1 AND
						stop_ts IS NULL
					RETURNING 1
					",
				))
				.bind(self.container_id)
				.bind(utils::now())
				.fetch_optional(&mut *ctx.sql().await?)
				.await
			})
			.await?
			.is_some();

			// Emit event if not stopped before
			if stop_ts_set {
				ctx.event(protocol::Event::ContainerStateUpdate {
					container_id: self.container_id,
					state: protocol::ContainerState::Stopped,
				})
				.await?;
			}
		}

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	pub async fn set_exit_code(&self, ctx: &Ctx, exit_code: Option<i32>) -> Result<()> {
		// Update DB
		utils::query(|| async {
			sqlx::query(indoc!(
				"
				UPDATE containers
				SET
					exit_ts = ?2,
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

		// Unbind ports
		utils::query(|| async {
			sqlx::query(indoc!(
				"
				UPDATE container_ports
				SET delete_ts = ?2
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
			state: protocol::ContainerState::Exited { exit_code },
		})
		.await?;

		self.exited.store(true, Ordering::SeqCst);

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	pub async fn cleanup(&self, ctx: &Ctx) -> Result<()> {
		tracing::info!(container_id=?self.container_id, "cleaning up");

		{
			// Cleanup ctx
			let mut containers = ctx.containers.write().await;
			containers.remove(&self.container_id);
		}

		if !self.exited.load(Ordering::SeqCst) {
			self.set_exit_code(ctx, None).await?;
		}

		self.cleanup_setup(ctx).await
	}
}

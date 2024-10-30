use std::{sync::Arc, time::Duration};

use anyhow::*;
use indoc::indoc;
use nix::{
	errno::Errno,
	sys::signal::{kill, Signal},
};
use pegboard::protocol;
use tokio::{fs, sync::Mutex};
use uuid::Uuid;

use crate::{ctx::Ctx, runner, utils};

mod oci_config;
mod seccomp;
mod setup;

/// How often to check for a PID when one is not present and a stop command was received.
const STOP_PID_INTERVAL: Duration = std::time::Duration::from_millis(250);
/// How many times to check for a PID when a stop command was received.
const STOP_PID_RETRIES: usize = 32;

pub struct Actor {
	actor_id: Uuid,
	config: protocol::ActorConfig,

	runner: Mutex<Option<runner::Handle>>,
	exited: Mutex<bool>,
}

impl Actor {
	pub fn new(actor_id: Uuid, config: protocol::ActorConfig) -> Arc<Self> {
		Arc::new(Actor {
			actor_id,
			config,

			runner: Mutex::new(None),
			exited: Mutex::new(false),
		})
	}

	pub fn with_runner(
		actor_id: Uuid,
		config: protocol::ActorConfig,
		runner: runner::Handle,
	) -> Arc<Self> {
		Arc::new(Actor {
			actor_id,
			config,

			runner: Mutex::new(Some(runner)),
			exited: Mutex::new(false),
		})
	}

	pub async fn start(self: &Arc<Self>, ctx: &Arc<Ctx>) -> Result<()> {
		tracing::info!(actor_id=?self.actor_id, "starting");

		// Write actor to DB
		let config_json = serde_json::to_vec(&self.config)?;

		utils::query(|| async {
			// NOTE: On conflict here in case this query runs but the command is not acknowledged
			sqlx::query(indoc!(
				"
				INSERT INTO actors (
					actor_id,
					config,
					start_ts
				)
				VALUES (?1, ?2, ?3)
				ON CONFLICT (actor_id) DO NOTHING
				",
			))
			.bind(self.actor_id)
			.bind(&config_json)
			.bind(utils::now())
			.execute(&mut *ctx.sql().await?)
			.await
		})
		.await?;

		ctx.event(protocol::Event::ActorStateUpdate {
			actor_id: self.actor_id,
			state: protocol::ActorState::Starting,
		})
		.await?;

		// Lifecycle
		let self2 = self.clone();
		let ctx2 = ctx.clone();
		tokio::spawn(async move {
			use std::result::Result::{Err, Ok};

			match self2.setup(&ctx2).await {
				Ok(proxied_ports) => match self2.run(&ctx2, proxied_ports).await {
					Ok(_) => {
						if let Err(err) = self2.observe(&ctx2).await {
							tracing::error!(actor_id=?self2.actor_id, ?err, "observe failed");
						}
					}
					Err(err) => {
						tracing::error!(actor_id=?self2.actor_id, ?err, "run failed")
					}
				},
				Err(err) => tracing::error!(actor_id=?self2.actor_id, ?err, "setup failed"),
			}

			// Cleanup afterwards
			self2.cleanup(&ctx2).await
		});

		Ok(())
	}

	async fn setup(
		self: &Arc<Self>,
		ctx: &Arc<Ctx>,
	) -> Result<protocol::HashableMap<String, protocol::ProxiedPort>> {
		tracing::info!(actor_id=?self.actor_id, "setting up");

		let actor_path = ctx.actor_path(self.actor_id);

		// Create actor working dir
		fs::create_dir(&actor_path).await?;

		// Download artifact
		self.download_image(&ctx).await?;

		let ports = self.bind_ports(ctx).await?;

		match self.config.image.kind {
			protocol::ImageKind::DockerImage | protocol::ImageKind::OciBundle => {
				self.setup_oci_bundle(&ctx, &ports).await?;

				// Run CNI setup script
				if let protocol::NetworkMode::Bridge = self.config.network_mode {
					self.setup_cni_network(&ctx, &ports).await?;
				}
			}
			protocol::ImageKind::JavaScript => self.setup_isolate(&ctx, &ports).await?,
		}

		Ok(ports)
	}

	async fn run(
		self: &Arc<Self>,
		ctx: &Arc<Ctx>,
		ports: protocol::HashableMap<String, protocol::ProxiedPort>,
	) -> Result<()> {
		tracing::info!(actor_id=?self.actor_id, "spawning");

		let mut runner_env = vec![
			(
				"ROOT_USER_ENABLED",
				self.config.root_user_enabled.to_string(),
			),
			(
				"VECTOR_SOCKET_ADDR",
				ctx.config().vector_socket_addr.to_string(),
			),
		];
		runner_env.extend(self.config.stakeholder.env());

		let runner = match self.config.image.kind {
			// Spawn runner which spawns the container
			protocol::ImageKind::DockerImage | protocol::ImageKind::OciBundle => {
				runner::Handle::spawn_orphaned(
					runner::Comms::Basic,
					&ctx.config().container_runner_binary_path,
					ctx.actor_path(self.actor_id),
					&runner_env,
				)?
			}
			// Shared runner
			protocol::ImageKind::JavaScript => {
				let runner = ctx.get_or_spawn_isolate_runner().await?;

				runner
					.send(&runner_protocol::ToRunner::Start {
						actor_id: self.actor_id,
					})
					.await?;

				runner
			}
		};
		let pid = runner.pid().clone();

		tracing::info!(actor_id=?self.actor_id, ?pid, "pid received");

		// Store runner
		{
			*self.runner.lock().await = Some(runner);
		}

		// Update DB
		utils::query(|| async {
			sqlx::query(indoc!(
				"
				UPDATE actors
				SET
					running_ts = ?2,
					pid = ?3
				WHERE actor_id = ?1
				",
			))
			.bind(self.actor_id)
			.bind(utils::now())
			.bind(pid.as_raw())
			.execute(&mut *ctx.sql().await?)
			.await
		})
		.await?;

		ctx.event(protocol::Event::ActorStateUpdate {
			actor_id: self.actor_id,
			state: protocol::ActorState::Running {
				pid: pid.as_raw().try_into()?,
				ports,
			},
		})
		.await?;

		Ok(())
	}

	// Watch actor for updates
	pub(crate) async fn observe(&self, ctx: &Arc<Ctx>) -> Result<()> {
		tracing::info!(actor_id=?self.actor_id, "observing");

		let Some(runner) = ({ (*self.runner.lock().await).clone() }) else {
			bail!("actor does not have a runner to observe yet");
		};

		let exit_code = match self.config.image.kind {
			protocol::ImageKind::DockerImage | protocol::ImageKind::OciBundle => {
				runner.observe().await?
			}
			// With isolates we have to check if the shared isolate runner exited and if the isolate itself
			// exited
			protocol::ImageKind::JavaScript => {
				let actor_path = ctx.actor_path(self.actor_id);
				let exit_code_path = actor_path.join("exit-code");

				tokio::select! {
					res = runner.observe() => res?,
					res = utils::wait_for_write(&exit_code_path) => {
						res?;

						use std::result::Result::{Err, Ok};
						let exit_code = match fs::read_to_string(&exit_code_path).await {
							Ok(contents) => match contents.trim().parse::<i32>() {
								Ok(x) => Some(x),
								Err(err) => {
									tracing::error!(actor_id=?self.actor_id, ?err, "failed to parse exit code file");

									None
								}
							},
							Err(err) => {
								tracing::error!(actor_id=?self.actor_id, ?err, "failed to read exit code file");

								None
							}
						};

						exit_code
					},
				}
			}
		};

		self.set_exit_code(ctx, exit_code).await?;

		tracing::info!(actor_id=?self.actor_id, "complete");

		Ok(())
	}

	pub async fn signal(self: &Arc<Self>, ctx: &Arc<Ctx>, signal: Signal) -> Result<()> {
		tracing::info!(actor_id=?self.actor_id, ?signal, "sending signal");

		let self2 = self.clone();
		let ctx2 = ctx.clone();
		tokio::spawn(async move {
			if let Err(err) = self2.signal_inner(&ctx2, signal).await {
				tracing::error!(?err, "actor signal failed");
			}
		});

		Ok(())
	}

	async fn signal_inner(self: &Arc<Self>, ctx: &Arc<Ctx>, signal: Signal) -> Result<()> {
		let mut i = 0;

		// Signal command might be sent before the actor has a runner. This loop waits for the runner to start
		let runner_guard = loop {
			let runner_guard = self.runner.lock().await;
			if runner_guard.is_some() {
				break Some(runner_guard);
			}

			tracing::warn!(actor_id=?self.actor_id, "waiting for pid to signal actor");

			if i > STOP_PID_RETRIES {
				tracing::error!(
					actor_id=?self.actor_id,
					"timed out waiting for actor to get PID, considering actor stopped",
				);

				break None;
			}
			i += 1;

			tokio::time::sleep(STOP_PID_INTERVAL).await;
		};

		let has_runner = runner_guard.is_some();

		// Kill if PID set
		if let Some(runner) = runner_guard {
			let runner = &*runner.as_ref().expect("must exist");

			// Send message
			if runner.has_socket() {
				runner
					.send(&runner_protocol::ToRunner::Signal {
						actor_id: self.actor_id,
						signal: signal as i32,
					})
					.await?;
			}
			// Send signal
			else {
				use std::result::Result::{Err, Ok};

				let pid = runner.pid().clone();

				match kill(pid, signal) {
					Ok(_) => {}
					Err(Errno::ESRCH) => {
						tracing::warn!(actor_id=?self.actor_id, ?pid, "pid not found for signalling")
					}
					Err(err) => return Err(err.into()),
				}
			}
		}

		// Update stop_ts
		if matches!(signal, Signal::SIGTERM | Signal::SIGKILL) || !has_runner {
			let stop_ts_set = utils::query(|| async {
				sqlx::query_as::<_, (bool,)>(indoc!(
					"
					UPDATE actors
					SET stop_ts = ?2
					WHERE
						actor_id = ?1 AND
						stop_ts IS NULL
					RETURNING 1
					",
				))
				.bind(self.actor_id)
				.bind(utils::now())
				.fetch_optional(&mut *ctx.sql().await?)
				.await
			})
			.await?
			.is_some();

			// Emit event if not stopped before
			if stop_ts_set {
				ctx.event(protocol::Event::ActorStateUpdate {
					actor_id: self.actor_id,
					state: protocol::ActorState::Stopped,
				})
				.await?;
			}
		}

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	pub async fn set_exit_code(&self, ctx: &Ctx, exit_code: Option<i32>) -> Result<()> {
		let mut guard = self.exited.lock().await;

		// Already exited
		if *guard {
			return Ok(());
		}

		// Update DB
		utils::query(|| async {
			sqlx::query(indoc!(
				"
				UPDATE actors
				SET
					exit_ts = ?2,
					exit_code = ?3
				WHERE actor_id = ?1
				",
			))
			.bind(self.actor_id)
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
				UPDATE actor_ports
				SET delete_ts = ?2
				WHERE actor_id = ?1
				",
			))
			.bind(self.actor_id)
			.bind(utils::now())
			.execute(&mut *ctx.sql().await?)
			.await
		})
		.await?;

		ctx.event(protocol::Event::ActorStateUpdate {
			actor_id: self.actor_id,
			state: protocol::ActorState::Exited { exit_code },
		})
		.await?;

		*guard = true;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	pub async fn cleanup(&self, ctx: &Ctx) -> Result<()> {
		tracing::info!(actor_id=?self.actor_id, "cleaning up");

		{
			// Cleanup ctx
			let mut actors = ctx.actors.write().await;
			actors.remove(&self.actor_id);
		}

		// Set exit code if it hasn't already been set
		self.set_exit_code(ctx, None).await?;

		self.cleanup_setup(ctx).await
	}
}

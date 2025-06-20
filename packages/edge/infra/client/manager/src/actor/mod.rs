use std::{
	result::Result::{Err, Ok},
	sync::Arc,
	time::{Duration, Instant},
};

use anyhow::*;
use indoc::indoc;
use nix::sys::signal::Signal;
use pegboard::protocol;
use pegboard_config::runner_protocol;
use sqlx::Acquire;
use tokio::{fs, sync::Mutex};
use uuid::Uuid;

use crate::{ctx::Ctx, runner, utils};

mod oci_config;
mod partial_oci_config;
mod seccomp;
mod setup;

/// How often to check for a PID when one is not present and a stop command was received.
const STOP_PID_INTERVAL: Duration = std::time::Duration::from_millis(250);
/// How many times to check for a PID when a stop command was received.
const STOP_PID_RETRIES: usize = 1024;

pub struct Actor {
	actor_id: Uuid,
	generation: u32,
	config: protocol::ActorConfig,

	runner: Mutex<Option<runner::Handle>>,
	exited: Mutex<bool>,
}

impl Actor {
	pub fn new(actor_id: Uuid, generation: u32, config: protocol::ActorConfig) -> Arc<Self> {
		Arc::new(Actor {
			actor_id,
			generation,
			config,

			runner: Mutex::new(None),
			exited: Mutex::new(false),
		})
	}

	pub fn with_runner(
		actor_id: Uuid,
		generation: u32,
		config: protocol::ActorConfig,
		runner: runner::Handle,
	) -> Arc<Self> {
		Arc::new(Actor {
			actor_id,
			generation,
			config,

			runner: Mutex::new(Some(runner)),
			exited: Mutex::new(false),
		})
	}

	pub async fn start(self: &Arc<Self>, ctx: &Arc<Ctx>) -> Result<()> {
		tracing::info!(actor_id=?self.actor_id, generation=?self.generation, "starting");

		// Write actor to DB
		let config_json = serde_json::to_vec(&self.config)?;

		utils::sql::query(|| async {
			// NOTE: On conflict here in case this query runs but the command is not acknowledged
			sqlx::query(indoc!(
				"
				INSERT INTO actors (
					actor_id,
					generation,
					config,
					start_ts,
					image_id
				)
				VALUES (?1, ?2, ?3, ?4, ?5)
				ON CONFLICT (actor_id, generation) DO NOTHING
				",
			))
			.bind(self.actor_id)
			.bind(self.generation as i64)
			.bind(&config_json)
			.bind(utils::now())
			.bind(self.config.image.id)
			.execute(&mut *ctx.sql().await?)
			.await
		})
		.await?;

		ctx.event(protocol::Event::ActorStateUpdate {
			actor_id: self.actor_id,
			generation: self.generation,
			state: protocol::ActorState::Starting,
		})
		.await?;

		// Lifecycle
		let self2 = self.clone();
		let ctx2 = ctx.clone();
		tokio::spawn(async move {
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
			if let Err(err) = self2.cleanup(&ctx2).await {
				tracing::error!(actor_id=?self2.actor_id, ?err, "cleanup failed");
			}
		});

		Ok(())
	}

	async fn setup(
		self: &Arc<Self>,
		ctx: &Arc<Ctx>,
	) -> Result<protocol::HashableMap<String, protocol::ProxiedPort>> {
		let setup_timer = Instant::now();
		tracing::info!(actor_id=?self.actor_id, generation=?self.generation, "setting up actor");

		let actor_path = ctx.actor_path(self.actor_id, self.generation);

		// Create actor working dir
		tracing::info!(actor_id=?self.actor_id, generation=?self.generation, "creating actor working directory");
		fs::create_dir(&actor_path)
			.await
			.context("failed to create actor dir")?;

		// Determine ahead of time if we need to set up CNI network
		let needs_cni_network =
			matches!(
				self.config.image.kind,
				protocol::ImageKind::DockerImage | protocol::ImageKind::OciBundle
			) && matches!(self.config.network_mode, protocol::NetworkMode::Bridge);

		tracing::info!(actor_id=?self.actor_id, generation=?self.generation, "starting parallel setup tasks");
		let parallel_timer = Instant::now();

		let (_, ports) = tokio::try_join!(
			async {
				self.download_image(&ctx).await?;
				self.make_fs(&ctx).await
			},
			async {
				let ports = self.bind_ports(ctx).await?;
				if needs_cni_network {
					self.setup_cni_network(&ctx, &ports).await?;
				}

				Ok(ports)
			}
		)?;

		let parallel_duration = parallel_timer.elapsed().as_secs_f64();
		crate::metrics::SETUP_PARALLEL_TASKS_DURATION.observe(parallel_duration);
		tracing::info!(
			actor_id=?self.actor_id,
			generation=?self.generation,
			duration_seconds=parallel_duration,
			"parallel setup tasks completed"
		);

		tracing::info!(actor_id=?self.actor_id, generation=?self.generation, "setting up runtime environment");
		match self.config.image.kind {
			protocol::ImageKind::DockerImage | protocol::ImageKind::OciBundle => {
				self.setup_oci_bundle(&ctx, &ports).await?;
			}
			protocol::ImageKind::JavaScript => self.setup_isolate(&ctx, &ports).await?,
		}

		let duration = setup_timer.elapsed().as_secs_f64();
		crate::metrics::SETUP_TOTAL_DURATION.observe(duration);
		tracing::info!(actor_id=?self.actor_id, generation=?self.generation, duration_seconds=duration, "actor setup completed");

		Ok(ports)
	}

	async fn run(
		self: &Arc<Self>,
		ctx: &Arc<Ctx>,
		ports: protocol::HashableMap<String, protocol::ProxiedPort>,
	) -> Result<()> {
		tracing::info!(actor_id=?self.actor_id, generation=?self.generation, "spawning");

		let mut runner_env = vec![
			(
				"ROOT_USER_ENABLED",
				if self.config.root_user_enabled {
					"1"
				} else {
					"0"
				}
				.to_string(),
			),
			("ACTOR_ID", self.actor_id.to_string()),
		];
		if let Some(vector) = &ctx.config().vector {
			runner_env.push(("VECTOR_SOCKET_ADDR", vector.address.to_string()));
		}

		let runner = match self.config.image.kind {
			// Spawn runner which spawns the container
			protocol::ImageKind::DockerImage | protocol::ImageKind::OciBundle => {
				runner::Handle::spawn_orphaned(
					runner::Comms::Basic,
					&ctx.config().runner.container_runner_binary_path(),
					ctx.actor_path(self.actor_id, self.generation),
					&runner_env,
				)?
			}
			// Shared runner
			protocol::ImageKind::JavaScript => {
				let runner = ctx.get_or_spawn_isolate_runner().await?;

				runner
					.send(&runner_protocol::ToRunner::Start {
						actor_id: self.actor_id,
						generation: self.generation,
					})
					.await?;

				runner
			}
		};
		let pid = runner.pid().clone();

		tracing::info!(actor_id=?self.actor_id, generation=?self.generation, ?pid, "pid received");

		// Store runner
		{
			*self.runner.lock().await = Some(runner);
		}

		// Update DB
		utils::sql::query(|| async {
			sqlx::query(indoc!(
				"
				UPDATE actors
				SET
					running_ts = ?3,
					pid = ?4
				WHERE
					actor_id = ?1 AND
					generation = ?2
				",
			))
			.bind(self.actor_id)
			.bind(self.generation as i64)
			.bind(utils::now())
			.bind(pid.as_raw())
			.execute(&mut *ctx.sql().await?)
			.await
		})
		.await?;

		ctx.event(protocol::Event::ActorStateUpdate {
			actor_id: self.actor_id,
			generation: self.generation,
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
		tracing::info!(actor_id=?self.actor_id, generation=?self.generation, "observing");

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
				let actor_path = ctx.actor_path(self.actor_id, self.generation);
				let exit_code_path = actor_path.join("exit-code");

				tokio::select! {
					res = runner.observe() => res?,
					res = utils::wait_for_write(&exit_code_path) => {
						res?;

						let exit_code = match fs::read_to_string(&exit_code_path).await {
							Ok(contents) => if contents.trim().is_empty() {
								// File exists but is empty. This is explicit
								None
							} else {
								match contents.trim().parse::<i32>() {
									Ok(x) => Some(x),
									Err(err) => {
										tracing::error!(
											actor_id=?self.actor_id,
											generation=?self.generation,
											?err,
											"failed to parse exit code file",
										);

										None
									}
								}
							},
							Err(err) => {
								tracing::error!(
									actor_id=?self.actor_id,
									generation=?self.generation,
									?err,
									"failed to read exit code file",
								);

								None
							}
						};

						exit_code
					},
				}
			}
		};

		self.set_exit_code(ctx, exit_code).await?;

		tracing::info!(actor_id=?self.actor_id, generation=?self.generation, "complete");

		Ok(())
	}

	pub async fn signal(
		self: &Arc<Self>,
		ctx: &Arc<Ctx>,
		signal: Signal,
		persist_storage: bool,
	) -> Result<()> {
		tracing::info!(actor_id=?self.actor_id, generation=?self.generation, ?signal, "sending signal");

		let self2 = self.clone();
		let ctx2 = ctx.clone();
		tokio::spawn(async move {
			if let Err(err) = self2.signal_inner(&ctx2, signal, persist_storage).await {
				tracing::error!(?err, "actor signal failed");
			}
		});

		Ok(())
	}

	async fn signal_inner(
		self: &Arc<Self>,
		ctx: &Arc<Ctx>,
		signal: Signal,
		persist_storage: bool,
	) -> Result<()> {
		let mut i = 0;

		// Signal command might be sent before the actor has a runner. This loop waits for the runner to start
		let runner_guard = loop {
			{
				let runner_guard = self.runner.lock().await;
				if runner_guard.is_some() {
					break Some(runner_guard);
				}
			}

			if *self.exited.lock().await {
				tracing::warn!(
					actor_id=?self.actor_id,
					generation=?self.generation,
					"actor exited before PID was set, ignoring signal",
				);

				break None;
			}

			// Progress log
			if i % 10 == 0 {
				tracing::warn!(
					actor_id=?self.actor_id,
					generation=?self.generation,
					"waiting for PID to signal actor",
				);
			}

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

		// Kill if runner exists
		if let Some(runner) = runner_guard {
			let runner = &*runner.as_ref().expect("must exist");

			// Send message
			if runner.has_socket() {
				runner
					.send(&runner_protocol::ToRunner::Signal {
						actor_id: self.actor_id,
						generation: self.generation,
						signal: signal as i32,
						persist_storage,
					})
					.await?;
			}
			// Send signal
			else {
				runner.signal(signal)?;
			}
		}

		// Update stop_ts
		if matches!(signal, Signal::SIGTERM | Signal::SIGKILL) || !has_runner {
			let stop_ts_set = utils::sql::query(|| async {
				let mut conn = ctx.sql().await?;
				let mut tx = conn.begin().await?;

				let res = sqlx::query_as::<_, (bool,)>(indoc!(
					"
					UPDATE actors
					SET stop_ts = ?3
					WHERE
						actor_id = ?1 AND
						generation = ?2 AND
						stop_ts IS NULL
					RETURNING 1
					",
				))
				.bind(self.actor_id)
				.bind(self.generation as i64)
				.bind(utils::now())
				.fetch_optional(&mut *tx)
				.await?;

				// Update LRU cache
				sqlx::query(indoc!(
					"
					UPDATE images_cache
					SET last_used_ts = ?2
					WHERE image_id = ?1
					",
				))
				.bind(self.config.image.id)
				.bind(utils::now())
				.execute(&mut *tx)
				.await?;

				tx.commit().await?;

				Ok(res.is_some())
			})
			.await?;

			// Emit event if not stopped before
			if stop_ts_set {
				ctx.event(protocol::Event::ActorStateUpdate {
					actor_id: self.actor_id,
					generation: self.generation,
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
		utils::sql::query(|| async {
			sqlx::query(indoc!(
				"
				UPDATE actors
				SET
					exit_ts = ?3,
					exit_code = ?4
				WHERE
					actor_id = ?1 AND
					generation = ?2
				",
			))
			.bind(self.actor_id)
			.bind(self.generation as i64)
			.bind(utils::now())
			.bind(exit_code)
			.execute(&mut *ctx.sql().await?)
			.await
		})
		.await?;

		// Unbind ports
		utils::sql::query(|| async {
			sqlx::query(indoc!(
				"
				UPDATE actor_ports
				SET delete_ts = ?3
				WHERE
					actor_id = ?1 AND
					generation = ?2
				",
			))
			.bind(self.actor_id)
			.bind(self.generation as i64)
			.bind(utils::now())
			.execute(&mut *ctx.sql().await?)
			.await
		})
		.await?;

		ctx.event(protocol::Event::ActorStateUpdate {
			actor_id: self.actor_id,
			generation: self.generation,
			state: protocol::ActorState::Exited { exit_code },
		})
		.await?;

		*guard = true;

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	pub async fn cleanup(&self, ctx: &Ctx) -> Result<()> {
		tracing::info!(actor_id=?self.actor_id, generation=?self.generation, "cleaning up");

		// Set exit code if it hasn't already been set
		self.set_exit_code(ctx, None).await?;

		// Cleanup setup. Should only be called after the exit code is set successfully for consistent state
		self.cleanup_setup(ctx).await;

		// It is important that we remove from the actors list last so that we prevent duplicates
		{
			let mut actors = ctx.actors.write().await;
			actors.remove(&(self.actor_id, self.generation));
		}

		Ok(())
	}
}

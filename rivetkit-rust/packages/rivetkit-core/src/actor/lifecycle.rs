use std::error::Error as StdError;
use std::fmt;
use std::any::Any;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use anyhow::{Context, Result};
use futures::future::BoxFuture;
use futures::FutureExt;
use tokio::runtime::Handle;
use tokio::time::{Instant, timeout};

use crate::actor::action::ActionInvoker;
use crate::actor::callbacks::{
	ActorInstanceCallbacks, OnDestroyRequest, OnMigrateRequest,
	OnSleepRequest, OnStateChangeRequest, OnWakeRequest, RunRequest,
};
use crate::actor::context::ActorContext;
use crate::actor::factory::{ActorFactory, FactoryRequest};
use crate::actor::state::{
	OnStateChangeCallback, PERSIST_DATA_KEY, PersistedActor, decode_persisted_actor,
};
use crate::types::SaveStateOpts;

pub type BeforeActorStartFn =
	dyn Fn(BeforeActorStartRequest) -> BoxFuture<'static, Result<()>> + Send + Sync;

#[derive(Clone, Debug)]
pub struct BeforeActorStartRequest {
	pub ctx: ActorContext,
	pub callbacks: Arc<ActorInstanceCallbacks>,
	pub is_new: bool,
}

#[derive(Clone, Default)]
pub struct ActorLifecycleDriverHooks {
	pub on_before_actor_start: Option<Arc<BeforeActorStartFn>>,
}

#[derive(Clone, Debug, Default)]
pub struct StartupOptions {
	pub preload_persisted_actor: Option<PersistedActor>,
	pub input: Option<Vec<u8>>,
	pub driver_hooks: ActorLifecycleDriverHooks,
}

#[derive(Clone, Debug)]
pub struct StartupOutcome {
	pub callbacks: Arc<ActorInstanceCallbacks>,
	pub is_new: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartupStage {
	LoadPersisted,
	Create,
	PersistInitialization,
	Migrate,
	Wake,
	RestoreConnections,
	BeforeActorStart,
}

#[derive(Debug)]
pub struct StartupError {
	stage: StartupStage,
	source: anyhow::Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShutdownStatus {
	Ok,
	Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShutdownOutcome {
	pub status: ShutdownStatus,
}

#[derive(Debug, Default)]
pub struct ActorLifecycle;

impl ActorLifecycle {
	pub async fn startup(
		&self,
		ctx: ActorContext,
		factory: &ActorFactory,
		options: StartupOptions,
	) -> std::result::Result<StartupOutcome, StartupError> {
		let persisted = self.load_persisted_actor(&ctx, &options).await?;
		let is_new = !persisted.has_initialized;
		ctx.load_persisted_actor(persisted);

		let callbacks = Arc::new(
			factory
				.create(FactoryRequest {
					ctx: ctx.clone(),
					input: ctx.persisted_actor().input.clone(),
					is_new,
				})
				.await
				.map_err(|source| StartupError::new(StartupStage::Create, source))?,
		);

		let config = factory.config().clone();
		ctx.configure_sleep(config.clone());
		ctx.configure_connection_runtime(config, callbacks.clone());
		ctx.set_on_state_change_callback(on_state_change_callback(&ctx, &callbacks));
		ctx.set_has_initialized(true);
		ctx.save_state(SaveStateOpts { immediate: true })
			.await
			.map_err(|source| StartupError::new(StartupStage::PersistInitialization, source))?;

		if let Some(on_migrate) = callbacks.on_migrate.as_ref() {
			let started_at = Instant::now();
			match timeout(
				factory.config().on_migrate_timeout,
				on_migrate(OnMigrateRequest {
					ctx: ctx.clone(),
					is_new,
				}),
			)
			.await
			{
				Ok(Ok(())) => {
					tracing::debug!(
						actor_id = ctx.actor_id(),
						on_migrate_ms = started_at.elapsed().as_millis() as u64,
						"actor on_migrate completed"
					);
				}
				Ok(Err(source)) => {
					return Err(StartupError::new(StartupStage::Migrate, source));
				}
				Err(_) => {
					return Err(StartupError::new(
						StartupStage::Migrate,
						anyhow::Error::msg(format!(
							"actor on_migrate timed out after {} ms",
							factory.config().on_migrate_timeout.as_millis()
						)),
					));
				}
			}
		}

		if let Some(on_wake) = callbacks.on_wake.as_ref() {
			on_wake(OnWakeRequest { ctx: ctx.clone() })
				.await
				.map_err(|source| StartupError::new(StartupStage::Wake, source))?;
		}

		ctx.schedule().sync_alarm_logged();
		ctx.restore_hibernatable_connections()
			.await
			.map_err(|source| StartupError::new(StartupStage::RestoreConnections, source))?;

		ctx.set_ready(true);

		if let Some(on_before_actor_start) =
			options.driver_hooks.on_before_actor_start.as_ref()
		{
			if let Err(source) = on_before_actor_start(BeforeActorStartRequest {
				ctx: ctx.clone(),
				callbacks: callbacks.clone(),
				is_new,
			})
			.await
			{
				ctx.set_ready(false);
				return Err(StartupError::new(StartupStage::BeforeActorStart, source));
			}
		}

		ctx.set_started(true);
		ctx.reset_sleep_timer();
		self.spawn_run_handler(ctx.clone(), callbacks.clone());
		self.process_overdue_scheduled_events(&ctx, factory, callbacks.clone())
			.await;

		Ok(StartupOutcome { callbacks, is_new })
	}

	pub async fn shutdown_for_sleep(
		&self,
		ctx: ActorContext,
		factory: &ActorFactory,
		callbacks: Arc<ActorInstanceCallbacks>,
	) -> Result<ShutdownOutcome> {
		let config = factory.config().clone();
		ctx.cancel_sleep_timer();
		ctx.cancel_local_alarm_timeouts();
		ctx.abort_signal().cancel();
		ctx
			.wait_for_run_handler(config.effective_run_stop_timeout())
			.await;

		let shutdown_deadline = Instant::now() + config.effective_sleep_grace_period();
		if !ctx.wait_for_sleep_idle_window(shutdown_deadline).await {
			tracing::warn!(
				timeout_ms = config.effective_sleep_grace_period().as_millis() as u64,
				"sleep shutdown reached the idle wait deadline"
			);
		}

		let mut status = ShutdownStatus::Ok;
		if let Some(on_sleep) = callbacks.on_sleep.as_ref() {
			let on_sleep_timeout =
				remaining_budget(shutdown_deadline).min(config.effective_on_sleep_timeout());
			match timeout(on_sleep_timeout, on_sleep(OnSleepRequest { ctx: ctx.clone() })).await
			{
				Ok(Ok(())) => {}
				Ok(Err(error)) => {
					status = ShutdownStatus::Error;
					tracing::error!(?error, "actor on_sleep failed during sleep shutdown");
				}
				Err(_) => {
					status = ShutdownStatus::Error;
					tracing::error!(
						timeout_ms = on_sleep_timeout.as_millis() as u64,
						"actor on_sleep timed out during sleep shutdown"
					);
				}
			}
		}

		if !ctx.wait_for_shutdown_tasks(shutdown_deadline).await {
			tracing::warn!("sleep shutdown timed out waiting for shutdown tasks");
		}

		ctx.persist_hibernatable_connections()
			.await
			.context("persist hibernatable connections during sleep shutdown")?;

		for conn in ctx.conns() {
			if conn.is_hibernatable() {
				continue;
			}

			if let Err(error) = conn.disconnect(Some("actor sleeping")).await {
				tracing::error!(
					?error,
					conn_id = conn.id(),
					"failed to disconnect connection during sleep shutdown"
				);
			}
		}

		if !ctx.wait_for_shutdown_tasks(shutdown_deadline).await {
			tracing::warn!("sleep shutdown timed out after disconnect callbacks");
		}

		ctx.save_state(SaveStateOpts { immediate: true })
			.await
			.context("persist actor state during sleep shutdown")?;
		ctx.sql()
			.cleanup()
			.await
			.context("cleanup sqlite during sleep shutdown")?;

		Ok(ShutdownOutcome { status })
	}

	pub async fn shutdown_for_destroy(
		&self,
		ctx: ActorContext,
		factory: &ActorFactory,
		callbacks: Arc<ActorInstanceCallbacks>,
	) -> Result<ShutdownOutcome> {
		let config = factory.config().clone();
		ctx.cancel_sleep_timer();
		ctx.cancel_local_alarm_timeouts();
		if !ctx.aborted() {
			ctx.abort_signal().cancel();
		}
		ctx
			.wait_for_run_handler(config.effective_run_stop_timeout())
			.await;

		let mut status = ShutdownStatus::Ok;
		if let Some(on_destroy) = callbacks.on_destroy.as_ref() {
			let on_destroy_timeout = config.effective_on_destroy_timeout();
			match timeout(
				on_destroy_timeout,
				on_destroy(OnDestroyRequest { ctx: ctx.clone() }),
			)
			.await
			{
				Ok(Ok(())) => {}
				Ok(Err(error)) => {
					status = ShutdownStatus::Error;
					tracing::error!(?error, "actor on_destroy failed during destroy shutdown");
				}
				Err(_) => {
					status = ShutdownStatus::Error;
					tracing::error!(
						timeout_ms = on_destroy_timeout.as_millis() as u64,
						"actor on_destroy timed out during destroy shutdown"
					);
				}
			}
		}

		let shutdown_deadline = Instant::now() + config.effective_sleep_grace_period();
		if !ctx.wait_for_shutdown_tasks(shutdown_deadline).await {
			tracing::warn!("destroy shutdown timed out waiting for shutdown tasks");
		}

		for conn in ctx.conns() {
			if let Err(error) = conn.disconnect(Some("actor destroyed")).await {
				tracing::error!(
					?error,
					conn_id = conn.id(),
					"failed to disconnect connection during destroy shutdown"
				);
			}
		}

		if !ctx.wait_for_shutdown_tasks(shutdown_deadline).await {
			tracing::warn!("destroy shutdown timed out after disconnect callbacks");
		}

		ctx.save_state(SaveStateOpts { immediate: true })
			.await
			.context("persist actor state during destroy shutdown")?;
		ctx.sql()
			.cleanup()
			.await
			.context("cleanup sqlite during destroy shutdown")?;

		Ok(ShutdownOutcome { status })
	}

	async fn load_persisted_actor(
		&self,
		ctx: &ActorContext,
		options: &StartupOptions,
	) -> std::result::Result<PersistedActor, StartupError> {
		if let Some(preloaded) = options.preload_persisted_actor.clone() {
			return Ok(preloaded);
		}

		match ctx
			.kv()
			.get(PERSIST_DATA_KEY)
			.await
			.map_err(|source| StartupError::new(StartupStage::LoadPersisted, source))?
		{
			Some(bytes) => decode_persisted_actor(&bytes)
				.context("decode persisted actor startup data")
				.map_err(|source| StartupError::new(StartupStage::LoadPersisted, source)),
			None => Ok(PersistedActor {
				input: options.input.clone(),
				..PersistedActor::default()
			}),
		}
	}

	fn spawn_run_handler(
		&self,
		ctx: ActorContext,
		callbacks: Arc<ActorInstanceCallbacks>,
	) {
		if callbacks.run.is_none() {
			return;
		}

		let Ok(runtime) = Handle::try_current() else {
			tracing::warn!("skipping actor run handler without a tokio runtime");
			return;
		};

		let callbacks = callbacks.clone();
		ctx.set_run_handler_active(true);
		let task_ctx = ctx.clone();
		let handle = runtime.spawn(async move {
			let run = callbacks
				.run
				.as_ref()
				.expect("run handler presence checked before spawn");
			let result = AssertUnwindSafe(run(RunRequest {
				ctx: task_ctx.clone(),
			}))
				.catch_unwind()
				.await;
			task_ctx.set_run_handler_active(false);

			match result {
				Ok(Ok(())) => {}
				Ok(Err(error)) => {
					tracing::error!(?error, "actor run handler failed");
				}
				Err(panic) => {
					let panic_message = panic_payload_message(panic.as_ref());
					tracing::error!(panic = %panic_message, "actor run handler panicked");
				}
			}
		});
		ctx.track_run_handler(handle);
	}

	async fn process_overdue_scheduled_events(
		&self,
		ctx: &ActorContext,
		factory: &ActorFactory,
		callbacks: Arc<ActorInstanceCallbacks>,
	) {
		let invoker =
			ActionInvoker::with_shared_callbacks(factory.config().clone(), callbacks);
		ctx.schedule().handle_alarm(ctx, &invoker).await;
	}
}

impl StartupError {
	pub fn stage(&self) -> StartupStage {
		self.stage
	}

	pub fn into_source(self) -> anyhow::Error {
		self.source
	}

	fn new(stage: StartupStage, source: anyhow::Error) -> Self {
		Self { stage, source }
	}
}

impl fmt::Display for StartupError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "actor startup failed during {}", self.stage)
	}
}

impl StdError for StartupError {
}

impl fmt::Display for StartupStage {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let stage = match self {
			Self::LoadPersisted => "persisted state load",
			Self::Create => "factory create",
			Self::PersistInitialization => "initial persistence",
			Self::Migrate => "on_migrate",
			Self::Wake => "on_wake",
			Self::RestoreConnections => "restore connections",
			Self::BeforeActorStart => "on_before_actor_start",
		};

		f.write_str(stage)
	}
}

impl fmt::Debug for ActorLifecycleDriverHooks {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("ActorLifecycleDriverHooks")
			.field(
				"on_before_actor_start",
				&self.on_before_actor_start.is_some(),
			)
			.finish()
	}
}

fn on_state_change_callback(
	ctx: &ActorContext,
	callbacks: &Arc<ActorInstanceCallbacks>,
) -> Option<OnStateChangeCallback> {
	if callbacks.on_state_change.is_none() {
		return None;
	}

	let ctx = ctx.clone();
	let callbacks = callbacks.clone();
	Some(Arc::new(move || {
		let ctx = ctx.clone();
		let callbacks = callbacks.clone();
		Box::pin(async move {
			let Some(on_state_change) = callbacks.on_state_change.as_ref() else {
				return Ok(());
			};

			on_state_change(OnStateChangeRequest {
				ctx: ctx.clone(),
				new_state: ctx.state(),
			})
			.await
		})
	}))
}

fn panic_payload_message(payload: &(dyn Any + Send + 'static)) -> String {
	if let Some(message) = payload.downcast_ref::<&'static str>() {
		return (*message).to_owned();
	}

	if let Some(message) = payload.downcast_ref::<String>() {
		return message.clone();
	}

	"unknown panic payload".to_owned()
}

fn remaining_budget(deadline: Instant) -> std::time::Duration {
	deadline
		.checked_duration_since(Instant::now())
		.unwrap_or_default()
}

#[cfg(test)]
#[path = "../../tests/modules/lifecycle.rs"]
mod tests;

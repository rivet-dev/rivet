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
	ActorInstanceCallbacks, OnDestroyRequest, OnSleepRequest,
	OnStateChangeRequest, OnWakeRequest, RunRequest,
};
use crate::actor::context::ActorContext;
use crate::actor::factory::{ActorFactory, FactoryRequest};
use crate::actor::state::{OnStateChangeCallback, PersistedActor, PERSIST_DATA_KEY};
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
			Some(bytes) => serde_bare::from_slice(&bytes)
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
mod tests {
	use std::collections::BTreeMap;
	use std::sync::Arc;
	use std::sync::Mutex;
	use std::sync::atomic::{AtomicUsize, Ordering};
	use std::time::{Duration, SystemTime, UNIX_EPOCH};

	use anyhow::anyhow;
	use tokio::sync::oneshot;
	use tokio::time::sleep;

	use super::{
		ActorLifecycle, ActorLifecycleDriverHooks, BeforeActorStartRequest,
		ShutdownStatus, StartupError, StartupOptions, StartupStage,
	};
	use crate::actor::callbacks::{
		ActorInstanceCallbacks, OnDestroyRequest, OnSleepRequest, OnWakeRequest,
		RunRequest,
	};
	use crate::actor::connection::{
		HibernatableConnectionMetadata, PersistedConnection, make_connection_key,
	};
	use crate::actor::factory::ActorFactory;
	use crate::actor::state::{PERSIST_DATA_KEY, PersistedScheduleEvent};
	use crate::actor::sleep::CanSleep;
	use crate::actor::state::PersistedActor;
	use crate::{ActorConfig, ActorContext, Kv};

	#[tokio::test]
	async fn startup_loads_preloaded_state_before_factory_and_starts_after_hook() {
		let lifecycle = ActorLifecycle;
		let ctx = ActorContext::new_with_kv(
			"actor-1",
			"counter",
			Vec::new(),
			"sea",
			Kv::new_in_memory(),
		);
		let wake_calls = Arc::new(AtomicUsize::new(0));
		let hook_calls = Arc::new(AtomicUsize::new(0));

		let preload = PersistedActor {
			input: Some(vec![1, 2, 3]),
			has_initialized: false,
			state: vec![9, 8, 7],
			scheduled_events: Vec::new(),
		};

		let wake_calls_for_factory = wake_calls.clone();
		let factory = ActorFactory::new(Default::default(), move |request| {
			let wake_calls = wake_calls_for_factory.clone();
			Box::pin(async move {
				assert!(request.is_new);
				assert_eq!(request.input, Some(vec![1, 2, 3]));
				assert_eq!(request.ctx.state(), vec![9, 8, 7]);

				let mut callbacks = ActorInstanceCallbacks::default();
				callbacks.on_wake = Some(Box::new(move |request: OnWakeRequest| {
					let wake_calls = wake_calls.clone();
					Box::pin(async move {
						assert_eq!(request.ctx.state(), vec![9, 8, 7]);
						wake_calls.fetch_add(1, Ordering::SeqCst);
						Ok(())
					})
				}));

				Ok(callbacks)
			})
		});

		let hook_calls_for_hook = hook_calls.clone();
		let outcome = lifecycle
			.startup(
				ctx.clone(),
				&factory,
				StartupOptions {
					preload_persisted_actor: Some(preload),
					input: None,
					driver_hooks: ActorLifecycleDriverHooks {
						on_before_actor_start: Some(Arc::new(
							move |request: BeforeActorStartRequest| {
								let hook_calls = hook_calls_for_hook.clone();
								Box::pin(async move {
									assert!(request.is_new);
									assert_eq!(request.ctx.can_sleep().await, CanSleep::NotReady);
									assert!(request.callbacks.on_wake.is_some());
									hook_calls.fetch_add(1, Ordering::SeqCst);
									Ok(())
								})
							},
						)),
					},
				},
			)
			.await
			.expect("startup should succeed");

		assert!(outcome.is_new);
		assert!(outcome.callbacks.on_wake.is_some());
		assert_eq!(wake_calls.load(Ordering::SeqCst), 1);
		assert_eq!(hook_calls.load(Ordering::SeqCst), 1);
		assert_eq!(ctx.persisted_actor().input, Some(vec![1, 2, 3]));
		assert!(ctx.persisted_actor().has_initialized);
		assert_eq!(ctx.can_sleep().await, CanSleep::Yes);
	}

	#[tokio::test]
	async fn startup_marks_restored_actor_as_existing() {
		let lifecycle = ActorLifecycle;
		let ctx = ActorContext::new_with_kv(
			"actor-2",
			"counter",
			Vec::new(),
			"sea",
			Kv::new_in_memory(),
		);

		let factory = ActorFactory::new(Default::default(), move |request| {
			Box::pin(async move {
				assert!(!request.is_new);
				assert_eq!(request.input, Some(vec![4, 5, 6]));
				Ok(ActorInstanceCallbacks::default())
			})
		});

		let outcome = lifecycle
			.startup(
				ctx,
				&factory,
				StartupOptions {
					preload_persisted_actor: Some(PersistedActor {
						input: Some(vec![4, 5, 6]),
						has_initialized: true,
						state: vec![1],
						scheduled_events: Vec::new(),
					}),
					input: Some(vec![9, 9, 9]),
					driver_hooks: ActorLifecycleDriverHooks::default(),
				},
			)
			.await
			.expect("startup should succeed");

		assert!(!outcome.is_new);
	}

	#[tokio::test]
	async fn startup_surfaces_factory_failures_with_stage() {
		let error = run_startup_failure(
			StartupStage::Create,
			|_ctx| {
				ActorFactory::new(Default::default(), move |_request| {
					Box::pin(async { Err(anyhow!("factory exploded")) })
				})
			},
			None,
		)
		.await;

		assert_eq!(error.stage(), StartupStage::Create);
	}

	#[tokio::test]
	async fn startup_persists_has_initialized_before_on_wake_runs() {
		let lifecycle = ActorLifecycle;
		let ctx = ActorContext::new_with_kv(
			"actor-3",
			"counter",
			Vec::new(),
			"sea",
			Kv::new_in_memory(),
		);

		let factory = ActorFactory::new(Default::default(), move |_request| {
			Box::pin(async move {
				let mut callbacks = ActorInstanceCallbacks::default();
				callbacks.on_wake = Some(Box::new(|request: OnWakeRequest| {
					Box::pin(async move {
						assert!(request.ctx.persisted_actor().has_initialized);
						Err(anyhow!("wake exploded"))
					})
				}));
				Ok(callbacks)
			})
		});

		let error = lifecycle
			.startup(
				ctx.clone(),
				&factory,
				StartupOptions {
					preload_persisted_actor: Some(PersistedActor {
						input: Some(vec![1]),
						has_initialized: false,
						state: Vec::new(),
						scheduled_events: Vec::new(),
					}),
					input: None,
					driver_hooks: ActorLifecycleDriverHooks::default(),
				},
			)
			.await
			.expect_err("startup should fail in on_wake");

		assert_eq!(error.stage(), StartupStage::Wake);
		assert!(ctx.persisted_actor().has_initialized);
		assert_eq!(ctx.can_sleep().await, CanSleep::NotReady);
	}

	#[tokio::test]
	async fn startup_restores_connections_and_processes_overdue_events() {
		let lifecycle = ActorLifecycle;
		let ctx = ActorContext::new_with_kv(
			"actor-5",
			"counter",
			Vec::new(),
			"sea",
			Kv::new_in_memory(),
		);
		let fired = Arc::new(AtomicUsize::new(0));
		let fired_for_factory = fired.clone();
		let now = current_timestamp_ms();
		let future_ts = now.saturating_add(60_000);

		let restored_conn = PersistedConnection {
			id: "conn-restored".to_owned(),
			parameters: b"params".to_vec(),
			state: b"state".to_vec(),
			subscriptions: Vec::new(),
			gateway_id: b"gateway".to_vec(),
			request_id: b"request".to_vec(),
			server_message_index: 3,
			client_message_index: 7,
			request_path: "/ws".to_owned(),
			request_headers: BTreeMap::from([(
				"x-test".to_owned(),
				"true".to_owned(),
			)]),
		};
		let restored_bytes =
			serde_bare::to_vec(&restored_conn).expect("persisted connection should encode");
		ctx.kv()
			.put(&make_connection_key("conn-restored"), &restored_bytes)
			.await
			.expect("persisted connection should write");

		let factory = ActorFactory::new(Default::default(), move |_request| {
			let fired = fired_for_factory.clone();
			Box::pin(async move {
				let mut callbacks = ActorInstanceCallbacks::default();
				callbacks.actions.insert(
					"tick".to_owned(),
					Box::new(move |request| {
						let fired = fired.clone();
						Box::pin(async move {
							assert_eq!(request.args, b"due");
							fired.fetch_add(1, Ordering::SeqCst);
							Ok(Vec::new())
						})
					}),
				);
				Ok(callbacks)
			})
		});

		lifecycle
			.startup(
				ctx.clone(),
				&factory,
				StartupOptions {
					preload_persisted_actor: Some(PersistedActor {
						input: None,
						has_initialized: true,
						state: Vec::new(),
						scheduled_events: vec![
							PersistedScheduleEvent {
								event_id: "due".to_owned(),
								timestamp_ms: now.saturating_sub(1),
								action: "tick".to_owned(),
								args: b"due".to_vec(),
							},
							PersistedScheduleEvent {
								event_id: "future".to_owned(),
								timestamp_ms: future_ts,
								action: "later".to_owned(),
								args: b"future".to_vec(),
							},
						],
					}),
					input: None,
					driver_hooks: ActorLifecycleDriverHooks::default(),
				},
			)
			.await
			.expect("startup should succeed");

		assert_eq!(fired.load(Ordering::SeqCst), 1);
		assert_eq!(ctx.conns().len(), 1);
		assert_eq!(ctx.conns()[0].id(), "conn-restored");
		assert_eq!(ctx.schedule().all_events().len(), 1);
		assert_eq!(
			ctx.schedule().next_event().expect("future event").event_id,
			"future"
		);
	}

	#[tokio::test]
	async fn startup_resets_sleep_timer_after_start() {
		let lifecycle = ActorLifecycle;
		let ctx = ActorContext::new_with_kv(
			"actor-6",
			"counter",
			Vec::new(),
			"sea",
			Kv::new_in_memory(),
		);
		let factory = ActorFactory::new(
			ActorConfig {
				sleep_timeout: Duration::from_millis(10),
				..ActorConfig::default()
			},
			move |_request| Box::pin(async move { Ok(ActorInstanceCallbacks::default()) }),
		);

		lifecycle
			.startup(
				ctx.clone(),
				&factory,
				StartupOptions {
					preload_persisted_actor: Some(PersistedActor {
						input: None,
						has_initialized: true,
						state: Vec::new(),
						scheduled_events: Vec::new(),
					}),
					input: None,
					driver_hooks: ActorLifecycleDriverHooks::default(),
				},
			)
			.await
			.expect("startup should succeed");

		sleep(Duration::from_millis(25)).await;
		assert!(ctx.sleep_requested());
	}

	#[tokio::test]
	async fn startup_runs_run_handler_in_background_and_keeps_actor_alive_on_error() {
		let lifecycle = ActorLifecycle;
		let ctx = ActorContext::new_with_kv(
			"actor-7",
			"counter",
			Vec::new(),
			"sea",
			Kv::new_in_memory(),
		);
		let (release_tx, release_rx) = oneshot::channel::<()>();
		let started = Arc::new(AtomicUsize::new(0));
		let started_for_factory = started.clone();
		let release_rx = Arc::new(std::sync::Mutex::new(Some(release_rx)));

		let factory = ActorFactory::new(Default::default(), move |_request| {
			let started = started_for_factory.clone();
			let release_rx = release_rx.clone();
			Box::pin(async move {
				let mut callbacks = ActorInstanceCallbacks::default();
				callbacks.run = Some(Box::new(move |_: RunRequest| {
					let started = started.clone();
					let release_rx = release_rx.clone();
					Box::pin(async move {
						started.fetch_add(1, Ordering::SeqCst);
						let rx = release_rx
							.lock()
							.expect("run release receiver lock poisoned")
							.take()
							.expect("run release receiver should exist");
						let _ = rx.await;
						Err(anyhow!("run exploded"))
					})
				}));
				Ok(callbacks)
			})
		});

		lifecycle
			.startup(
				ctx.clone(),
				&factory,
				StartupOptions {
					preload_persisted_actor: Some(PersistedActor {
						input: None,
						has_initialized: true,
						state: Vec::new(),
						scheduled_events: Vec::new(),
					}),
					input: None,
					driver_hooks: ActorLifecycleDriverHooks::default(),
				},
			)
			.await
			.expect("startup should succeed");

		tokio::task::yield_now().await;
		assert_eq!(started.load(Ordering::SeqCst), 1);
		assert_eq!(ctx.can_sleep().await, CanSleep::ActiveRun);

		release_tx
			.send(())
			.expect("run release should be delivered");
		sleep(Duration::from_millis(10)).await;

		assert_eq!(ctx.can_sleep().await, CanSleep::Yes);
	}

	#[tokio::test]
	async fn startup_catches_run_handler_panics() {
		let lifecycle = ActorLifecycle;
		let ctx = ActorContext::new_with_kv(
			"actor-8",
			"counter",
			Vec::new(),
			"sea",
			Kv::new_in_memory(),
		);
		let panics = Arc::new(AtomicUsize::new(0));
		let panics_for_factory = panics.clone();

		let factory = ActorFactory::new(Default::default(), move |_request| {
			let panics = panics_for_factory.clone();
			Box::pin(async move {
				let mut callbacks = ActorInstanceCallbacks::default();
				callbacks.run = Some(Box::new(move |_: RunRequest| {
					let panics = panics.clone();
					Box::pin(async move {
						panics.fetch_add(1, Ordering::SeqCst);
						panic!("run panic");
					})
				}));
				Ok(callbacks)
			})
		});

		lifecycle
			.startup(
				ctx.clone(),
				&factory,
				StartupOptions {
					preload_persisted_actor: Some(PersistedActor {
						input: None,
						has_initialized: true,
						state: Vec::new(),
						scheduled_events: Vec::new(),
					}),
					input: None,
					driver_hooks: ActorLifecycleDriverHooks::default(),
				},
			)
			.await
			.expect("startup should succeed");

		tokio::task::yield_now().await;
		sleep(Duration::from_millis(10)).await;

		assert_eq!(panics.load(Ordering::SeqCst), 1);
		assert_eq!(ctx.can_sleep().await, CanSleep::Yes);
	}

	#[tokio::test]
	async fn sleep_shutdown_waits_for_idle_window_and_persists_state() {
		let lifecycle = ActorLifecycle;
		let config = ActorConfig {
			sleep_grace_period: Some(Duration::from_millis(200)),
			on_sleep_timeout: Duration::from_millis(50),
			run_stop_timeout: Duration::from_millis(50),
			..ActorConfig::default()
		};
		let ctx = ActorContext::new_with_kv(
			"actor-9",
			"counter",
			Vec::new(),
			"sea",
			Kv::new_in_memory(),
		);
		let on_sleep_calls = Arc::new(AtomicUsize::new(0));
		let idle_gate = Arc::new(AtomicUsize::new(0));
		let disconnects = Arc::new(Mutex::new(Vec::<String>::new()));

		let on_sleep_calls_for_callback = on_sleep_calls.clone();
		let idle_gate_for_callback = idle_gate.clone();
		let disconnects_for_callback = disconnects.clone();
		let mut callbacks = ActorInstanceCallbacks::default();
		callbacks.on_sleep = Some(Box::new(move |request: OnSleepRequest| {
			let on_sleep_calls = on_sleep_calls_for_callback.clone();
			let idle_gate = idle_gate_for_callback.clone();
			Box::pin(async move {
				assert_eq!(idle_gate.load(Ordering::SeqCst), 1);
				assert!(request.ctx.aborted());
				assert_eq!(request.ctx.conns().len(), 2);
				on_sleep_calls.fetch_add(1, Ordering::SeqCst);
				Ok(())
			})
		}));
		callbacks.on_disconnect = Some(Box::new(move |request| {
			let disconnects = disconnects_for_callback.clone();
			Box::pin(async move {
				disconnects
					.lock()
					.expect("disconnects lock poisoned")
					.push(request.conn.id().to_owned());
				Ok(())
			})
		}));
		let callbacks = Arc::new(callbacks);
		let factory =
			ActorFactory::new(config.clone(), move |_request| Box::pin(async move {
				Ok(ActorInstanceCallbacks::default())
			}));

		ctx.configure_sleep(config.clone());
		ctx.configure_connection_runtime(config.clone(), callbacks.clone());
		ctx.load_persisted_actor(PersistedActor {
			input: None,
			has_initialized: true,
			state: b"initial".to_vec(),
			scheduled_events: Vec::new(),
		});
		ctx.set_state(b"updated".to_vec());

		let normal_conn = ctx
			.connect_conn(Vec::new(), false, None, async { Ok(Vec::new()) })
			.await
			.expect("non-hibernatable connection should connect");
		let hibernating_conn = ctx
			.connect_conn(
				Vec::new(),
				true,
				Some(HibernatableConnectionMetadata {
					gateway_id: b"gateway".to_vec(),
					request_id: b"request".to_vec(),
					server_message_index: 3,
					client_message_index: 7,
					request_path: "/ws".to_owned(),
					request_headers: BTreeMap::from([(
						"x-test".to_owned(),
						"true".to_owned(),
					)]),
				}),
				async { Ok(Vec::new()) },
			)
			.await
			.expect("hibernatable connection should connect");

		ctx.begin_internal_keep_awake();
		let idle_release_ctx = ctx.clone();
		let idle_gate_for_release = idle_gate.clone();
		tokio::spawn(async move {
			sleep(Duration::from_millis(20)).await;
			idle_gate_for_release.store(1, Ordering::SeqCst);
			idle_release_ctx.end_internal_keep_awake();
		});

		let outcome = lifecycle
			.shutdown_for_sleep(ctx.clone(), &factory, callbacks)
			.await
			.expect("sleep shutdown should succeed");

		assert_eq!(outcome.status, ShutdownStatus::Ok);
		assert!(ctx.aborted());
		assert_eq!(on_sleep_calls.load(Ordering::SeqCst), 1);
		assert_eq!(
			disconnects.lock().expect("disconnects lock poisoned").as_slice(),
			[normal_conn.id().to_owned()]
		);
		assert_eq!(ctx.conns().len(), 1);
		assert_eq!(ctx.conns()[0].id(), hibernating_conn.id());

		let persisted_conn = ctx
			.kv()
			.get(&make_connection_key(hibernating_conn.id()))
			.await
			.expect("hibernated connection lookup should succeed");
		assert!(persisted_conn.is_some());

		let persisted_actor = ctx
			.kv()
			.get(PERSIST_DATA_KEY)
			.await
			.expect("persisted actor lookup should succeed")
			.expect("persisted actor should exist");
		let persisted_actor: PersistedActor =
			serde_bare::from_slice(&persisted_actor).expect("persisted actor should decode");
		assert_eq!(persisted_actor.state, b"updated");
	}

	#[tokio::test]
	async fn sleep_shutdown_reports_error_when_on_sleep_fails() {
		let lifecycle = ActorLifecycle;
		let config = ActorConfig {
			sleep_grace_period: Some(Duration::from_millis(100)),
			on_sleep_timeout: Duration::from_millis(25),
			..ActorConfig::default()
		};
		let ctx = ActorContext::new_with_kv(
			"actor-10",
			"counter",
			Vec::new(),
			"sea",
			Kv::new_in_memory(),
		);
		let mut raw_callbacks = ActorInstanceCallbacks::default();
		raw_callbacks.on_sleep = Some(Box::new(|_request: OnSleepRequest| {
			Box::pin(async { Err(anyhow!("sleep exploded")) })
		}));
		let callbacks = Arc::new(raw_callbacks);
		let factory =
			ActorFactory::new(config.clone(), move |_request| Box::pin(async move {
				Ok(ActorInstanceCallbacks::default())
			}));

		ctx.configure_sleep(config.clone());
		ctx.configure_connection_runtime(config, callbacks.clone());
		ctx.load_persisted_actor(PersistedActor {
			input: None,
			has_initialized: true,
			state: Vec::new(),
			scheduled_events: Vec::new(),
		});
		ctx.set_state(b"updated".to_vec());
		let normal_conn = ctx
			.connect_conn(Vec::new(), false, None, async { Ok(Vec::new()) })
			.await
			.expect("connection should connect");

		let outcome = lifecycle
			.shutdown_for_sleep(ctx.clone(), &factory, callbacks)
			.await
			.expect("sleep shutdown should continue after on_sleep error");

		assert_eq!(outcome.status, ShutdownStatus::Error);
		assert!(ctx.conns().is_empty());
		let persisted_actor = ctx
			.kv()
			.get(PERSIST_DATA_KEY)
			.await
			.expect("persisted actor lookup should succeed")
			.expect("persisted actor should exist");
		let persisted_actor: PersistedActor =
			serde_bare::from_slice(&persisted_actor).expect("persisted actor should decode");
		assert_eq!(persisted_actor.state, b"updated");
		assert_ne!(normal_conn.id(), "");
	}

	#[tokio::test]
	async fn sleep_shutdown_times_out_run_handler_and_finishes() {
		let lifecycle = ActorLifecycle;
		let ctx = ActorContext::new_with_kv(
			"actor-11",
			"counter",
			Vec::new(),
			"sea",
			Kv::new_in_memory(),
		);
		let factory = ActorFactory::new(
			ActorConfig {
				run_stop_timeout: Duration::from_millis(10),
				sleep_grace_period: Some(Duration::from_millis(40)),
				..ActorConfig::default()
			},
			move |_request| {
				Box::pin(async move {
					let mut callbacks = ActorInstanceCallbacks::default();
					callbacks.run = Some(Box::new(|_: RunRequest| {
						Box::pin(async move {
							std::future::pending::<()>().await;
							Ok(())
						})
					}));
					Ok(callbacks)
				})
			},
		);

		let outcome = lifecycle
			.startup(
				ctx.clone(),
				&factory,
				StartupOptions {
					preload_persisted_actor: Some(PersistedActor {
						input: None,
						has_initialized: true,
						state: Vec::new(),
						scheduled_events: Vec::new(),
					}),
					input: None,
					driver_hooks: ActorLifecycleDriverHooks::default(),
				},
			)
			.await
			.expect("startup should succeed");

		let shutdown = tokio::time::timeout(
			Duration::from_millis(100),
			lifecycle.shutdown_for_sleep(ctx.clone(), &factory, outcome.callbacks),
		)
		.await
		.expect("sleep shutdown should finish before the outer timeout")
		.expect("sleep shutdown should succeed");

		assert_eq!(shutdown.status, ShutdownStatus::Ok);
		assert_ne!(ctx.can_sleep().await, CanSleep::ActiveRun);
	}

	#[tokio::test]
	async fn destroy_shutdown_skips_idle_wait_and_disconnects_all_connections() {
		let lifecycle = ActorLifecycle;
		let config = ActorConfig {
			sleep_grace_period: Some(Duration::from_millis(100)),
			on_destroy_timeout: Duration::from_millis(50),
			run_stop_timeout: Duration::from_millis(50),
			..ActorConfig::default()
		};
		let ctx = ActorContext::new_with_kv(
			"actor-12",
			"counter",
			Vec::new(),
			"sea",
			Kv::new_in_memory(),
		);
		let on_destroy_calls = Arc::new(AtomicUsize::new(0));
		let disconnects = Arc::new(Mutex::new(Vec::<String>::new()));
		let destroy_gate = Arc::new(AtomicUsize::new(0));

		let on_destroy_calls_for_callback = on_destroy_calls.clone();
		let destroy_gate_for_callback = destroy_gate.clone();
		let disconnects_for_callback = disconnects.clone();
		let mut raw_callbacks = ActorInstanceCallbacks::default();
		raw_callbacks.on_destroy = Some(Box::new(move |request: OnDestroyRequest| {
			let on_destroy_calls = on_destroy_calls_for_callback.clone();
			let destroy_gate = destroy_gate_for_callback.clone();
			Box::pin(async move {
				assert_eq!(destroy_gate.load(Ordering::SeqCst), 0);
				assert!(request.ctx.aborted());
				assert_eq!(request.ctx.conns().len(), 2);
				on_destroy_calls.fetch_add(1, Ordering::SeqCst);
				Ok(())
			})
		}));
		raw_callbacks.on_disconnect = Some(Box::new(move |request| {
			let disconnects = disconnects_for_callback.clone();
			Box::pin(async move {
				disconnects
					.lock()
					.expect("disconnects lock poisoned")
					.push(request.conn.id().to_owned());
				Ok(())
			})
		}));
		let callbacks = Arc::new(raw_callbacks);
		let factory =
			ActorFactory::new(config.clone(), move |_request| Box::pin(async move {
				Ok(ActorInstanceCallbacks::default())
			}));

		ctx.configure_sleep(config.clone());
		ctx.configure_connection_runtime(config, callbacks.clone());
		ctx.load_persisted_actor(PersistedActor {
			input: None,
			has_initialized: true,
			state: b"initial".to_vec(),
			scheduled_events: Vec::new(),
		});
		ctx.set_state(b"updated".to_vec());

		let normal_conn = ctx
			.connect_conn(Vec::new(), false, None, async { Ok(Vec::new()) })
			.await
			.expect("non-hibernatable connection should connect");
		let hibernating_conn = ctx
			.connect_conn(
				Vec::new(),
				true,
				Some(HibernatableConnectionMetadata {
					gateway_id: b"gateway".to_vec(),
					request_id: b"request".to_vec(),
					server_message_index: 1,
					client_message_index: 2,
					request_path: "/ws".to_owned(),
					request_headers: BTreeMap::new(),
				}),
				async { Ok(Vec::new()) },
			)
			.await
			.expect("hibernatable connection should connect");

		ctx.begin_internal_keep_awake();
		ctx.wait_until({
			let destroy_gate = destroy_gate.clone();
			async move {
				sleep(Duration::from_millis(20)).await;
				destroy_gate.store(1, Ordering::SeqCst);
			}
		});
		ctx.destroy();

		let outcome = lifecycle
			.shutdown_for_destroy(ctx.clone(), &factory, callbacks)
			.await
			.expect("destroy shutdown should succeed");

		assert_eq!(outcome.status, ShutdownStatus::Ok);
		assert!(ctx.aborted());
		assert_eq!(on_destroy_calls.load(Ordering::SeqCst), 1);
		let disconnects = disconnects.lock().expect("disconnects lock poisoned");
		assert_eq!(disconnects.len(), 2);
		assert!(disconnects.contains(&normal_conn.id().to_owned()));
		assert!(disconnects.contains(&hibernating_conn.id().to_owned()));
		assert!(ctx.conns().is_empty());
		assert!(
			ctx.kv()
				.get(&make_connection_key(hibernating_conn.id()))
				.await
				.expect("persisted connection lookup should succeed")
				.is_none()
		);

		let persisted_actor = ctx
			.kv()
			.get(PERSIST_DATA_KEY)
			.await
			.expect("persisted actor lookup should succeed")
			.expect("persisted actor should exist");
		let persisted_actor: PersistedActor =
			serde_bare::from_slice(&persisted_actor).expect("persisted actor should decode");
		assert_eq!(persisted_actor.state, b"updated");
	}

	#[tokio::test]
	async fn destroy_shutdown_reports_error_when_on_destroy_fails() {
		let lifecycle = ActorLifecycle;
		let config = ActorConfig {
			sleep_grace_period: Some(Duration::from_millis(100)),
			on_destroy_timeout: Duration::from_millis(25),
			..ActorConfig::default()
		};
		let ctx = ActorContext::new_with_kv(
			"actor-13",
			"counter",
			Vec::new(),
			"sea",
			Kv::new_in_memory(),
		);
		let mut raw_callbacks = ActorInstanceCallbacks::default();
		raw_callbacks.on_destroy = Some(Box::new(|_request: OnDestroyRequest| {
			Box::pin(async { Err(anyhow!("destroy exploded")) })
		}));
		let callbacks = Arc::new(raw_callbacks);
		let factory =
			ActorFactory::new(config.clone(), move |_request| Box::pin(async move {
				Ok(ActorInstanceCallbacks::default())
			}));

		ctx.configure_sleep(config.clone());
		ctx.configure_connection_runtime(config, callbacks.clone());
		ctx.load_persisted_actor(PersistedActor {
			input: None,
			has_initialized: true,
			state: Vec::new(),
			scheduled_events: Vec::new(),
		});
		ctx.set_state(b"updated".to_vec());
		ctx.connect_conn(Vec::new(), false, None, async { Ok(Vec::new()) })
			.await
			.expect("connection should connect");

		let outcome = lifecycle
			.shutdown_for_destroy(ctx.clone(), &factory, callbacks)
			.await
			.expect("destroy shutdown should continue after on_destroy error");

		assert_eq!(outcome.status, ShutdownStatus::Error);
		assert!(ctx.aborted());
		assert!(ctx.conns().is_empty());
		let persisted_actor = ctx
			.kv()
			.get(PERSIST_DATA_KEY)
			.await
			.expect("persisted actor lookup should succeed")
			.expect("persisted actor should exist");
		let persisted_actor: PersistedActor =
			serde_bare::from_slice(&persisted_actor).expect("persisted actor should decode");
		assert_eq!(persisted_actor.state, b"updated");
	}

	async fn run_startup_failure<F>(
		expected_stage: StartupStage,
		build_factory: F,
		preload: Option<PersistedActor>,
	) -> StartupError
	where
		F: FnOnce(&ActorContext) -> ActorFactory,
	{
		let lifecycle = ActorLifecycle;
		let ctx = ActorContext::new_with_kv(
			"actor-4",
			"counter",
			Vec::new(),
			"sea",
			Kv::new_in_memory(),
		);
		let factory = build_factory(&ctx);

		let error = lifecycle
			.startup(
				ctx,
				&factory,
				StartupOptions {
					preload_persisted_actor: preload.or_else(|| {
						Some(PersistedActor {
							input: Some(vec![1]),
							has_initialized: false,
							state: Vec::new(),
							scheduled_events: Vec::new(),
						})
					}),
					input: None,
					driver_hooks: ActorLifecycleDriverHooks::default(),
				},
			)
			.await
			.expect_err("startup should fail");

		assert_eq!(error.stage(), expected_stage);
		error
	}

	fn current_timestamp_ms() -> i64 {
		let duration = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.expect("system time should be after epoch");
		i64::try_from(duration.as_millis()).expect("timestamp should fit in i64")
	}
}

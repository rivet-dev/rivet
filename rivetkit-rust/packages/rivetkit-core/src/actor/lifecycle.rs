use std::error::Error as StdError;
use std::fmt;
use std::any::Any;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use anyhow::{Context, Result};
use futures::future::BoxFuture;
use futures::FutureExt;
use tokio::runtime::Handle;

use crate::actor::action::ActionInvoker;
use crate::actor::callbacks::{
	ActorInstanceCallbacks, OnStateChangeRequest, OnWakeRequest, RunRequest,
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
		runtime.spawn(async move {
			let run = callbacks
				.run
				.as_ref()
				.expect("run handler presence checked before spawn");
			let result = AssertUnwindSafe(run(RunRequest { ctx: ctx.clone() }))
				.catch_unwind()
				.await;
			ctx.set_run_handler_active(false);

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

#[cfg(test)]
mod tests {
	use std::collections::BTreeMap;
	use std::sync::Arc;
	use std::sync::atomic::{AtomicUsize, Ordering};
	use std::time::{Duration, SystemTime, UNIX_EPOCH};

	use anyhow::anyhow;
	use tokio::sync::oneshot;
	use tokio::time::sleep;

	use super::{
		ActorLifecycle, ActorLifecycleDriverHooks, BeforeActorStartRequest,
		StartupError, StartupOptions, StartupStage,
	};
	use crate::actor::callbacks::{ActorInstanceCallbacks, OnWakeRequest, RunRequest};
	use crate::actor::connection::{PersistedConnection, make_connection_key};
	use crate::actor::factory::ActorFactory;
	use crate::actor::state::PersistedScheduleEvent;
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

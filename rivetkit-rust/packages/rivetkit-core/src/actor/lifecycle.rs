use std::error::Error as StdError;
use std::fmt;
use std::sync::Arc;

use anyhow::{Context, Result};
use futures::future::BoxFuture;

use crate::actor::callbacks::{
	ActorInstanceCallbacks, OnStateChangeRequest, OnWakeRequest,
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

#[cfg(test)]
mod tests {
	use std::sync::Arc;
	use std::sync::atomic::{AtomicUsize, Ordering};

	use anyhow::anyhow;

	use super::{
		ActorLifecycle, ActorLifecycleDriverHooks, BeforeActorStartRequest,
		StartupError, StartupOptions, StartupStage,
	};
	use crate::actor::callbacks::{ActorInstanceCallbacks, OnWakeRequest};
	use crate::actor::factory::ActorFactory;
	use crate::actor::sleep::CanSleep;
	use crate::actor::state::PersistedActor;
	use crate::{ActorContext, Kv};

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
}

use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow};
use futures::future::BoxFuture;
use rivet_envoy_client::handle::EnvoyHandle;
use tokio::runtime::Handle;
use uuid::Uuid;

use crate::actor::action::{ActionDispatchError, ActionInvoker};
use crate::actor::callbacks::ActionRequest;
use crate::actor::config::ActorConfig;
use crate::actor::connection::ConnHandle;
use crate::actor::context::ActorContext;
use crate::actor::state::{ActorState, PersistedScheduleEvent};
use crate::types::SaveStateOpts;

type InternalKeepAwakeCallback = Arc<
	dyn Fn(BoxFuture<'static, Result<()>>) -> BoxFuture<'static, Result<()>> + Send + Sync,
>;

#[derive(Clone)]
pub struct Schedule(Arc<ScheduleInner>);

struct ScheduleInner {
	state: ActorState,
	actor_id: String,
	generation: Mutex<Option<u32>>,
	config: ActorConfig,
	envoy_handle: Mutex<Option<EnvoyHandle>>,
	#[allow(dead_code)]
	internal_keep_awake: Mutex<Option<InternalKeepAwakeCallback>>,
}

impl Schedule {
	pub fn new(
		state: ActorState,
		actor_id: impl Into<String>,
		config: ActorConfig,
	) -> Self {
		Self(Arc::new(ScheduleInner {
			state,
			actor_id: actor_id.into(),
			generation: Mutex::new(None),
			config,
			envoy_handle: Mutex::new(None),
			internal_keep_awake: Mutex::new(None),
		}))
	}

	pub fn after(&self, duration: Duration, action_name: &str, args: &[u8]) {
		let duration_ms = i64::try_from(duration.as_millis()).unwrap_or(i64::MAX);
		let timestamp_ms = now_timestamp_ms().saturating_add(duration_ms);
		self.at(timestamp_ms, action_name, args);
	}

	pub fn at(&self, timestamp_ms: i64, action_name: &str, args: &[u8]) {
		if let Err(error) = self.schedule_event(timestamp_ms, action_name, args) {
			tracing::error!(
				?error,
				action_name,
				timestamp_ms,
				"failed to schedule actor event"
			);
		}
	}

	#[allow(dead_code)]
	pub(crate) fn configure_envoy(
		&self,
		envoy_handle: EnvoyHandle,
		generation: Option<u32>,
	) {
		*self
			.0
			.envoy_handle
			.lock()
			.expect("schedule envoy handle lock poisoned") = Some(envoy_handle);
		*self
			.0
			.generation
			.lock()
			.expect("schedule generation lock poisoned") = generation;
	}

	#[allow(dead_code)]
	pub(crate) fn clear_envoy(&self) {
		*self
			.0
			.envoy_handle
			.lock()
			.expect("schedule envoy handle lock poisoned") = None;
		*self
			.0
			.generation
			.lock()
			.expect("schedule generation lock poisoned") = None;
	}

	#[allow(dead_code)]
	pub(crate) fn set_internal_keep_awake(
		&self,
		callback: Option<InternalKeepAwakeCallback>,
	) {
		*self
			.0
			.internal_keep_awake
			.lock()
			.expect("schedule keep-awake lock poisoned") = callback;
	}

	#[allow(dead_code)]
	pub(crate) fn cancel(&self, event_id: &str) -> bool {
		let removed = self.0.state.update_scheduled_events(|events| {
			let before = events.len();
			events.retain(|event| event.event_id != event_id);
			before != events.len()
		});

		if removed {
			self.persist_scheduled_events("persist scheduled events after cancellation");
			self.sync_alarm_logged();
		}

		removed
	}

	pub(crate) fn next_event(&self) -> Option<PersistedScheduleEvent> {
		self.0.state.scheduled_events().into_iter().next()
	}

	#[allow(dead_code)]
	pub(crate) fn all_events(&self) -> Vec<PersistedScheduleEvent> {
		self.0.state.scheduled_events()
	}

	#[allow(dead_code)]
	pub(crate) fn clear_all(&self) {
		self.0.state.set_scheduled_events(Vec::new());
		self.persist_scheduled_events("persist scheduled events after clear");
		self.sync_alarm_logged();
	}

	pub(crate) fn cancel_local_alarm_timeouts(&self) {
	}

	#[allow(dead_code)]
	pub(crate) async fn handle_alarm(
		&self,
		ctx: &ActorContext,
		invoker: &ActionInvoker,
	) -> usize {
		let now_ms = now_timestamp_ms();
		let due_events: Vec<_> = self
			.all_events()
			.into_iter()
			.filter(|event| event.timestamp_ms <= now_ms)
			.collect();

		if due_events.is_empty() {
			self.sync_alarm_logged();
			return 0;
		}

		let keep_awake = self
			.0
			.internal_keep_awake
			.lock()
			.expect("schedule keep-awake lock poisoned")
			.clone();

		for event in &due_events {
			let schedule = self.clone();
			let ctx = ctx.clone();
			let invoker = invoker.clone();
			let event = event.clone();
			let event_for_task = event.clone();
			let task: BoxFuture<'static, Result<()>> = Box::pin(async move {
				schedule
					.invoke_action_by_name(&ctx, &invoker, &event_for_task)
					.await
					.map(|_| ())
					.map_err(|error| anyhow!(error.message))
			});

			let result = if let Some(callback) = keep_awake.clone() {
				callback(task).await
			} else {
				task.await
			};

			if let Err(error) = result {
				tracing::error!(
					?error,
					event_id = event.event_id,
					action_name = event.action,
					"scheduled event execution failed"
				);
			}

			self.cancel(&event.event_id);
		}

		self.sync_alarm_logged();
		due_events.len()
	}

	#[allow(dead_code)]
	pub(crate) async fn invoke_action_by_name(
		&self,
		ctx: &ActorContext,
		invoker: &ActionInvoker,
		event: &PersistedScheduleEvent,
	) -> std::result::Result<Vec<u8>, ActionDispatchError> {
		invoker
			.dispatch(ActionRequest {
				ctx: ctx.clone(),
				conn: ConnHandle::default(),
				name: event.action.clone(),
				args: event.args.clone(),
			})
			.await
	}

	fn schedule_event(
		&self,
		timestamp_ms: i64,
		action_name: &str,
		args: &[u8],
	) -> Result<()> {
		let event = PersistedScheduleEvent {
			event_id: Uuid::new_v4().to_string(),
			timestamp_ms,
			action: action_name.to_owned(),
			args: args.to_vec(),
		};

		self.insert_event_sorted(event);
		self.persist_scheduled_events("persist scheduled events");
		self.sync_alarm()
	}

	fn insert_event_sorted(&self, event: PersistedScheduleEvent) {
		self.0.state.update_scheduled_events(|events| {
			let position = events
				.binary_search_by(|existing| {
					existing
						.timestamp_ms
						.cmp(&event.timestamp_ms)
						.then_with(|| existing.event_id.cmp(&event.event_id))
				})
				.unwrap_or_else(|index| index);
			events.insert(position, event);
		});
	}

	fn persist_scheduled_events(&self, description: &'static str) {
		let Ok(runtime) = Handle::try_current() else {
			tracing::warn!(description, "skipping immediate schedule persistence without runtime");
			return;
		};

		let state = self.0.state.clone();
		runtime.spawn(async move {
			if let Err(error) = state
				.save_state(SaveStateOpts { immediate: true })
				.await
			{
				tracing::error!(?error, description, "failed to persist scheduled events");
			}
		});
	}

	fn sync_alarm(&self) -> Result<()> {
		let next_alarm = self.next_event().map(|event| event.timestamp_ms);
		let envoy_handle = self
			.0
			.envoy_handle
			.lock()
			.expect("schedule envoy handle lock poisoned")
			.clone();

		let Some(envoy_handle) = envoy_handle else {
			tracing::warn!(
				actor_id = self.0.actor_id,
				sleep_timeout_ms = self.0.config.sleep_timeout.as_millis() as u64,
				"schedule alarm sync skipped because envoy handle is not configured"
			);
			return Ok(());
		};

		let generation = *self
			.0
			.generation
			.lock()
			.expect("schedule generation lock poisoned");
		envoy_handle.set_alarm(self.0.actor_id.clone(), next_alarm, generation);
		Ok(())
	}

	#[allow(dead_code)]
	pub(crate) fn sync_alarm_logged(&self) {
		if let Err(error) = self.sync_alarm() {
			tracing::error!(?error, "failed to sync scheduled actor alarm");
		}
	}
}

impl Default for Schedule {
	fn default() -> Self {
		Self::new(ActorState::default(), "", ActorConfig::default())
	}
}

impl std::fmt::Debug for Schedule {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Schedule")
			.field("actor_id", &self.0.actor_id)
			.field("next_event", &self.next_event())
			.finish()
	}
}

fn now_timestamp_ms() -> i64 {
	let duration = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.unwrap_or_default();
	i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
	use std::sync::Arc;
	use std::sync::atomic::{AtomicUsize, Ordering};
	use std::time::Duration;

	use anyhow::{Result, anyhow};
	use futures::future::BoxFuture;

	use super::Schedule;
	use crate::actor::action::ActionInvoker;
	use crate::actor::callbacks::{ActionHandler, ActorInstanceCallbacks};
	use crate::actor::config::ActorConfig;
	use crate::actor::context::ActorContext;
	use crate::actor::state::ActorState;

	fn action_handler<F>(handler: F) -> ActionHandler
	where
		F: Fn(
				crate::actor::callbacks::ActionRequest,
			) -> BoxFuture<'static, Result<Vec<u8>>>
			+ Send
			+ Sync
			+ 'static,
	{
		Box::new(handler)
	}

	#[test]
	fn at_inserts_events_in_timestamp_order() {
		let schedule = Schedule::default();

		schedule.at(50, "later", b"");
		schedule.at(10, "sooner", b"");
		schedule.at(30, "middle", b"");

		let actions: Vec<_> = schedule
			.all_events()
			.into_iter()
			.map(|event| event.action)
			.collect();

		assert_eq!(actions, vec!["sooner", "middle", "later"]);
	}

	#[test]
	fn after_creates_future_event() {
		let schedule = Schedule::default();

		schedule.after(Duration::from_millis(5), "ping", b"abc");

		let event = schedule.next_event().expect("scheduled event should exist");
		assert_eq!(event.action, "ping");
		assert_eq!(event.args, b"abc");
		assert!(event.timestamp_ms >= super::now_timestamp_ms());
	}

	#[tokio::test]
	async fn handle_alarm_dispatches_due_events_and_removes_them() {
		let schedule = Schedule::new(ActorState::default(), "actor-1", ActorConfig::default());
		let ctx = ActorContext::default();
		let mut callbacks = ActorInstanceCallbacks::default();
		let seen = Arc::new(AtomicUsize::new(0));
		let seen_clone = seen.clone();
		callbacks.actions.insert(
			"run".to_owned(),
			action_handler(move |request| {
				let seen_clone = seen_clone.clone();
				Box::pin(async move {
					assert_eq!(request.args, b"payload");
					seen_clone.fetch_add(1, Ordering::SeqCst);
					Ok(Vec::new())
				})
			}),
		);

		let invoker = ActionInvoker::new(ActorConfig::default(), callbacks);
		schedule.at(super::now_timestamp_ms().saturating_sub(1), "run", b"payload");
		schedule.at(super::now_timestamp_ms().saturating_add(60_000), "later", b"");

		let executed = schedule.handle_alarm(&ctx, &invoker).await;

		assert_eq!(executed, 1);
		assert_eq!(seen.load(Ordering::SeqCst), 1);
		assert_eq!(schedule.all_events().len(), 1);
		assert_eq!(schedule.next_event().expect("future event").action, "later");
	}

	#[tokio::test]
	async fn handle_alarm_continues_after_errors_and_uses_keep_awake_wrapper() {
		let schedule = Schedule::new(ActorState::default(), "actor-1", ActorConfig::default());
		let ctx = ActorContext::default();
		let mut callbacks = ActorInstanceCallbacks::default();
		let keep_awake_calls = Arc::new(AtomicUsize::new(0));
		let keep_awake_calls_clone = keep_awake_calls.clone();
		schedule.set_internal_keep_awake(Some(Arc::new(move |future| {
			let keep_awake_calls_clone = keep_awake_calls_clone.clone();
			Box::pin(async move {
				keep_awake_calls_clone.fetch_add(1, Ordering::SeqCst);
				future.await
			})
		})));

		let succeeded = Arc::new(AtomicUsize::new(0));
		let succeeded_clone = succeeded.clone();
		callbacks.actions.insert(
			"ok".to_owned(),
			action_handler(move |_| {
				let succeeded_clone = succeeded_clone.clone();
				Box::pin(async move {
					succeeded_clone.fetch_add(1, Ordering::SeqCst);
					Ok(Vec::new())
				})
			}),
		);
		callbacks.actions.insert(
			"fail".to_owned(),
			action_handler(|_| Box::pin(async move { Err(anyhow!("boom")) })),
		);

		let invoker = ActionInvoker::new(ActorConfig::default(), callbacks);
		schedule.at(super::now_timestamp_ms().saturating_sub(1), "fail", b"");
		schedule.at(super::now_timestamp_ms().saturating_sub(1), "ok", b"");

		let executed = schedule.handle_alarm(&ctx, &invoker).await;

		assert_eq!(executed, 2);
		assert_eq!(keep_awake_calls.load(Ordering::SeqCst), 2);
		assert_eq!(succeeded.load(Ordering::SeqCst), 1);
		assert!(schedule.all_events().is_empty());
	}
}

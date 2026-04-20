use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
#[cfg(test)]
use std::sync::atomic::AtomicUsize;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow};
use futures::future::BoxFuture;
use rivet_envoy_client::handle::EnvoyHandle;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::actor::config::ActorConfig;
use crate::actor::state::{ActorState, PersistedScheduleEvent};

type InternalKeepAwakeCallback = Arc<
	dyn Fn(BoxFuture<'static, Result<()>>) -> BoxFuture<'static, Result<()>> + Send + Sync,
>;
type LocalAlarmCallback = Arc<dyn Fn() -> BoxFuture<'static, ()> + Send + Sync>;

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
	local_alarm_callback: Mutex<Option<LocalAlarmCallback>>,
	local_alarm_task: Mutex<Option<JoinHandle<()>>>,
	local_alarm_epoch: AtomicU64,
	alarm_dispatch_enabled: AtomicBool,
	#[cfg(test)]
	driver_alarm_cancel_count: AtomicUsize,
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
			local_alarm_callback: Mutex::new(None),
			local_alarm_task: Mutex::new(None),
			local_alarm_epoch: AtomicU64::new(0),
			alarm_dispatch_enabled: AtomicBool::new(true),
			#[cfg(test)]
			driver_alarm_cancel_count: AtomicUsize::new(0),
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

	pub fn set_alarm(&self, timestamp_ms: Option<i64>) -> Result<()> {
		let envoy_handle = self
			.0
			.envoy_handle
			.lock()
			.expect("schedule envoy handle lock poisoned")
			.clone()
			.ok_or_else(|| anyhow!("schedule alarm handle is not configured"))?;
		let generation = *self
			.0
			.generation
			.lock()
			.expect("schedule generation lock poisoned");
		envoy_handle.set_alarm(self.0.actor_id.clone(), timestamp_ms, generation);
		Ok(())
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

	pub(crate) fn set_local_alarm_callback(
		&self,
		callback: Option<LocalAlarmCallback>,
	) {
		*self
			.0
			.local_alarm_callback
			.lock()
			.expect("schedule local alarm callback lock poisoned") = callback;
	}

	#[allow(dead_code)]
	pub(crate) fn cancel(&self, event_id: &str) -> bool {
		let removed = self.0.state.update_scheduled_events(|events| {
			let before = events.len();
			events.retain(|event| event.event_id != event_id);
			before != events.len()
		});

		if removed {
			self.persist_scheduled_events("schedule_cancel");
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
		self.persist_scheduled_events("schedule_clear");
		self.sync_alarm_logged();
	}

	pub(crate) fn cancel_local_alarm_timeouts(&self) {
		self.0.local_alarm_epoch.fetch_add(1, Ordering::SeqCst);
		if let Some(handle) = self
			.0
			.local_alarm_task
			.lock()
			.expect("schedule local alarm task lock poisoned")
			.take()
		{
			handle.abort();
		}
	}

	pub(crate) fn cancel_driver_alarm_logged(&self) {
		self.cancel_local_alarm_timeouts();
		#[cfg(test)]
		self
			.0
			.driver_alarm_cancel_count
			.fetch_add(1, Ordering::SeqCst);

		let envoy_handle = self
			.0
			.envoy_handle
			.lock()
			.expect("schedule envoy handle lock poisoned")
			.clone();
		let Some(envoy_handle) = envoy_handle else {
			return;
		};

		let generation = *self
			.0
			.generation
			.lock()
			.expect("schedule generation lock poisoned");
		envoy_handle.set_alarm(self.0.actor_id.clone(), None, generation);
	}

	#[cfg(test)]
	pub(crate) fn test_driver_alarm_cancel_count(&self) -> usize {
		self
			.0
			.driver_alarm_cancel_count
			.load(Ordering::SeqCst)
	}

	pub(crate) async fn wait_for_pending_alarm_writes(&self) {
		// Alarm writes are synchronous EnvoyHandle sends in rivetkit-core. Keep
		// the awaitable boundary so shutdown sequencing mirrors the TS runtime.
	}

	pub(crate) fn due_events(&self, now_ms: i64) -> Vec<PersistedScheduleEvent> {
		if !self.0.alarm_dispatch_enabled.load(Ordering::SeqCst) {
			return Vec::new();
		}

		self
			.all_events()
			.into_iter()
			.filter(|event| event.timestamp_ms <= now_ms)
			.collect()
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
		self.persist_scheduled_events("schedule_insert");
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
		self.0.state.persist_now_tracked(description);
	}

	fn sync_alarm(&self) -> Result<()> {
		let next_alarm = self.next_event().map(|event| event.timestamp_ms);
		self.arm_local_alarm(next_alarm);
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

	fn sync_future_alarm(&self) -> Result<()> {
		let now_ms = now_timestamp_ms();
		let next_alarm = self
			.next_event()
			.and_then(|event| (event.timestamp_ms > now_ms).then_some(event.timestamp_ms));
		self.arm_local_alarm(next_alarm);
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
				"future schedule alarm sync skipped because envoy handle is not configured"
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

	fn arm_local_alarm(&self, next_alarm: Option<i64>) {
		self.cancel_local_alarm_timeouts();

		let Some(next_alarm) = next_alarm else {
			return;
		};

		let has_callback = self
			.0
			.local_alarm_callback
			.lock()
			.expect("schedule local alarm callback lock poisoned")
			.is_some();
		if !has_callback {
			return;
		}

		let Ok(tokio_handle) = Handle::try_current() else {
			return;
		};

		let delay_ms = next_alarm.saturating_sub(now_timestamp_ms()).max(0) as u64;
		let local_alarm_epoch = self.0.local_alarm_epoch.load(Ordering::SeqCst);
		let schedule = self.clone();
		// Intentionally detached but abortable: the handle is stored in
		// `local_alarm_task` and cancelled when alarms are resynced or stopped.
		let handle = tokio_handle.spawn(async move {
			tokio::time::sleep(Duration::from_millis(delay_ms)).await;
			if schedule.0.local_alarm_epoch.load(Ordering::SeqCst) != local_alarm_epoch {
				return;
			}
			let Some(callback) = schedule
				.0
				.local_alarm_callback
				.lock()
				.expect("schedule local alarm callback lock poisoned")
				.clone()
			else {
				return;
			};
			callback().await;
		});

		*self
			.0
			.local_alarm_task
			.lock()
			.expect("schedule local alarm task lock poisoned") = Some(handle);
	}

	#[allow(dead_code)]
	pub(crate) fn sync_alarm_logged(&self) {
		if let Err(error) = self.sync_alarm() {
			tracing::error!(?error, "failed to sync scheduled actor alarm");
		}
	}

	#[allow(dead_code)]
	pub(crate) fn sync_future_alarm_logged(&self) {
		if let Err(error) = self.sync_future_alarm() {
			tracing::error!(?error, "failed to sync future scheduled actor alarm");
		}
	}

	pub(crate) fn suspend_alarm_dispatch(&self) {
		self.0
			.alarm_dispatch_enabled
			.store(false, Ordering::SeqCst);
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

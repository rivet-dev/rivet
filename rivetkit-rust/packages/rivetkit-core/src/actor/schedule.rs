use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use crate::time::{SystemTime, UNIX_EPOCH, sleep};

use anyhow::Result;
use futures::future::BoxFuture;
use rivet_envoy_client::handle::EnvoyHandle;
use tokio::runtime::Handle;
use tokio::sync::oneshot;
use tracing::Instrument;
use uuid::Uuid;

use crate::actor::context::ActorContext;
use crate::actor::state::PersistedScheduleEvent;
use crate::error::ActorRuntime;
#[cfg(feature = "wasm-runtime")]
use crate::runtime::RuntimeSpawner;

pub(super) type InternalKeepAwakeCallback =
	Arc<dyn Fn(BoxFuture<'static, Result<()>>) -> BoxFuture<'static, Result<()>> + Send + Sync>;
pub(super) type LocalAlarmCallback = Arc<dyn Fn() -> BoxFuture<'static, ()> + Send + Sync>;

impl ActorContext {
	#[cfg(test)]
	pub(crate) fn new_for_schedule_tests(actor_id: impl Into<String>) -> Self {
		Self::new(actor_id, "schedule-test", Vec::new(), "local")
	}

	pub fn after(&self, duration: Duration, action_name: &str, args: &[u8]) {
		let duration_ms = i64::try_from(duration.as_millis()).unwrap_or(i64::MAX);
		let timestamp_ms = now_timestamp_ms().saturating_add(duration_ms);
		self.at(timestamp_ms, action_name, args);
	}

	pub fn at(&self, timestamp_ms: i64, action_name: &str, args: &[u8]) {
		if let Err(error) = self.schedule_event(timestamp_ms, action_name, args) {
			tracing::error!(
				actor_id = %self.actor_id(),
				?error,
				action_name,
				timestamp_ms,
				"failed to schedule actor event"
			);
		}
	}

	pub(crate) fn set_schedule_alarm(&self, timestamp_ms: Option<i64>) -> Result<()> {
		let envoy_handle = self.0.schedule_envoy_handle.lock().clone().ok_or_else(|| {
			ActorRuntime::NotConfigured {
				component: "schedule alarm handle".to_owned(),
			}
			.build()
		})?;
		let generation = *self.0.schedule_generation.lock();
		self.set_alarm_tracked(envoy_handle, timestamp_ms, generation);
		Ok(())
	}

	pub(crate) fn configure_schedule_envoy(
		&self,
		envoy_handle: EnvoyHandle,
		generation: Option<u32>,
	) {
		*self.0.schedule_envoy_handle.lock() = Some(envoy_handle);
		*self.0.schedule_generation.lock() = generation;
	}

	pub(crate) fn set_internal_keep_awake(&self, callback: Option<InternalKeepAwakeCallback>) {
		*self.0.schedule_internal_keep_awake.lock() = callback;
	}

	pub(crate) fn set_local_alarm_callback(&self, callback: Option<LocalAlarmCallback>) {
		*self.0.schedule_local_alarm_callback.lock() = callback;
	}

	pub(crate) fn cancel_scheduled_event(&self, event_id: &str) -> bool {
		let removed = self.update_scheduled_events(|events| {
			let before = events.len();
			events.retain(|event| event.event_id != event_id);
			before != events.len()
		});

		if removed {
			tracing::debug!(
				actor_id = %self.actor_id(),
				event_id,
				"scheduled actor event cancelled"
			);
			self.mark_dirty_since_push();
			self.persist_scheduled_events("schedule_cancel");
			self.sync_alarm_logged();
		}

		removed
	}

	pub(crate) fn next_event(&self) -> Option<PersistedScheduleEvent> {
		self.scheduled_events().into_iter().next()
	}

	pub(crate) fn all_events(&self) -> Vec<PersistedScheduleEvent> {
		self.scheduled_events()
	}

	pub(crate) fn cancel_local_alarm_timeouts(&self) {
		self.0
			.schedule_local_alarm_epoch
			.fetch_add(1, Ordering::SeqCst);
		if let Some(handle) = self.0.schedule_local_alarm_task.lock().take() {
			handle.abort();
		}
	}

	pub(crate) fn cancel_driver_alarm_logged(&self) {
		self.cancel_local_alarm_timeouts();
		#[cfg(test)]
		self.0
			.schedule_driver_alarm_cancel_count
			.fetch_add(1, Ordering::SeqCst);

		let envoy_handle = self.0.schedule_envoy_handle.lock().clone();
		let Some(envoy_handle) = envoy_handle else {
			return;
		};

		let generation = *self.0.schedule_generation.lock();
		self.set_alarm_tracked(envoy_handle, None, generation);
	}

	#[cfg(test)]
	pub(crate) fn test_driver_alarm_cancel_count(&self) -> usize {
		self.0
			.schedule_driver_alarm_cancel_count
			.load(Ordering::SeqCst)
	}

	pub(crate) async fn wait_for_pending_alarm_writes(&self) {
		let pending = {
			let mut guard = self.0.schedule_pending_alarm_writes.lock();
			std::mem::take(&mut *guard)
		};
		tracing::debug!(
			actor_id = %self.actor_id(),
			pending_alarm_writes = pending.len(),
			"waiting for pending actor alarm writes"
		);

		for ack_rx in pending {
			let _ = ack_rx.await;
		}
		tracing::debug!(
			actor_id = %self.actor_id(),
			"pending actor alarm writes drained"
		);
	}

	pub(crate) fn due_scheduled_events(&self, now_ms: i64) -> Vec<PersistedScheduleEvent> {
		if !self
			.0
			.schedule_alarm_dispatch_enabled
			.load(Ordering::SeqCst)
		{
			return Vec::new();
		}

		self.all_events()
			.into_iter()
			.filter(|event| event.timestamp <= now_ms)
			.collect()
	}

	fn schedule_event(&self, timestamp_ms: i64, action_name: &str, args: &[u8]) -> Result<()> {
		let event = PersistedScheduleEvent {
			event_id: Uuid::new_v4().to_string(),
			timestamp: timestamp_ms,
			action: action_name.to_owned(),
			args: (!args.is_empty()).then(|| args.to_vec()),
		};
		let event_id = event.event_id.clone();
		let args_len = event.args.as_ref().map_or(0, Vec::len);

		self.insert_event_sorted(event);
		tracing::debug!(
			actor_id = %self.actor_id(),
			event_id,
			action_name,
			timestamp_ms,
			args_len,
			"scheduled actor event added"
		);
		self.mark_dirty_since_push();
		self.persist_scheduled_events("schedule_insert");
		self.sync_alarm()
	}

	fn insert_event_sorted(&self, event: PersistedScheduleEvent) {
		self.update_scheduled_events(|events| {
			let position = events
				.binary_search_by(|existing| {
					existing
						.timestamp
						.cmp(&event.timestamp)
						.then_with(|| existing.event_id.cmp(&event.event_id))
				})
				.unwrap_or_else(|index| index);
			events.insert(position, event);
		});
	}

	fn persist_scheduled_events(&self, description: &'static str) {
		self.persist_now_tracked(description);
	}

	fn mark_dirty_since_push(&self) {
		self.0
			.schedule_dirty_since_push
			.store(true, Ordering::SeqCst);
	}

	fn sync_alarm(&self) -> Result<()> {
		let should_push = self
			.0
			.schedule_dirty_since_push
			.swap(false, Ordering::SeqCst);
		let next_alarm = self.next_event().map(|event| event.timestamp);
		self.arm_local_alarm(next_alarm);
		if !should_push {
			return Ok(());
		}
		// Only dedup concrete future alarms; a dirty `None` still needs to clear
		// the driver alarm on fresh/no-event schedules.
		if next_alarm.is_some() && self.last_pushed_alarm() == next_alarm {
			return Ok(());
		}

		let envoy_handle = self.0.schedule_envoy_handle.lock().clone();

		let Some(envoy_handle) = envoy_handle else {
			self.mark_dirty_since_push();
			tracing::warn!(
				actor_id = self.actor_id(),
				sleep_timeout_ms = self.sleep_state_config().sleep_timeout.as_millis() as u64,
				"schedule alarm sync skipped because envoy handle is not configured"
			);
			return Ok(());
		};

		let generation = *self.0.schedule_generation.lock();
		self.set_alarm_tracked(envoy_handle, next_alarm, generation);
		Ok(())
	}

	fn sync_future_alarm(&self) -> Result<()> {
		let should_push = self
			.0
			.schedule_dirty_since_push
			.swap(false, Ordering::SeqCst);
		let now_ms = now_timestamp_ms();
		let next_alarm = self
			.next_event()
			.and_then(|event| (event.timestamp > now_ms).then_some(event.timestamp));
		self.arm_local_alarm(next_alarm);
		if !should_push {
			return Ok(());
		}
		// Only dedup concrete future alarms; a dirty `None` still needs to clear
		// the driver alarm on fresh/no-event schedules.
		if next_alarm.is_some() && self.last_pushed_alarm() == next_alarm {
			return Ok(());
		}

		let envoy_handle = self.0.schedule_envoy_handle.lock().clone();

		let Some(envoy_handle) = envoy_handle else {
			self.mark_dirty_since_push();
			tracing::warn!(
				actor_id = self.actor_id(),
				sleep_timeout_ms = self.sleep_state_config().sleep_timeout.as_millis() as u64,
				"future schedule alarm sync skipped because envoy handle is not configured"
			);
			return Ok(());
		};

		let generation = *self.0.schedule_generation.lock();
		self.set_alarm_tracked(envoy_handle, next_alarm, generation);
		Ok(())
	}

	fn set_alarm_tracked(
		&self,
		envoy_handle: EnvoyHandle,
		timestamp_ms: Option<i64>,
		generation: Option<u32>,
	) {
		let previous_alarm = self.last_pushed_alarm();
		tracing::debug!(
			actor_id = %self.actor_id(),
			generation,
			old_timestamp_ms = previous_alarm,
			new_timestamp_ms = timestamp_ms,
			"pushing actor alarm to envoy"
		);
		let (ack_tx, ack_rx) = oneshot::channel();
		envoy_handle.set_alarm_with_ack(
			self.actor_id().to_owned(),
			timestamp_ms,
			generation,
			Some(ack_tx),
		);
		self.load_last_pushed_alarm(timestamp_ms);
		if let Ok(handle) = Handle::try_current() {
			let state_ctx = self.clone();
			let (persist_done_tx, persist_done_rx) = oneshot::channel();
			handle.spawn(
				async move {
					let _ = ack_rx.await;
					if let Err(error) = state_ctx.persist_last_pushed_alarm(timestamp_ms).await {
						tracing::error!(
							?error,
							?timestamp_ms,
							"failed to persist last pushed actor alarm"
						);
					}
					let _ = persist_done_tx.send(());
				}
				.in_current_span(),
			);
			self.0
				.schedule_pending_alarm_writes
				.lock()
				.push(persist_done_rx);
			return;
		}

		self.0.schedule_pending_alarm_writes.lock().push(ack_rx);
	}

	fn arm_local_alarm(&self, next_alarm: Option<i64>) {
		self.cancel_local_alarm_timeouts();

		let Some(next_alarm) = next_alarm else {
			return;
		};

		let has_callback = self.0.schedule_local_alarm_callback.lock().is_some();
		if !has_callback {
			return;
		}

		#[cfg(not(feature = "wasm-runtime"))]
		let tokio_handle = match Handle::try_current() {
			Ok(handle) => handle,
			Err(_) => return,
		};

		let delay_ms = next_alarm.saturating_sub(now_timestamp_ms()).max(0) as u64;
		let local_alarm_epoch = self.0.schedule_local_alarm_epoch.load(Ordering::SeqCst);
		let schedule = self.clone();
		tracing::debug!(
			actor_id = %self.actor_id(),
			timestamp_ms = next_alarm,
			delay_ms,
			local_alarm_epoch,
			"local actor alarm armed"
		);
		// Intentionally detached but abortable: the handle is stored in
		// `local_alarm_task` and cancelled when alarms are resynced or stopped.
		let task = async move {
			sleep(Duration::from_millis(delay_ms)).await;
			if schedule.0.schedule_local_alarm_epoch.load(Ordering::SeqCst) != local_alarm_epoch {
				return;
			}
			tracing::debug!(
				timestamp_ms = next_alarm,
				local_alarm_epoch,
				"local actor alarm fired"
			);
			let Some(callback) = schedule.0.schedule_local_alarm_callback.lock().clone() else {
				return;
			};
			callback().await;
		}
		.in_current_span();

		#[cfg(not(feature = "wasm-runtime"))]
		let handle = tokio_handle.spawn(task);

		#[cfg(feature = "wasm-runtime")]
		let handle = RuntimeSpawner::spawn(task);

		*self.0.schedule_local_alarm_task.lock() = Some(handle);
	}

	pub(crate) fn sync_alarm_logged(&self) {
		if let Err(error) = self.sync_alarm() {
			tracing::error!(
				actor_id = %self.actor_id(),
				?error,
				"failed to sync scheduled actor alarm"
			);
		}
	}

	pub(crate) fn sync_future_alarm_logged(&self) {
		if let Err(error) = self.sync_future_alarm() {
			tracing::error!(
				actor_id = %self.actor_id(),
				?error,
				"failed to sync future scheduled actor alarm"
			);
		}
	}

	pub(crate) fn suspend_alarm_dispatch(&self) {
		self.0
			.schedule_alarm_dispatch_enabled
			.store(false, Ordering::SeqCst);
	}
}

fn now_timestamp_ms() -> i64 {
	let duration = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.unwrap_or_default();
	i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
}

// Test shim keeps moved tests in crate-root tests/ with private-module access.
#[cfg(test)]
#[path = "../../tests/schedule.rs"]
mod tests;

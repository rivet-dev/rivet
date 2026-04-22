use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use futures::future::BoxFuture;
use rivet_envoy_client::handle::EnvoyHandle;
use tokio::runtime::Handle;
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::actor::context::ActorContext;
use crate::actor::state::PersistedScheduleEvent;
use crate::error::ActorRuntime;

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
			.filter(|event| event.timestamp_ms <= now_ms)
			.collect()
	}

	fn schedule_event(&self, timestamp_ms: i64, action_name: &str, args: &[u8]) -> Result<()> {
		let event = PersistedScheduleEvent {
			event_id: Uuid::new_v4().to_string(),
			timestamp_ms,
			action: action_name.to_owned(),
			args: args.to_vec(),
		};
		let event_id = event.event_id.clone();
		let args_len = event.args.len();

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
						.timestamp_ms
						.cmp(&event.timestamp_ms)
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
		let next_alarm = self.next_event().map(|event| event.timestamp_ms);
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
			.and_then(|event| (event.timestamp_ms > now_ms).then_some(event.timestamp_ms));
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
			handle.spawn(async move {
				let _ = ack_rx.await;
				if let Err(error) = state_ctx.persist_last_pushed_alarm(timestamp_ms).await {
					tracing::error!(
						?error,
						?timestamp_ms,
						"failed to persist last pushed actor alarm"
					);
				}
				let _ = persist_done_tx.send(());
			});
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

		let Ok(tokio_handle) = Handle::try_current() else {
			return;
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
		let handle = tokio_handle.spawn(async move {
			tokio::time::sleep(Duration::from_millis(delay_ms)).await;
			if schedule.0.schedule_local_alarm_epoch.load(Ordering::SeqCst) != local_alarm_epoch {
				return;
			}
			tracing::debug!(
				actor_id = %schedule.actor_id(),
				timestamp_ms = next_alarm,
				local_alarm_epoch,
				"local actor alarm fired"
			);
			let Some(callback) = schedule.0.schedule_local_alarm_callback.lock().clone() else {
				return;
			};
			callback().await;
		});

		*self.0.schedule_local_alarm_task.lock() = Some(handle);
	}

	pub(crate) fn sync_alarm_logged(&self) {
		if let Err(error) = self.sync_alarm() {
			tracing::error!(?error, "failed to sync scheduled actor alarm");
		}
	}

	pub(crate) fn sync_future_alarm_logged(&self) {
		if let Err(error) = self.sync_future_alarm() {
			tracing::error!(?error, "failed to sync future scheduled actor alarm");
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

#[cfg(test)]
mod tests {
	use std::collections::HashMap;
	use std::sync::Mutex as EnvoySharedMutex;
	use std::sync::atomic::AtomicBool;

	use rivet_envoy_client::config::{
		BoxFuture, EnvoyCallbacks, EnvoyConfig, HttpRequest, HttpResponse, WebSocketHandler,
		WebSocketSender,
	};
	use rivet_envoy_client::context::{SharedContext, WsTxMessage};
	use rivet_envoy_client::envoy::ToEnvoyMessage;
	use rivet_envoy_client::protocol;
	use tokio::sync::mpsc;

	use super::*;

	struct IdleEnvoyCallbacks;

	impl EnvoyCallbacks for IdleEnvoyCallbacks {
		fn on_actor_start(
			&self,
			_handle: EnvoyHandle,
			_actor_id: String,
			_generation: u32,
			_config: protocol::ActorConfig,
			_preloaded_kv: Option<protocol::PreloadedKv>,
			_sqlite_schema_version: u32,
			_sqlite_startup_data: Option<protocol::SqliteStartupData>,
		) -> BoxFuture<anyhow::Result<()>> {
			Box::pin(async { Ok(()) })
		}

		fn on_shutdown(&self) {}

		fn fetch(
			&self,
			_handle: EnvoyHandle,
			_actor_id: String,
			_gateway_id: protocol::GatewayId,
			_request_id: protocol::RequestId,
			_request: HttpRequest,
		) -> BoxFuture<anyhow::Result<HttpResponse>> {
			Box::pin(async { anyhow::bail!("fetch should not run in schedule tests") })
		}

		fn websocket(
			&self,
			_handle: EnvoyHandle,
			_actor_id: String,
			_gateway_id: protocol::GatewayId,
			_request_id: protocol::RequestId,
			_request: HttpRequest,
			_path: String,
			_headers: HashMap<String, String>,
			_is_hibernatable: bool,
			_is_restoring_hibernatable: bool,
			_sender: WebSocketSender,
		) -> BoxFuture<anyhow::Result<WebSocketHandler>> {
			Box::pin(async { anyhow::bail!("websocket should not run in schedule tests") })
		}

		fn can_hibernate(
			&self,
			_actor_id: &str,
			_gateway_id: &protocol::GatewayId,
			_request_id: &protocol::RequestId,
			_request: &HttpRequest,
		) -> BoxFuture<anyhow::Result<bool>> {
			Box::pin(async { Ok(false) })
		}
	}

	fn test_envoy_handle() -> (EnvoyHandle, mpsc::UnboundedReceiver<ToEnvoyMessage>) {
		let (envoy_tx, envoy_rx) = mpsc::unbounded_channel();
		let shared = Arc::new(SharedContext {
			config: EnvoyConfig {
				version: 1,
				endpoint: "http://127.0.0.1:1".to_string(),
				token: None,
				namespace: "test".to_string(),
				pool_name: "test".to_string(),
				prepopulate_actor_names: HashMap::new(),
				metadata: None,
				not_global: true,
				debug_latency_ms: None,
				callbacks: Arc::new(IdleEnvoyCallbacks),
			},
			envoy_key: "test-envoy".to_string(),
			envoy_tx,
			// Forced-std-sync: envoy-client's test SharedContext owns these
			// fields as std mutexes, so construction must match that API.
			actors: Arc::new(EnvoySharedMutex::new(HashMap::new())),
			live_tunnel_requests: Arc::new(EnvoySharedMutex::new(HashMap::new())),
			pending_hibernation_restores: Arc::new(EnvoySharedMutex::new(HashMap::new())),
			ws_tx: Arc::new(tokio::sync::Mutex::new(
				None::<mpsc::UnboundedSender<WsTxMessage>>,
			)),
			protocol_metadata: Arc::new(tokio::sync::Mutex::new(None)),
			shutting_down: AtomicBool::new(false),
		});

		(EnvoyHandle::from_shared(shared), envoy_rx)
	}

	fn recv_alarm_now(
		rx: &mut mpsc::UnboundedReceiver<ToEnvoyMessage>,
		expected_actor_id: &str,
		expected_generation: Option<u32>,
	) -> Option<i64> {
		match rx.try_recv() {
			Ok(ToEnvoyMessage::SetAlarm {
				actor_id,
				generation,
				alarm_ts,
				ack_tx,
			}) => {
				assert_eq!(actor_id, expected_actor_id);
				assert_eq!(generation, expected_generation);
				if let Some(ack_tx) = ack_tx {
					let _ = ack_tx.send(());
				}
				alarm_ts
			}
			Ok(_) => panic!("expected set_alarm envoy message"),
			Err(error) => panic!("expected set_alarm envoy message, got {error:?}"),
		}
	}

	fn assert_no_alarm(rx: &mut mpsc::UnboundedReceiver<ToEnvoyMessage>) {
		assert!(matches!(
			rx.try_recv(),
			Err(mpsc::error::TryRecvError::Empty)
		));
	}

	#[test]
	fn sync_alarm_skips_driver_push_until_schedule_changes() {
		let schedule = ActorContext::new_for_schedule_tests("actor-schedule-dirty");
		let (handle, mut rx) = test_envoy_handle();
		schedule.configure_schedule_envoy(handle, Some(7));

		schedule.sync_alarm_logged();
		assert_eq!(
			recv_alarm_now(&mut rx, "actor-schedule-dirty", Some(7)),
			None
		);

		schedule.sync_alarm_logged();
		assert_no_alarm(&mut rx);

		schedule.at(123, "tick", b"args");
		assert_eq!(
			recv_alarm_now(&mut rx, "actor-schedule-dirty", Some(7)),
			Some(123)
		);

		schedule.sync_alarm_logged();
		assert_no_alarm(&mut rx);

		let event_id = schedule
			.next_event()
			.expect("scheduled event should exist")
			.event_id;
		assert!(schedule.cancel_scheduled_event(&event_id));
		assert_eq!(
			recv_alarm_now(&mut rx, "actor-schedule-dirty", Some(7)),
			None
		);

		schedule.sync_alarm_logged();
		assert_no_alarm(&mut rx);
	}

	#[test]
	fn sync_future_alarm_uses_dirty_since_push_gate() {
		let schedule = ActorContext::new_for_schedule_tests("actor-future-alarm-dirty");
		let (handle, mut rx) = test_envoy_handle();
		schedule.configure_schedule_envoy(handle, Some(8));

		let future_ts = now_timestamp_ms() + 60_000;
		schedule.set_scheduled_events(vec![PersistedScheduleEvent {
			event_id: "event-1".to_owned(),
			timestamp_ms: future_ts,
			action: "tick".to_owned(),
			args: vec![1, 2, 3],
		}]);

		schedule.sync_future_alarm_logged();
		assert_eq!(
			recv_alarm_now(&mut rx, "actor-future-alarm-dirty", Some(8)),
			Some(future_ts)
		);

		schedule.sync_future_alarm_logged();
		assert_no_alarm(&mut rx);
	}
}

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant as StdInstant};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
#[cfg(test)]
use tokio::time::timeout;
use tracing::Instrument;

use crate::actor::connection::make_connection_key;
use crate::actor::context::ActorContext;
use crate::actor::messages::StateDelta;
use crate::actor::persist::{decode_with_embedded_version, encode_with_embedded_version};
use crate::actor::task::{
	LIFECYCLE_EVENT_INBOX_CHANNEL, LifecycleEvent, actor_channel_overloaded_error,
};
use crate::actor::task_types::StateMutationReason;
use crate::error::ActorRuntime;
use crate::types::SaveStateOpts;

pub const PERSIST_DATA_KEY: &[u8] = &[1];
pub const LAST_PUSHED_ALARM_KEY: &[u8] = &[6];
const ACTOR_PERSIST_VERSION: u16 = 4;
const ACTOR_PERSIST_COMPATIBLE_VERSIONS: &[u16] = &[3, 4];
const LAST_PUSHED_ALARM_VERSION: u16 = 1;
const LAST_PUSHED_ALARM_COMPATIBLE_VERSIONS: &[u16] = &[1];

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedScheduleEvent {
	pub event_id: String,
	pub timestamp_ms: i64,
	pub action: String,
	pub args: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedActor {
	pub input: Option<Vec<u8>>,
	pub has_initialized: bool,
	pub state: Vec<u8>,
	pub scheduled_events: Vec<PersistedScheduleEvent>,
}

pub(crate) fn encode_persisted_actor(actor: &PersistedActor) -> Result<Vec<u8>> {
	encode_with_embedded_version(actor, ACTOR_PERSIST_VERSION, "persisted actor")
}

pub(crate) fn decode_persisted_actor(payload: &[u8]) -> Result<PersistedActor> {
	decode_with_embedded_version(
		payload,
		ACTOR_PERSIST_COMPATIBLE_VERSIONS,
		"persisted actor",
	)
}

pub(crate) fn encode_last_pushed_alarm(alarm_ts: Option<i64>) -> Result<Vec<u8>> {
	encode_with_embedded_version(&alarm_ts, LAST_PUSHED_ALARM_VERSION, "last pushed alarm")
}

pub(crate) fn decode_last_pushed_alarm(payload: &[u8]) -> Result<Option<i64>> {
	decode_with_embedded_version(
		payload,
		LAST_PUSHED_ALARM_COMPATIBLE_VERSIONS,
		"last pushed alarm",
	)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RequestSaveOpts {
	pub immediate: bool,
	pub max_wait_ms: Option<u32>,
}

pub(super) struct PendingSave {
	scheduled_at: StdInstant,
	handle: JoinHandle<()>,
}

pub struct OnStateChangeGuard {
	ctx: Option<ActorContext>,
}

impl OnStateChangeGuard {
	fn new(ctx: ActorContext) -> Self {
		ctx.on_state_change_started();
		Self { ctx: Some(ctx) }
	}
}

impl Drop for OnStateChangeGuard {
	fn drop(&mut self) {
		if let Some(ctx) = self.ctx.take() {
			ctx.on_state_change_finished();
		}
	}
}

impl ActorContext {
	pub fn state(&self) -> Vec<u8> {
		self.0.current_state.read().clone()
	}

	pub(crate) async fn persist_state(&self, opts: SaveStateOpts) -> Result<()> {
		if !self.is_dirty() {
			return Ok(());
		}

		let result = if opts.immediate {
			self.clear_pending_save();
			self.persist_if_dirty().await
		} else {
			let delay = self.compute_save_delay(None);
			if !delay.is_zero() {
				tokio::time::sleep(delay).await;
			}
			self.persist_if_dirty().await
		};
		result?;
		self.record_state_updated();
		Ok(())
	}

	/// Foreign-runtime bootstrap hook for installing the actor state snapshot
	/// before the actor starts handling lifecycle/dispatch work.
	pub fn set_state_initial(&self, state: Vec<u8>) {
		self.set_initial_state(state);
	}

	/// Fire-and-forget save request helper.
	///
	/// If the lifecycle event inbox is overloaded or unavailable, this only logs
	/// a warning and returns. That `warn!` is the sole failure signal for this
	/// path; callers do not receive a `Result`. Call
	/// [`Self::request_save_and_wait`] when the caller must observe
	/// save-request delivery failures.
	pub fn request_save(&self, opts: RequestSaveOpts) {
		if let Err(error) = self.request_save_with_revision(opts) {
			tracing::warn!(?error, "failed to request actor state save");
		}
	}

	pub async fn request_save_and_wait(&self, opts: RequestSaveOpts) -> Result<()> {
		let save_request_revision = self.request_save_with_revision(opts)?;
		self.wait_for_save_request(save_request_revision).await;
		Ok(())
	}

	pub async fn save_state(&self, deltas: Vec<StateDelta>) -> Result<()> {
		let save_request_revision = self.save_request_revision();
		self.save_state_with_revision(deltas, save_request_revision)
			.await
	}

	pub(crate) fn request_save_with_revision(&self, opts: RequestSaveOpts) -> Result<u64> {
		let immediate = opts.immediate;
		let save_request_revision = self.0.save_request_revision.fetch_add(1, Ordering::SeqCst) + 1;
		self.notify_request_save_hooks(opts);
		let already_requested = self.0.save_requested.swap(true, Ordering::SeqCst);
		let immediate_already_requested = if immediate {
			self.0.save_requested_immediate.swap(true, Ordering::SeqCst)
		} else {
			self.0.save_requested_immediate.load(Ordering::SeqCst)
		};

		if let Some(max_wait_ms) = opts.max_wait_ms {
			let deadline = StdInstant::now() + Duration::from_millis(u64::from(max_wait_ms));
			let mut requested_deadline = self.0.save_requested_within_deadline.lock();
			*requested_deadline = Some(match *requested_deadline {
				Some(existing) => existing.min(deadline),
				None => deadline,
			});
		}

		let Some(sender) = self.lifecycle_event_sender() else {
			return Err(ActorRuntime::NotConfigured {
				component: "lifecycle events".to_owned(),
			}
			.build());
		};

		if opts.max_wait_ms.is_none()
			&& already_requested
			&& (!immediate || immediate_already_requested)
		{
			return Ok(save_request_revision);
		}

		match sender.try_reserve() {
			Ok(permit) => {
				permit.send(LifecycleEvent::SaveRequested { immediate });
				Ok(save_request_revision)
			}
			Err(_) => Err(actor_channel_overloaded_error(
				LIFECYCLE_EVENT_INBOX_CHANNEL,
				self.0.lifecycle_event_inbox_capacity,
				"save_requested",
				Some(&self.0.metrics),
			)),
		}
	}

	pub(crate) async fn wait_for_save_request(&self, save_request_revision: u64) {
		loop {
			if self.0.save_completed_revision.load(Ordering::SeqCst) >= save_request_revision {
				return;
			}

			self.0.save_completion.notified().await;
		}
	}

	pub(crate) fn save_requested(&self) -> bool {
		self.0.save_requested.load(Ordering::SeqCst)
	}

	pub(crate) fn save_requested_immediate(&self) -> bool {
		self.0.save_requested_immediate.load(Ordering::SeqCst)
	}

	pub(crate) fn save_deadline(&self, immediate: bool) -> tokio::time::Instant {
		self.compute_save_deadline(immediate).into()
	}

	pub(crate) fn compute_save_deadline(&self, immediate: bool) -> StdInstant {
		if immediate || self.save_requested_immediate() {
			return StdInstant::now();
		}

		let throttled_deadline = StdInstant::now() + self.compute_save_delay(None);
		let requested_deadline = *self.0.save_requested_within_deadline.lock();

		match requested_deadline {
			Some(requested_deadline) => throttled_deadline.min(requested_deadline),
			None => throttled_deadline,
		}
	}

	pub(crate) fn save_request_revision(&self) -> u64 {
		self.0.save_request_revision.load(Ordering::SeqCst)
	}

	pub(crate) async fn apply_state_deltas(
		&self,
		deltas: Vec<StateDelta>,
		save_request_revision: u64,
	) -> Result<()> {
		let delta_count = deltas.len();
		let delta_bytes: usize = deltas.iter().map(StateDelta::payload_len).sum();
		let current_revision = self.0.state_revision.load(Ordering::SeqCst);
		tracing::debug!(
			delta_count,
			delta_bytes,
			state_revision = current_revision,
			save_request_revision,
			"applying actor state deltas"
		);
		self.clear_pending_save();

		if deltas.is_empty() {
			self.mark_save_request_completed(save_request_revision);
			self.finish_save_request(save_request_revision);
			tracing::debug!(
				delta_count,
				state_revision = current_revision,
				save_request_revision,
				"actor state deltas applied without kv write"
			);
			return Ok(());
		}

		let (puts, deletes, next_state, revision, _write_guard) = {
			let _save_guard = self.0.save_guard.lock().await;
			let revision = self.0.state_revision.load(Ordering::SeqCst);
			let mut persisted = self.persisted();
			let mut next_state = None;
			let mut puts = Vec::new();
			let mut deletes = Vec::new();

			for delta in deltas {
				match delta {
					StateDelta::ActorState(bytes) => {
						next_state = Some(bytes.clone());
						persisted.state = bytes;
					}
					StateDelta::ConnHibernation { conn, bytes } => {
						puts.push((make_connection_key(&conn), bytes));
					}
					StateDelta::ConnHibernationRemoved(conn) => {
						deletes.push(make_connection_key(&conn));
					}
				}
			}

			if next_state.is_some() {
				let encoded =
					encode_persisted_actor(&persisted).context("encode persisted actor state")?;
				puts.push((PERSIST_DATA_KEY.to_vec(), encoded));
				*self.0.persisted.write() = persisted;
			}

			(puts, deletes, next_state, revision, self.begin_write())
		};

		self.0
			.kv
			.apply_batch(&puts, &deletes)
			.await
			.context("persist actor state deltas to kv")?;

		if let Some(state) = next_state {
			*self.0.current_state.write() = state;
		}

		*self.0.last_save_at.lock() = Some(StdInstant::now());

		if self.0.state_revision.load(Ordering::SeqCst) == revision {
			self.0.state_dirty.store(false, Ordering::SeqCst);
		}

		self.mark_save_request_completed(save_request_revision);
		self.finish_save_request(save_request_revision);
		tracing::debug!(
			delta_count,
			delta_bytes,
			state_revision = self.0.state_revision.load(Ordering::SeqCst),
			save_request_revision,
			"actor state deltas applied"
		);
		Ok(())
	}

	pub(crate) async fn wait_for_pending_writes(&self) {
		loop {
			if let Some(handle) = self.take_tracked_persist() {
				let _ = handle.await;
				continue;
			}

			let save_guard = self.0.save_guard.lock().await;
			if self.has_tracked_persist() {
				drop(save_guard);
				continue;
			}

			if self.0.in_flight_state_writes.load(Ordering::SeqCst) == 0 {
				return;
			}
			drop(save_guard);

			self.wait_for_in_flight_writes().await;
		}
	}

	pub(crate) async fn wait_for_pending_state_writes(&self) {
		self.wait_for_pending_writes().await;
	}

	pub fn begin_on_state_change(&self) -> OnStateChangeGuard {
		OnStateChangeGuard::new(self.clone())
	}

	pub fn on_state_change_started(&self) {
		self.0
			.on_state_change_in_flight
			.fetch_add(1, Ordering::SeqCst);
		self.0.sleep.work.keep_awake.increment();
		self.reset_sleep_timer();
	}

	pub fn on_state_change_finished(&self) {
		let previous = self.0.on_state_change_in_flight.fetch_update(
			Ordering::SeqCst,
			Ordering::SeqCst,
			|count| count.checked_sub(1),
		);

		match previous {
			Ok(1) => {
				self.0.sleep.work.keep_awake.decrement();
				self.0.on_state_change_idle.notify_waiters();
				self.reset_sleep_timer();
			}
			Ok(_) => {
				self.0.sleep.work.keep_awake.decrement();
				self.reset_sleep_timer();
			}
			Err(_) => {
				tracing::warn!(
					actor_id = %self.actor_id(),
					"on_state_change finished without a matching start"
				);
			}
		}
	}

	#[cfg(test)]
	#[allow(dead_code)]
	pub(crate) async fn wait_for_on_state_change_idle(&self, timeout_duration: Duration) -> bool {
		if self.0.on_state_change_in_flight.load(Ordering::SeqCst) == 0 {
			return true;
		}

		timeout(timeout_duration, async {
			loop {
				let idle = self.0.on_state_change_idle.notified();
				tokio::pin!(idle);
				idle.as_mut().enable();

				if self.0.on_state_change_in_flight.load(Ordering::SeqCst) == 0 {
					return;
				}

				idle.await;
			}
		})
		.await
		.is_ok()
	}

	pub fn persisted(&self) -> PersistedActor {
		self.0.persisted.read().clone()
	}

	pub fn load_persisted(&self, persisted: PersistedActor) {
		let state = persisted.state.clone();
		*self.0.persisted.write() = persisted;
		*self.0.current_state.write() = state;
		self.0.state_dirty.store(false, Ordering::SeqCst);
		self.finish_save_request(self.save_request_revision());
		self.0
			.metrics
			.inc_state_mutation(StateMutationReason::InternalReplace);
	}

	pub(crate) fn load_last_pushed_alarm(&self, alarm_ts: Option<i64>) {
		*self.0.last_pushed_alarm.write() = alarm_ts;
	}

	pub(crate) fn last_pushed_alarm(&self) -> Option<i64> {
		*self.0.last_pushed_alarm.read()
	}

	pub(crate) async fn persist_last_pushed_alarm(&self, alarm_ts: Option<i64>) -> Result<()> {
		let encoded = encode_last_pushed_alarm(alarm_ts).context("encode last pushed alarm")?;
		self.0
			.kv
			.put(LAST_PUSHED_ALARM_KEY, &encoded)
			.await
			.context("persist last pushed alarm to kv")?;
		self.load_last_pushed_alarm(alarm_ts);
		Ok(())
	}

	pub(crate) fn set_initial_state(&self, state: Vec<u8>) {
		*self.0.current_state.write() = state.clone();
		self.0.persisted.write().state = state;
		self.0.state_dirty.store(true, Ordering::SeqCst);
		self.0.state_revision.fetch_add(1, Ordering::SeqCst);
	}

	pub fn scheduled_events(&self) -> Vec<PersistedScheduleEvent> {
		self.0.persisted.read().scheduled_events.clone()
	}

	pub fn set_scheduled_events(&self, scheduled_events: Vec<PersistedScheduleEvent>) {
		self.0.persisted.write().scheduled_events = scheduled_events;
		self.0
			.metrics
			.inc_state_mutation(StateMutationReason::ScheduledEventsUpdate);
		self.mark_dirty();
		self.schedule_save(None);
	}

	pub(crate) fn update_scheduled_events<R>(
		&self,
		update: impl FnOnce(&mut Vec<PersistedScheduleEvent>) -> R,
	) -> R {
		let result = {
			let mut persisted = self.0.persisted.write();
			update(&mut persisted.scheduled_events)
		};

		self.0
			.metrics
			.inc_state_mutation(StateMutationReason::ScheduledEventsUpdate);
		self.mark_dirty();
		self.schedule_save(None);
		result
	}

	pub fn set_input(&self, input: Option<Vec<u8>>) {
		self.0.persisted.write().input = input;
		self.0
			.metrics
			.inc_state_mutation(StateMutationReason::InputSet);
		self.mark_dirty();
		self.schedule_save(None);
	}

	pub fn input(&self) -> Option<Vec<u8>> {
		self.0.persisted.read().input.clone()
	}

	pub fn set_has_initialized(&self, has_initialized: bool) {
		self.0.persisted.write().has_initialized = has_initialized;
		self.0
			.metrics
			.inc_state_mutation(StateMutationReason::HasInitialized);
		self.mark_dirty();
		self.schedule_save(None);
	}

	pub fn has_initialized(&self) -> bool {
		self.0.persisted.read().has_initialized
	}

	pub fn flush_on_shutdown(&self) {
		self.persist_now_tracked("shutdown_flush");
	}

	pub fn on_request_save(&self, hook: Box<dyn Fn(RequestSaveOpts) + Send + Sync>) {
		self.0.request_save_hooks.write().push(Arc::from(hook));
	}

	fn is_dirty(&self) -> bool {
		self.0.state_dirty.load(Ordering::SeqCst)
	}

	fn mark_dirty(&self) {
		self.0.state_dirty.store(true, Ordering::SeqCst);
		self.0.state_revision.fetch_add(1, Ordering::SeqCst);
	}

	fn lifecycle_event_sender(&self) -> Option<mpsc::Sender<LifecycleEvent>> {
		self.0.lifecycle_events.read().clone()
	}

	fn compute_save_delay(&self, max_wait: Option<Duration>) -> Duration {
		let elapsed = self
			.0
			.last_save_at
			.lock()
			.map(|instant| instant.elapsed())
			.unwrap_or_default();

		throttled_save_delay(self.0.state_save_interval, elapsed, max_wait)
	}

	fn schedule_save(&self, max_wait: Option<Duration>) {
		if !self.is_dirty() {
			return;
		}

		let Ok(tokio_handle) = Handle::try_current() else {
			return;
		};

		let delay = self.compute_save_delay(max_wait);
		let scheduled_at = StdInstant::now() + delay;

		let mut pending_save = self.0.pending_save.lock();

		if let Some(existing) = pending_save.as_ref() {
			if existing.scheduled_at <= scheduled_at {
				return;
			}

			existing.handle.abort();
		}

		let state = self.clone();
		// Intentionally detached but abortable: pending delayed saves are
		// retained in `pending_save`, replaced by newer saves, and awaited at
		// shutdown through the state save guard.
		let handle = tokio_handle.spawn(
			async move {
				if !delay.is_zero() {
					tokio::time::sleep(delay).await;
				}

				state.take_pending_save();

				if let Err(error) = state.persist_if_dirty().await {
					tracing::error!(?error, "failed to persist actor state");
				}
			}
			.in_current_span(),
		);

		*pending_save = Some(PendingSave {
			scheduled_at,
			handle,
		});
	}

	fn clear_pending_save(&self) {
		if let Some(pending_save) = self.take_pending_save() {
			pending_save.handle.abort();
		}
	}

	pub(crate) fn persist_now_tracked(&self, description: &'static str) {
		self.clear_pending_save();

		let Ok(tokio_handle) = Handle::try_current() else {
			tracing::warn!(
				description,
				"skipping tracked actor state persistence without runtime"
			);
			return;
		};

		let state = self.clone();
		let mut tracked_persist = self.0.tracked_persist.lock();
		let previous = tracked_persist.take();
		let handle = tokio_handle.spawn(
			async move {
				if let Some(previous) = previous {
					let _ = previous.await;
				}

				if let Err(error) = state.persist_state(SaveStateOpts { immediate: true }).await {
					tracing::error!(?error, description, "failed to persist actor state");
				}
			}
			.in_current_span(),
		);
		*tracked_persist = Some(handle);
	}

	fn take_pending_save(&self) -> Option<PendingSave> {
		self.0.pending_save.lock().take()
	}

	fn take_tracked_persist(&self) -> Option<JoinHandle<()>> {
		self.0.tracked_persist.lock().take()
	}

	fn has_tracked_persist(&self) -> bool {
		self.0.tracked_persist.lock().is_some()
	}

	#[cfg(test)]
	pub(crate) fn tracked_persist_pending(&self) -> bool {
		self.has_tracked_persist()
	}

	async fn persist_if_dirty(&self) -> Result<()> {
		if !self.is_dirty() {
			return Ok(());
		}

		let (revision, encoded, _write_guard) = {
			let _save_guard = self.0.save_guard.lock().await;
			if !self.is_dirty() {
				return Ok(());
			}

			let revision = self.0.state_revision.load(Ordering::SeqCst);
			let persisted = self.persisted();
			let encoded =
				encode_persisted_actor(&persisted).context("encode persisted actor state")?;

			(revision, encoded, self.begin_write())
		};

		self.0
			.kv
			.put(PERSIST_DATA_KEY, &encoded)
			.await
			.context("persist actor state to kv")?;

		*self.0.last_save_at.lock() = Some(StdInstant::now());

		if self.0.state_revision.load(Ordering::SeqCst) == revision {
			self.0.state_dirty.store(false, Ordering::SeqCst);
		}

		Ok(())
	}

	fn begin_write(&self) -> InFlightWrite {
		self.0.in_flight_state_writes.fetch_add(1, Ordering::SeqCst);
		InFlightWrite { ctx: self.clone() }
	}

	async fn wait_for_in_flight_writes(&self) {
		loop {
			if self.0.in_flight_state_writes.load(Ordering::SeqCst) == 0 {
				return;
			}
			self.0.state_write_completion.notified().await;
		}
	}

	fn finish_save_request(&self, save_request_revision: u64) {
		if self.0.save_request_revision.load(Ordering::SeqCst) == save_request_revision {
			self.0.save_requested.store(false, Ordering::SeqCst);
			self.0
				.save_requested_immediate
				.store(false, Ordering::SeqCst);
			*self.0.save_requested_within_deadline.lock() = None;
		}
	}

	fn mark_save_request_completed(&self, save_request_revision: u64) {
		self.0
			.save_completed_revision
			.fetch_max(save_request_revision, Ordering::SeqCst);
		self.0.save_completion.notify_waiters();
	}

	fn notify_request_save_hooks(&self, opts: RequestSaveOpts) {
		let hooks = self.0.request_save_hooks.read().clone();
		for hook in hooks {
			hook(opts);
		}
	}
}

struct InFlightWrite {
	ctx: ActorContext,
}

impl Drop for InFlightWrite {
	fn drop(&mut self) {
		if self
			.ctx
			.0
			.in_flight_state_writes
			.fetch_sub(1, Ordering::SeqCst)
			== 1
		{
			self.ctx.0.state_write_completion.notify_waiters();
			self.ctx.0.state_write_completion.notify_one();
		}
	}
}

fn throttled_save_delay(
	save_interval: Duration,
	time_since_last_save: Duration,
	max_wait: Option<Duration>,
) -> Duration {
	let save_delay = save_interval.saturating_sub(time_since_last_save);
	if let Some(max_wait) = max_wait {
		save_delay.min(max_wait)
	} else {
		save_delay
	}
}

// Test shim keeps moved tests in crate-root tests/ with private-module access.
#[cfg(test)]
#[path = "../../tests/state.rs"]
mod tests;

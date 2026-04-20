use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tokio::sync::Mutex as AsyncMutex;
use tokio::task::JoinHandle;

use crate::actor::callbacks::StateDelta;
use crate::actor::connection::make_connection_key;
use crate::actor::config::ActorConfig;
use crate::actor::metrics::ActorMetrics;
use crate::actor::persist::{
	decode_with_embedded_version, encode_with_embedded_version,
};
use crate::actor::task::{
	LIFECYCLE_EVENT_INBOX_CHANNEL, LifecycleEvent, actor_channel_overloaded_error,
};
use crate::actor::task_types::StateMutationReason;
use crate::error::ActorLifecycle as ActorLifecycleError;
use crate::kv::Kv;
use crate::types::SaveStateOpts;

pub const PERSIST_DATA_KEY: &[u8] = &[1];
const ACTOR_PERSIST_VERSION: u16 = 4;
const ACTOR_PERSIST_COMPATIBLE_VERSIONS: &[u16] = &[3, 4];

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

#[derive(Clone)]
pub struct ActorState(Arc<ActorStateInner>);

struct ActorStateInner {
	current_state: RwLock<Vec<u8>>,
	persisted: RwLock<PersistedActor>,
	kv: Kv,
	save_interval: Duration,
	dirty: AtomicBool,
	revision: AtomicU64,
	save_request_revision: AtomicU64,
	in_on_state_change: Arc<AtomicBool>,
	save_requested: AtomicBool,
	save_requested_immediate: AtomicBool,
	save_requested_within_deadline: Mutex<Option<Instant>>,
	last_save_at: Mutex<Option<Instant>>,
	pending_save: Mutex<Option<PendingSave>>,
	tracked_persist: Mutex<Option<JoinHandle<()>>>,
	save_guard: AsyncMutex<()>,
	lifecycle_events: RwLock<Option<mpsc::Sender<LifecycleEvent>>>,
	request_save_hooks: RwLock<Vec<Arc<dyn Fn(bool) + Send + Sync>>>,
	lifecycle_event_inbox_capacity: usize,
	metrics: ActorMetrics,
}

struct PendingSave {
	scheduled_at: Instant,
	handle: JoinHandle<()>,
}

impl ActorState {
	pub fn new(kv: Kv, config: ActorConfig) -> Self {
		Self::new_with_metrics(kv, config, ActorMetrics::default())
	}

	pub(crate) fn new_with_metrics(
		kv: Kv,
		config: ActorConfig,
		metrics: ActorMetrics,
	) -> Self {
		Self(Arc::new(ActorStateInner {
			current_state: RwLock::new(Vec::new()),
			persisted: RwLock::new(PersistedActor::default()),
			kv,
			save_interval: config.state_save_interval,
			dirty: AtomicBool::new(false),
			revision: AtomicU64::new(0),
			save_request_revision: AtomicU64::new(0),
			in_on_state_change: Arc::new(AtomicBool::new(false)),
			save_requested: AtomicBool::new(false),
			save_requested_immediate: AtomicBool::new(false),
			save_requested_within_deadline: Mutex::new(None),
			last_save_at: Mutex::new(None),
			pending_save: Mutex::new(None),
			tracked_persist: Mutex::new(None),
			save_guard: AsyncMutex::new(()),
			lifecycle_events: RwLock::new(None),
			request_save_hooks: RwLock::new(Vec::new()),
			lifecycle_event_inbox_capacity: config.lifecycle_event_inbox_capacity,
			metrics,
		}))
	}

	pub fn state(&self) -> Vec<u8> {
		self.0
			.current_state
			.read()
			.expect("actor state lock poisoned")
			.clone()
	}

	pub fn set_state(&self, state: Vec<u8>) -> Result<()> {
		self.mutate_state(StateMutationReason::UserSetState, |current| {
			*current = state;
			Ok(())
		})
	}

	pub fn mutate_state<F>(
		&self,
		reason: StateMutationReason,
		mutate: F,
	) -> Result<()>
	where
		F: FnOnce(&mut Vec<u8>) -> Result<()>,
	{
		if self.in_on_state_change_callback() {
			return Err(ActorLifecycleError::StateMutationReentrant.build());
		}

		let sender = self.lifecycle_event_sender();
		if let Some(sender) = sender {
			let permit = sender.try_reserve().map_err(|_| {
				self.0.metrics.inc_state_mutation_overload(reason);
				actor_channel_overloaded_error(
					LIFECYCLE_EVENT_INBOX_CHANNEL,
					self.0.lifecycle_event_inbox_capacity,
					"state_mutated",
					Some(&self.0.metrics),
				)
			})?;

			self.replace_state(mutate)?;
			self.mark_dirty();
			self.0.metrics.inc_state_mutation(reason);
			permit.send(LifecycleEvent::StateMutated { reason });
			Ok(())
		} else {
			self.replace_state(mutate)?;
			self.mark_dirty();
			self.0.metrics.inc_state_mutation(reason);
			Ok(())
		}
	}

	pub(crate) async fn persist_state(&self, opts: SaveStateOpts) -> Result<()> {
		if !self.is_dirty() {
			return Ok(());
		}

		if opts.immediate {
			self.clear_pending_save();
			self.persist_if_dirty().await
		} else {
			let delay = self.compute_save_delay(None);
			if !delay.is_zero() {
				tokio::time::sleep(delay).await;
			}
			self.persist_if_dirty().await
		}
	}

	pub fn request_save(&self, immediate: bool) {
		self.0.save_request_revision.fetch_add(1, Ordering::SeqCst);
		self.notify_request_save_hooks(immediate);
		let already_requested = self.0.save_requested.swap(true, Ordering::SeqCst);
		let immediate_already_requested = if immediate {
			self
				.0
				.save_requested_immediate
				.swap(true, Ordering::SeqCst)
		} else {
			self.0.save_requested_immediate.load(Ordering::SeqCst)
		};

		let Some(sender) = self.lifecycle_event_sender() else {
			return;
		};

		if already_requested && (!immediate || immediate_already_requested) {
			return;
		}

		match sender.try_reserve() {
			Ok(permit) => {
				permit.send(LifecycleEvent::SaveRequested { immediate });
			}
			Err(_) => {
				let _ = actor_channel_overloaded_error(
					LIFECYCLE_EVENT_INBOX_CHANNEL,
					self.0.lifecycle_event_inbox_capacity,
					"save_requested",
					Some(&self.0.metrics),
				);
			}
		}
	}

	pub fn request_save_within(&self, ms: u32) {
		self.0.save_request_revision.fetch_add(1, Ordering::SeqCst);
		self.notify_request_save_hooks(false);
		self.0.save_requested.store(true, Ordering::SeqCst);

		let deadline = Instant::now() + Duration::from_millis(u64::from(ms));
		let mut requested_deadline = self
			.0
			.save_requested_within_deadline
			.lock()
			.expect("actor state save-within deadline lock poisoned");
		*requested_deadline = Some(match *requested_deadline {
			Some(existing) => existing.min(deadline),
			None => deadline,
		});
		drop(requested_deadline);

		let Some(sender) = self.lifecycle_event_sender() else {
			return;
		};

		match sender.try_reserve() {
			Ok(permit) => {
				permit.send(LifecycleEvent::SaveRequested { immediate: false });
			}
			Err(_) => {
				let _ = actor_channel_overloaded_error(
					LIFECYCLE_EVENT_INBOX_CHANNEL,
					self.0.lifecycle_event_inbox_capacity,
					"save_requested",
					Some(&self.0.metrics),
				);
			}
		}
	}

	pub(crate) fn save_requested(&self) -> bool {
		self.0.save_requested.load(Ordering::SeqCst)
	}

	pub(crate) fn save_requested_immediate(&self) -> bool {
		self
			.0
			.save_requested_immediate
			.load(Ordering::SeqCst)
	}

	pub(crate) fn compute_save_deadline(&self, immediate: bool) -> Instant {
		if immediate || self.save_requested_immediate() {
			return Instant::now();
		}

		let throttled_deadline = Instant::now() + self.compute_save_delay(None);
		let requested_deadline = *self
			.0
			.save_requested_within_deadline
			.lock()
			.expect("actor state save-within deadline lock poisoned");

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
		self.clear_pending_save();

		if deltas.is_empty() {
			self.finish_save_request(save_request_revision);
			return Ok(());
		}

		let _save_guard = self.0.save_guard.lock().await;
		let revision = self.0.revision.load(Ordering::SeqCst);
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
			let encoded = encode_persisted_actor(&persisted)
				.context("encode persisted actor state")?;
			puts.push((PERSIST_DATA_KEY.to_vec(), encoded));
			*self
				.0
				.persisted
				.write()
				.expect("actor persisted state lock poisoned") = persisted;
		}

		self.0
			.kv
			.apply_batch(&puts, &deletes)
			.await
			.context("persist actor state deltas to kv")?;

		if let Some(state) = next_state {
			*self
				.0
				.current_state
				.write()
				.expect("actor state lock poisoned") = state;
		}

		*self
			.0
			.last_save_at
			.lock()
			.expect("actor state save timestamp lock poisoned") = Some(Instant::now());

		if self.0.revision.load(Ordering::SeqCst) == revision {
			self.0.dirty.store(false, Ordering::SeqCst);
		}

		self.finish_save_request(save_request_revision);
		Ok(())
	}

	pub(crate) async fn wait_for_pending_writes(&self) {
		loop {
			if let Some(handle) = self.take_tracked_persist() {
				let _ = handle.await;
				continue;
			}

			let _save_guard = self.0.save_guard.lock().await;
			if self.has_tracked_persist() {
				continue;
			}

			return;
		}
	}

	pub fn persisted(&self) -> PersistedActor {
		self.0
			.persisted
			.read()
			.expect("actor persisted state lock poisoned")
			.clone()
	}

	pub fn load_persisted(&self, persisted: PersistedActor) {
		let state = persisted.state.clone();
		*self
			.0
			.persisted
			.write()
			.expect("actor persisted state lock poisoned") = persisted;
		*self
			.0
			.current_state
			.write()
			.expect("actor state lock poisoned") = state;
		self.0.dirty.store(false, Ordering::SeqCst);
		self.finish_save_request(self.save_request_revision());
		self
			.0
			.metrics
			.inc_state_mutation(StateMutationReason::InternalReplace);
	}

	pub fn scheduled_events(&self) -> Vec<PersistedScheduleEvent> {
		self.0
			.persisted
			.read()
			.expect("actor persisted state lock poisoned")
			.scheduled_events
			.clone()
	}

	pub fn set_scheduled_events(&self, scheduled_events: Vec<PersistedScheduleEvent>) {
		self
			.0
			.persisted
			.write()
			.expect("actor persisted state lock poisoned")
			.scheduled_events = scheduled_events;
		self
			.0
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
			let mut persisted = self
				.0
				.persisted
				.write()
				.expect("actor persisted state lock poisoned");
			update(&mut persisted.scheduled_events)
		};

		self
			.0
			.metrics
			.inc_state_mutation(StateMutationReason::ScheduledEventsUpdate);
		self.mark_dirty();
		self.schedule_save(None);
		result
	}

	pub fn set_input(&self, input: Option<Vec<u8>>) {
		self
			.0
			.persisted
			.write()
			.expect("actor persisted state lock poisoned")
			.input = input;
		self
			.0
			.metrics
			.inc_state_mutation(StateMutationReason::InputSet);
		self.mark_dirty();
		self.schedule_save(None);
	}

	pub fn input(&self) -> Option<Vec<u8>> {
		self.0
			.persisted
			.read()
			.expect("actor persisted state lock poisoned")
			.input
			.clone()
	}

	pub fn set_has_initialized(&self, has_initialized: bool) {
		self
			.0
			.persisted
			.write()
			.expect("actor persisted state lock poisoned")
			.has_initialized = has_initialized;
		self
			.0
			.metrics
			.inc_state_mutation(StateMutationReason::HasInitialized);
		self.mark_dirty();
		self.schedule_save(None);
	}

	pub fn has_initialized(&self) -> bool {
		self.0
			.persisted
			.read()
			.expect("actor persisted state lock poisoned")
			.has_initialized
	}

	pub fn flush_on_shutdown(&self) {
		self.persist_now_tracked("shutdown_flush");
	}

	pub(crate) fn configure_lifecycle_events(
		&self,
		sender: Option<mpsc::Sender<LifecycleEvent>>,
	) {
		*self
			.0
			.lifecycle_events
			.write()
			.expect("actor state lifecycle events lock poisoned") = sender;
	}

	pub(crate) fn on_request_save(
		&self,
		hook: Box<dyn Fn(bool) + Send + Sync>,
	) {
		self
			.0
			.request_save_hooks
			.write()
			.expect("actor state request-save hooks lock poisoned")
			.push(Arc::from(hook));
	}

	pub(crate) fn lifecycle_events_configured(&self) -> bool {
		self
			.0
			.lifecycle_events
			.read()
			.expect("actor state lifecycle events lock poisoned")
			.is_some()
	}

	pub(crate) fn in_on_state_change_callback(&self) -> bool {
		self.0.in_on_state_change.load(Ordering::SeqCst)
	}

	pub(crate) fn in_on_state_change_flag(&self) -> Arc<AtomicBool> {
		self.0.in_on_state_change.clone()
	}

	pub(crate) fn set_in_on_state_change_callback(&self, in_callback: bool) {
		self.0
			.in_on_state_change
			.store(in_callback, Ordering::SeqCst);
	}

	fn is_dirty(&self) -> bool {
		self.0.dirty.load(Ordering::SeqCst)
	}

	fn mark_dirty(&self) {
		self.0.dirty.store(true, Ordering::SeqCst);
		self.0.revision.fetch_add(1, Ordering::SeqCst);
	}

	fn lifecycle_event_sender(&self) -> Option<mpsc::Sender<LifecycleEvent>> {
		self
			.0
			.lifecycle_events
			.read()
			.expect("actor state lifecycle events lock poisoned")
			.clone()
	}

	fn replace_state<F>(&self, mutate: F) -> Result<()>
	where
		F: FnOnce(&mut Vec<u8>) -> Result<()>,
	{
		let next_state = {
			let mut current = self
				.0
				.current_state
				.write()
				.expect("actor state lock poisoned");
			let mut next = current.clone();
			mutate(&mut next)?;
			*current = next.clone();
			next
		};

		self
			.0
			.persisted
			.write()
			.expect("actor persisted state lock poisoned")
			.state = next_state;
		Ok(())
	}

	fn compute_save_delay(&self, max_wait: Option<Duration>) -> Duration {
		let elapsed = self
			.0
			.last_save_at
			.lock()
			.expect("actor state save timestamp lock poisoned")
			.map(|instant| instant.elapsed())
			.unwrap_or_default();

		throttled_save_delay(self.0.save_interval, elapsed, max_wait)
	}

	fn schedule_save(&self, max_wait: Option<Duration>) {
		if !self.is_dirty() {
			return;
		}

		let Ok(tokio_handle) = Handle::try_current() else {
			return;
		};

		let delay = self.compute_save_delay(max_wait);
		let scheduled_at = Instant::now() + delay;

		let mut pending_save = self
			.0
			.pending_save
			.lock()
			.expect("actor pending save lock poisoned");

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
		let handle = tokio_handle.spawn(async move {
			if !delay.is_zero() {
				tokio::time::sleep(delay).await;
			}

			state.take_pending_save();

			if let Err(error) = state.persist_if_dirty().await {
				tracing::error!(?error, "failed to persist actor state");
			}
		});

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
		let mut tracked_persist = self
			.0
			.tracked_persist
			.lock()
			.expect("actor tracked persist lock poisoned");
		let previous = tracked_persist.take();
		let handle = tokio_handle.spawn(async move {
			if let Some(previous) = previous {
				let _ = previous.await;
			}

			if let Err(error) = state
				.persist_state(SaveStateOpts { immediate: true })
				.await
			{
				tracing::error!(?error, description, "failed to persist actor state");
			}
		});
		*tracked_persist = Some(handle);
	}

	fn take_pending_save(&self) -> Option<PendingSave> {
		self.0
			.pending_save
			.lock()
			.expect("actor pending save lock poisoned")
			.take()
	}

	fn take_tracked_persist(&self) -> Option<JoinHandle<()>> {
		self.0
			.tracked_persist
			.lock()
			.expect("actor tracked persist lock poisoned")
			.take()
	}

	fn has_tracked_persist(&self) -> bool {
		self.0
			.tracked_persist
			.lock()
			.expect("actor tracked persist lock poisoned")
			.is_some()
	}

	#[cfg(test)]
	pub(crate) fn tracked_persist_pending(&self) -> bool {
		self.has_tracked_persist()
	}

	async fn persist_if_dirty(&self) -> Result<()> {
		if !self.is_dirty() {
			return Ok(());
		}

		let _save_guard = self.0.save_guard.lock().await;
		if !self.is_dirty() {
			return Ok(());
		}

		let revision = self.0.revision.load(Ordering::SeqCst);
		let persisted = self.persisted();
		let encoded = encode_persisted_actor(&persisted)
			.context("encode persisted actor state")?;

		*self
			.0
			.last_save_at
			.lock()
			.expect("actor state save timestamp lock poisoned") = Some(Instant::now());

		self.0
			.kv
			.put(PERSIST_DATA_KEY, &encoded)
			.await
			.context("persist actor state to kv")?;

		if self.0.revision.load(Ordering::SeqCst) == revision {
			self.0.dirty.store(false, Ordering::SeqCst);
		}

		Ok(())
	}

	fn finish_save_request(&self, save_request_revision: u64) {
		if self.0.save_request_revision.load(Ordering::SeqCst) == save_request_revision {
			self.0.save_requested.store(false, Ordering::SeqCst);
			self
				.0
				.save_requested_immediate
				.store(false, Ordering::SeqCst);
			*self
				.0
				.save_requested_within_deadline
				.lock()
				.expect("actor state save-within deadline lock poisoned") = None;
		}
	}

	fn notify_request_save_hooks(&self, immediate: bool) {
		let hooks = self
			.0
			.request_save_hooks
			.read()
			.expect("actor state request-save hooks lock poisoned")
			.clone();
		for hook in hooks {
			hook(immediate);
		}
	}
}

impl Default for ActorState {
	fn default() -> Self {
		Self::new(Kv::default(), ActorConfig::default())
	}
}

impl std::fmt::Debug for ActorState {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ActorState")
			.field("dirty", &self.is_dirty())
			.field("state_len", &self.state().len())
			.finish()
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

#[cfg(test)]
#[path = "../../tests/modules/state.rs"]
mod tests;

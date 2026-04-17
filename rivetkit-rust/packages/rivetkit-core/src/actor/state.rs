use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::runtime::Handle;
use tokio::sync::Mutex as AsyncMutex;
use tokio::task::JoinHandle;

use crate::actor::config::ActorConfig;
use crate::kv::Kv;
use crate::types::SaveStateOpts;

pub const PERSIST_DATA_KEY: &[u8] = &[1];

pub type StateCallbackFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;
pub type OnStateChangeCallback = Arc<dyn Fn() -> StateCallbackFuture + Send + Sync>;

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

#[derive(Clone)]
pub struct ActorState(Arc<ActorStateInner>);

struct ActorStateInner {
	current_state: RwLock<Vec<u8>>,
	persisted: RwLock<PersistedActor>,
	kv: Kv,
	save_interval: Duration,
	dirty: AtomicBool,
	revision: AtomicU64,
	last_save_at: Mutex<Option<Instant>>,
	pending_save: Mutex<Option<PendingSave>>,
	save_guard: AsyncMutex<()>,
	on_state_change: RwLock<Option<OnStateChangeCallback>>,
	is_in_on_state_change: AtomicBool,
}

struct PendingSave {
	scheduled_at: Instant,
	handle: JoinHandle<()>,
}

impl ActorState {
	pub fn new(kv: Kv, config: ActorConfig) -> Self {
		Self(Arc::new(ActorStateInner {
			current_state: RwLock::new(Vec::new()),
			persisted: RwLock::new(PersistedActor::default()),
			kv,
			save_interval: config.state_save_interval,
			dirty: AtomicBool::new(false),
			revision: AtomicU64::new(0),
			last_save_at: Mutex::new(None),
			pending_save: Mutex::new(None),
			save_guard: AsyncMutex::new(()),
			on_state_change: RwLock::new(None),
			is_in_on_state_change: AtomicBool::new(false),
		}))
	}

	pub fn state(&self) -> Vec<u8> {
		self.0
			.current_state
			.read()
			.expect("actor state lock poisoned")
			.clone()
	}

	pub fn set_state(&self, state: Vec<u8>) {
		*self
			.0
			.current_state
			.write()
			.expect("actor state lock poisoned") = state.clone();
		self
			.0
			.persisted
			.write()
			.expect("actor persisted state lock poisoned")
			.state = state;

		self.mark_dirty();
		self.schedule_save(None);
		self.trigger_on_state_change();
	}

	pub async fn save_state(&self, opts: SaveStateOpts) -> Result<()> {
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
		self.mark_dirty();
		self.schedule_save(None);
	}

	pub fn set_input(&self, input: Option<Vec<u8>>) {
		self
			.0
			.persisted
			.write()
			.expect("actor persisted state lock poisoned")
			.input = input;
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
		self.clear_pending_save();

		if let Ok(runtime) = Handle::try_current() {
			let state = self.clone();
			runtime.spawn(async move {
				if let Err(error) = state
					.save_state(SaveStateOpts { immediate: true })
					.await
				{
					tracing::error!(?error, "failed to flush actor state on shutdown");
				}
			});
		}
	}

	#[allow(dead_code)]
	pub(crate) fn set_on_state_change_callback(
		&self,
		callback: Option<OnStateChangeCallback>,
	) {
		*self
			.0
			.on_state_change
			.write()
			.expect("actor on_state_change lock poisoned") = callback;
	}

	fn is_dirty(&self) -> bool {
		self.0.dirty.load(Ordering::SeqCst)
	}

	fn mark_dirty(&self) {
		self.0.dirty.store(true, Ordering::SeqCst);
		self.0.revision.fetch_add(1, Ordering::SeqCst);
	}

	fn compute_save_delay(&self, max_wait: Option<Duration>) -> Duration {
		let elapsed = self
			.0
			.last_save_at
			.lock()
			.expect("actor state save timestamp lock poisoned")
			.map(|instant| instant.elapsed())
			.unwrap_or(self.0.save_interval);

		throttled_save_delay(self.0.save_interval, elapsed, max_wait)
	}

	fn schedule_save(&self, max_wait: Option<Duration>) {
		if !self.is_dirty() {
			return;
		}

		let Ok(runtime) = Handle::try_current() else {
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
		let handle = runtime.spawn(async move {
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

	fn take_pending_save(&self) -> Option<PendingSave> {
		self.0
			.pending_save
			.lock()
			.expect("actor pending save lock poisoned")
			.take()
	}

	fn trigger_on_state_change(&self) {
		let Some(callback) = self
			.0
			.on_state_change
			.read()
			.expect("actor on_state_change lock poisoned")
			.clone()
		else {
			return;
		};

		if self
			.0
			.is_in_on_state_change
			.swap(true, Ordering::SeqCst)
		{
			return;
		}

		let Ok(runtime) = Handle::try_current() else {
			self
				.0
				.is_in_on_state_change
				.store(false, Ordering::SeqCst);
			return;
		};

		let state = self.clone();
		runtime.spawn(async move {
			if let Err(error) = callback().await {
				tracing::error!(?error, "error in on_state_change callback");
			}

			state
				.0
				.is_in_on_state_change
				.store(false, Ordering::SeqCst);
		});
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
		let encoded = serde_bare::to_vec(&persisted).context("encode persisted actor state")?;

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
mod tests {
	use super::{PersistedActor, PersistedScheduleEvent, throttled_save_delay};
	use std::time::Duration;

	#[test]
	fn persisted_actor_round_trips_with_bare() {
		let actor = PersistedActor {
			input: Some(vec![1, 2, 3]),
			has_initialized: true,
			state: vec![4, 5, 6],
			scheduled_events: vec![PersistedScheduleEvent {
				event_id: "event-1".into(),
				timestamp_ms: 42,
				action: "ping".into(),
				args: vec![7, 8],
			}],
		};

		let encoded = serde_bare::to_vec(&actor).expect("persisted actor should encode");
		let decoded: PersistedActor =
			serde_bare::from_slice(&encoded).expect("persisted actor should decode");

		assert_eq!(decoded, actor);
	}

	#[test]
	fn throttled_save_delay_uses_remaining_interval() {
		let delay = throttled_save_delay(
			Duration::from_secs(1),
			Duration::from_millis(250),
			None,
		);

		assert_eq!(delay, Duration::from_millis(750));
	}

	#[test]
	fn throttled_save_delay_respects_max_wait() {
		let delay = throttled_save_delay(
			Duration::from_secs(1),
			Duration::from_millis(250),
			Some(Duration::from_millis(100)),
		);

		assert_eq!(delay, Duration::from_millis(100));
	}
}

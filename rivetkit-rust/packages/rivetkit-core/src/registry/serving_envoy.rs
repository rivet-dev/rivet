//! The envoy a serverful registry currently serves through, including replacements after a
//! lost connection.

use std::sync::Arc;

use tokio::sync::watch;

use super::CoreEnvoyHandle;

#[derive(Clone)]
enum ServingEnvoyState {
	Pending,
	Serving(CoreEnvoyHandle),
	Closed,
}

#[derive(Clone)]
pub struct ServingEnvoy {
	state: Arc<watch::Sender<ServingEnvoyState>>,
}

impl Default for ServingEnvoy {
	fn default() -> Self {
		Self {
			state: Arc::new(watch::Sender::new(ServingEnvoyState::Pending)),
		}
	}
}

impl ServingEnvoy {
	pub fn attach(&self, handle: CoreEnvoyHandle) {
		self.state.send_replace(ServingEnvoyState::Serving(handle));
	}

	/// Mark that the registry will not serve through an envoy again.
	pub fn close(&self) {
		self.state.send_replace(ServingEnvoyState::Closed);
	}

	pub fn current(&self) -> Option<CoreEnvoyHandle> {
		match &*self.state.borrow() {
			ServingEnvoyState::Serving(handle) => Some(handle.clone()),
			ServingEnvoyState::Pending | ServingEnvoyState::Closed => None,
		}
	}

	/// Zero while no envoy is attached or the attached envoy has stopped.
	pub fn active_actor_count(&self) -> usize {
		match self.current() {
			Some(handle) if !handle.is_stopped() => handle.status().active_actor_count,
			Some(_) | None => 0,
		}
	}

	pub fn active_actor_count_changes(&self) -> ActiveActorCountChanges {
		ActiveActorCountChanges {
			state: self.state.subscribe(),
			last: None,
		}
	}
}

pub struct ActiveActorCountChanges {
	state: watch::Receiver<ServingEnvoyState>,
	last: Option<usize>,
}

impl ActiveActorCountChanges {
	/// Resolve with the next distinct count, starting with the current one. Counts zero while
	/// no envoy is attached or the attached envoy has stopped. Returns `None` once the
	/// registry closes.
	pub async fn next(&mut self) -> Option<usize> {
		loop {
			let state = self.state.borrow_and_update().clone();
			let handle = match state {
				ServingEnvoyState::Pending => {
					if let Some(zero) = self.zero() {
						return Some(zero);
					}
					if self.state.changed().await.is_err() {
						return self.zero();
					}
					continue;
				}
				ServingEnvoyState::Closed => return self.zero(),
				ServingEnvoyState::Serving(handle) => handle,
			};

			tokio::select! {
				count = handle.wait_active_actor_count_change(self.last) => match count {
					Some(count) => {
						self.last = Some(count);
						return Some(count);
					}
					None => {
						if let Some(zero) = self.zero() {
							return Some(zero);
						}
						if self.state.changed().await.is_err() {
							return self.zero();
						}
					}
				},
				changed = self.state.changed() => {
					if changed.is_err() {
						return self.zero();
					}
				}
			}
		}
	}

	/// Yields zero unless zero was the last count yielded.
	fn zero(&mut self) -> Option<usize> {
		if self.last == Some(0) {
			return None;
		}
		self.last = Some(0);
		Some(0)
	}
}

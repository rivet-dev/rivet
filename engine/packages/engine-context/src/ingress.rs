//! Admission and completion ownership for ingress that must finish before storage closes.
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

#[derive(Default)]
struct State {
	closed: bool,
	active: usize,
}

#[derive(Default)]
struct Inner {
	state: Mutex<State>,
	changed: Notify,
}

/// Closing admission and reserving an owner are serialized. An accepted owner can
/// therefore cross an asynchronous upgrade/spawn without a drain observing zero.
#[derive(Clone, Default)]
pub struct IngressDrain(Arc<Inner>);

pub struct IngressGuard(Arc<Inner>);

impl IngressDrain {
	pub fn enter(&self) -> Option<IngressGuard> {
		let mut state = self.0.state.lock().expect("ingress state poisoned");
		if state.closed {
			return None;
		}
		state.active += 1;
		Some(IngressGuard(self.0.clone()))
	}

	/// Stops new owners and waits for every previously admitted owner to finish.
	/// The caller retains the process's original shutdown deadline around this wait.
	pub async fn drain(&self) {
		loop {
			let changed = self.0.changed.notified();
			tokio::pin!(changed);
			changed.as_mut().enable();
			{
				let mut state = self.0.state.lock().expect("ingress state poisoned");
				state.closed = true;
				if state.active == 0 {
					return;
				}
			}
			changed.await;
		}
	}
}

impl Drop for IngressGuard {
	fn drop(&mut self) {
		let done = {
			let mut state = self.0.state.lock().expect("ingress state poisoned");
			state.active -= 1;
			state.closed && state.active == 0
		};
		if done {
			self.0.changed.notify_waiters();
		}
	}
}



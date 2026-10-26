//! Shutdown requests share the first request's boot-time deadline origin.
//! The first two requests ask for graceful shutdown; the third and later requests
//! ask for forced shutdown. Repeated requests never extend the shutdown budget.

use crate::{Clock, ClockError};
use std::time::Duration;
use tokio::sync::watch;

#[derive(Clone, Default)]
enum State {
	/// No shutdown request has been received.
	#[default]
	Running,
	Requested {
		/// Time of the first request. A failed clock read is retained and returned
		/// to subscribers instead of calculating a deadline.
		started_at_boot_ms: Result<u64, ClockError>,
		/// Number of shutdown requests received, including the first.
		request_count: usize,
	},
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShutdownEvent {
	Graceful,
	Force,
}

/// Each subscriber observes the latest shutdown request; requests may coalesce.
/// Cloning replays an existing shutdown to the new subscriber.
pub struct Shutdown {
	/// Shared state retains the original deadline origin for late subscribers.
	state: watch::Receiver<State>,
}

impl Clone for Shutdown {
	fn clone(&self) -> Self {
		let mut state = self.state.clone();
		state.mark_changed();

		Self { state }
	}
}

#[derive(Clone)]
pub struct ShutdownTrigger(watch::Sender<State>);

impl Shutdown {
	pub fn channel() -> (ShutdownTrigger, Self) {
		let (tx, rx) = watch::channel(State::default());

		(ShutdownTrigger(tx), Self { state: rx })
	}

	pub async fn recv(&mut self) -> Result<ShutdownEvent, ClockError> {
		loop {
			if self.state.changed().await.is_err() {
				std::future::pending::<()>().await;
			}

			let state = self.state.borrow_and_update();
			if let State::Requested {
				started_at_boot_ms,
				request_count,
			} = &*state
			{
				started_at_boot_ms.as_ref().map_err(Clone::clone)?;

				return Ok(if *request_count >= 3 {
					ShutdownEvent::Force
				} else {
					ShutdownEvent::Graceful
				});
			}
		}
	}

	pub fn deadline(&self, duration: Duration) -> Result<u64, ClockError> {
		let state = self.state.borrow();
		let State::Requested {
			started_at_boot_ms, ..
		} = &*state
		else {
			return Err(ClockError::Source("shutdown has not started".into()));
		};
		let started_at_boot_ms = started_at_boot_ms.as_ref().map_err(Clone::clone)?;

		let millis = u64::try_from(duration.as_millis()).map_err(|_| ClockError::Overflow)?;
		started_at_boot_ms
			.checked_add(millis)
			.ok_or(ClockError::Overflow)
	}
}

impl ShutdownTrigger {
	pub fn request(&self, clock: &dyn Clock) {
		self.0.send_modify(|state| match state {
			State::Running => {
				*state = State::Requested {
					started_at_boot_ms: clock.observe().map(|time| time.boot_ms),
					request_count: 1,
				};
			}
			State::Requested { request_count, .. } => {
				*request_count = request_count.saturating_add(1);
			}
		});
	}
}

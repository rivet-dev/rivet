use futures_util::FutureExt;
use gas::prelude::*;
use serde::{Deserialize, Serialize};

mod reconcile;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input;

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Wake {
	/// Normal key lifecycle deadline. An already-retained signal bypasses this wait.
	Lifecycle(i64),
	/// Retry delay. Retained signals cannot turn storage or consensus failures into a hot loop.
	Backoff(i64),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct State {
	#[serde(default)]
	wake: Option<Wake>,
	start_rotation: Option<StartRotation>,
	emergency: Option<EmergencyRotate>,
}

#[workflow]
pub async fn auth_jwt_key_rotation(ctx: &mut WorkflowCtx, _input: &Input) -> Result<()> {
	ctx.loope(State::default(), |ctx, state| {
		async move {
			if let Some(deadline) = state.take_wait_deadline()
				&& let Some(signal) = ctx.listen_until::<Main>(deadline).await?
			{
				state.record(signal);
			}

			let outcome = ctx
				.activity(reconcile::Input {
					start_rotation: state.start_rotation.clone(),
					emergency: state.emergency.clone(),
				})
				.await?;

			match outcome {
				reconcile::Output::Committed {
					next_wake_ts,
					consumed,
				} => {
					state.consume(consumed);
					state.wake = Some(Wake::Lifecycle(next_wake_ts));
				}
				reconcile::Output::Stable {
					next_wake_ts,
					consumed,
				} => {
					state.consume(consumed);
					state.wake = Some(Wake::Lifecycle(next_wake_ts));
				}
				reconcile::Output::Retry { after_ts } => {
					state.wake = Some(Wake::Backoff(after_ts));
				}
				reconcile::Output::Blocked { reason, after_ts } => {
					tracing::error!(%reason, "JWT key-ring reconciliation is blocked");
					state.wake = Some(Wake::Backoff(after_ts));
				}
			}

			Ok(Loop::<()>::Continue)
		}
		.boxed()
	})
	.await?;
	Ok(())
}

impl State {
	fn has_trigger(&self) -> bool {
		self.start_rotation.is_some() || self.emergency.is_some()
	}

	fn take_wait_deadline(&mut self) -> Option<i64> {
		match self.wake.take() {
			Some(Wake::Lifecycle(_)) if self.has_trigger() => None,
			Some(Wake::Lifecycle(deadline) | Wake::Backoff(deadline)) => Some(deadline),
			None => None,
		}
	}

	fn record(&mut self, signal: Main) {
		match signal {
			Main::StartRotation(signal) => {
				if self.start_rotation.is_none() {
					self.start_rotation = Some(signal);
				}
			}
			Main::EmergencyRotate(signal) => {
				if self.emergency.is_none() {
					self.emergency = Some(signal);
				}
			}
		}
	}

	fn consume(&mut self, consumed: reconcile::ConsumedSignals) {
		if consumed.start_rotation {
			self.start_rotation = None;
		}
		if consumed.emergency {
			self.emergency = None;
		}
	}
}

#[signal("auth_jwt_start_rotation")]
#[derive(Debug, Clone, Hash)]
pub struct StartRotation {
	pub expected_generation: u64,
}

#[signal("auth_jwt_emergency_rotate")]
#[derive(Debug, Clone, Hash)]
pub struct EmergencyRotate {
	pub request_id: [u8; 16],
	pub expected_generation: u64,
	pub revoke_kids: Vec<[u8; 16]>,
}

join_signal!(Main {
	StartRotation,
	EmergencyRotate,
});

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn lifecycle_wait_is_bypassed_by_a_retained_signal() {
		let mut state = State {
			wake: Some(Wake::Lifecycle(123)),
			start_rotation: Some(StartRotation {
				expected_generation: 1,
			}),
			emergency: None,
		};
		assert_eq!(state.take_wait_deadline(), None);
	}

	#[test]
	fn retry_backoff_is_not_bypassed_by_a_retained_signal() {
		let mut state = State {
			wake: Some(Wake::Backoff(123)),
			start_rotation: Some(StartRotation {
				expected_generation: 1,
			}),
			emergency: None,
		};
		assert_eq!(state.take_wait_deadline(), Some(123));
	}

	#[test]
	fn consuming_one_signal_retains_the_other_for_immediate_reconcile() {
		let mut state = State {
			wake: Some(Wake::Lifecycle(123)),
			start_rotation: Some(StartRotation {
				expected_generation: 1,
			}),
			emergency: Some(EmergencyRotate {
				request_id: [1; 16],
				expected_generation: 1,
				revoke_kids: Vec::new(),
			}),
		};
		state.consume(reconcile::ConsumedSignals {
			start_rotation: false,
			emergency: true,
		});

		assert!(state.start_rotation.is_some());
		assert!(state.emergency.is_none());
		assert_eq!(state.take_wait_deadline(), None);
	}
}

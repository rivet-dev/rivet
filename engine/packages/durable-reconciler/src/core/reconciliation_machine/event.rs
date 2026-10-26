use super::types::Slot;
use rivet_state_machine::OpId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum Body<S> {
	Loaded {
		op: OpId<Slot>,
		result: Result<S, String>,
	},
	Completed {
		op: OpId<Slot>,
		result: Result<(), String>,
	},
	/// **When**
	///
	/// `run_reconciler_with_wake` received a signal that a process-local writer changed
	/// the durable record this reconciler loads.
	///
	/// **State**
	///
	/// While waiting to poll, starts the next pass at once. While a pass is loading or
	/// executing, records that one more pass must follow it. Ignored before start and
	/// after stop.
	///
	/// **Effects**
	///
	/// `Load` and cancelling `Timer::Poll` when it starts a pass; otherwise none.
	Wake,
}

impl<
	S: Clone + std::fmt::Debug + std::hash::Hash + Eq + Serialize + serde::de::DeserializeOwned + Send,
> rivet_state_machine::EventBody for Body<S>
{
	fn name(&self) -> &'static str {
		match self {
			Body::Loaded { .. } => "loaded",
			Body::Completed { .. } => "completed",
			Body::Wake => "wake",
		}
	}
}

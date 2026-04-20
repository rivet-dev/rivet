use rivet_error::*;
use serde::{Deserialize, Serialize};

#[derive(RivetError, Debug, Clone, Deserialize, Serialize)]
#[error("actor")]
pub enum ActorLifecycle {
	#[error("not_ready", "Actor is not ready.")]
	NotReady,

	#[error("stopping", "Actor is stopping.")]
	Stopping,

	#[error("destroying", "Actor is destroying.")]
	Destroying,

	#[error("shutdown_timeout", "Actor shutdown timed out.")]
	ShutdownTimeout,

	#[error("dropped_reply", "Actor reply channel was dropped without a response.")]
	DroppedReply,

	#[error(
		"overloaded",
		"Actor is overloaded.",
		"Actor channel '{channel}' is overloaded while attempting to {operation} (capacity {capacity})."
	)]
	Overloaded {
		channel: String,
		capacity: usize,
		operation: String,
	},

	#[error(
		"state_mutation_reentrant",
		"Actor state mutation is re-entrant."
	)]
	StateMutationReentrant,
}

use rivet_util::Id;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RunnerPoolError {
	/// Serverless: SSE returned non-200 status code (e.g., 404, 500)
	ServerlessHttpError { status_code: u16, body: String },

	/// Serverless: SSE stream ended unexpectedly before runner initialized
	ServerlessStreamEndedEarly,

	/// Serverless: SSE connection or network error
	ServerlessConnectionError { message: String },

	/// Serverless: Runner sent invalid payload
	ServerlessInvalidSsePayload {
		message: String,
		raw_payload: Option<String>,
	},

	/// Internal error
	InternalError,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActorError {
	/// Runner pool-related errors
	RunnerPoolError(RunnerPoolError),
	/// No runners available matching the runner name
	NoCapacity,
	/// Runner was allocated but never started the actor (GC timeout)
	RunnerNoResponse { runner_id: Id },
	/// Runner connection was lost (no recent ping, network issue, or crash)
	RunnerConnectionLost { runner_id: Id },
	/// Runner was draining but actor didn't stop in time
	RunnerDrainingTimeout { runner_id: Id },
	/// Actor exited with an error and is now sleeping
	Crashed { message: Option<String> },
}

use std::time::{SystemTime, UNIX_EPOCH};

use tokio::time::Instant;
use global_error::GlobalError;
use uuid::Uuid;

use crate::ctx::workflow::RETRY_TIMEOUT_MS;

pub type WorkflowResult<T> = Result<T, WorkflowError>;

/// Throwing this will eject from the workflow scope back in to the engine.
///
/// This error should not be touched by the user and is only intended to be handled by the workflow
/// engine.
#[derive(thiserror::Error, Debug)]
pub enum WorkflowError {
	#[error("workflow failure: {0:?}")]
	WorkflowFailure(GlobalError),

	// Includes error count
	#[error("activity failure: {0:?}")]
	ActivityFailure(GlobalError, u32),

	#[error("activity failure, max retries reached: {0:?}")]
	ActivityMaxFailuresReached(GlobalError),

	#[error("operation failure: {0:?}")]
	OperationFailure(GlobalError),

	#[error("workflow missing from registry: {0}")]
	WorkflowMissingFromRegistry(String),

	#[error("workflow not found")]
	WorkflowNotFound,

	#[error("history diverged: {0}")]
	HistoryDiverged(String),

	#[error("serialize workflow input: {0}")]
	SerializeWorkflowInput(serde_json::Error),

	#[error("deserialize workflow input: {0}")]
	DeserializeWorkflowInput(serde_json::Error),

	#[error("serialize activity output: {0}")]
	SerializeWorkflowOutput(serde_json::Error),

	#[error("deserialize workflow input: {0}")]
	DeserializeWorkflowOutput(serde_json::Error),

	#[error("serialize activity input: {0}")]
	SerializeActivityInput(serde_json::Error),

	#[error("serialize activity output: {0}")]
	SerializeActivityOutput(serde_json::Error),

	#[error("deserialize activity output: {0}")]
	DeserializeActivityOutput(serde_json::Error),

	#[error("serialize signal body: {0}")]
	SerializeSignalBody(serde_json::Error),

	#[error("deserialize signal body: {0}")]
	DeserializeSignalBody(serde_json::Error),

	#[error("serialize message body: {0}")]
	SerializeMessageBody(serde_json::Error),

	#[error("serialize message: {0}")]
	SerializeMessage(serde_json::Error),

	#[error("decode message body: {0}")]
	DeserializeMessageBody(serde_json::Error),

	#[error("decode message: {0}")]
	DeserializeMessage(serde_json::Error),

	#[error("serialize message tags: {0:?}")]
	SerializeMessageTags(cjson::Error),

	#[error("create subscription: {0}")]
	CreateSubscription(rivet_pools::prelude::nats::Error),

	#[error("flush nats: {0}")]
	FlushNats(rivet_pools::prelude::nats::Error),

	#[error("subscription unsubscribed")]
	SubscriptionUnsubscribed,

	#[error("missing message data")]
	MissingMessageData,

	#[error("redis: {source}")]
	Redis {
		#[from]
		source: rivet_pools::prelude::redis::RedisError,
	},

	#[error("no signal found: {0:?}")]
	NoSignalFound(Box<[&'static str]>),

	#[error("sub workflow incomplete: {0:?}")]
	SubWorkflowIncomplete(Uuid),

	#[error("integer conversion failed")]
	IntegerConversion,

	#[error("build sql pool: {0}")]
	BuildSqlx(sqlx::Error),

	#[error("sql: {0}")]
	Sqlx(sqlx::Error),

	#[error("max sql retries")]
	MaxSqlRetries,

	#[error("pools: {0}")]
	Pools(#[from] rivet_pools::Error),

	#[error("activity timed out")]
	ActivityTimeout,

	#[error("operation timed out")]
	OperationTimeout,
}

impl WorkflowError {
	pub fn backoff(&self) -> Option<i64> {
		if let WorkflowError::ActivityFailure(_, error_count) = self {
			// NOTE: Max retry is handled in `WorkflowCtx::activity`
			let mut backoff =
				rivet_util::Backoff::new_at(8, None, RETRY_TIMEOUT_MS, 500, *error_count as usize);
			let next = backoff.step().expect("should not have max retry");

			// Calculate timestamp based on the backoff
			let duration_until = next.duration_since(Instant::now());
			let deadline_ts = (SystemTime::now() + duration_until)
			.duration_since(UNIX_EPOCH)
			.unwrap_or_else(|err| unreachable!("time is broken: {}", err))
			.as_millis()
			.try_into()
			.expect("doesn't fit in i64");

			Some(deadline_ts)
		} else {
			None
		}
	}
	
	pub fn is_recoverable(&self) -> bool {
		match self {
			WorkflowError::ActivityFailure(_, _) => true,
			WorkflowError::ActivityTimeout => true,
			WorkflowError::OperationTimeout => true,
			_ => false,
		}
	}

	pub(crate) fn signals(&self) -> &[&'static str] {
		if let WorkflowError::NoSignalFound(signals) = self {
			signals
		} else {
			&[]
		}
	}

	pub(crate) fn sub_workflow(&self) -> Option<Uuid> {
		if let WorkflowError::SubWorkflowIncomplete(sub_workflow_id) = self {
			Some(*sub_workflow_id)
		} else {
			None
		}
	}
}

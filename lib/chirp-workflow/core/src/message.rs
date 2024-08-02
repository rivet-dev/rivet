use std::fmt::Debug;

use rivet_operation::prelude::proto::chirp;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{WorkflowError, WorkflowResult};

pub trait Message: Debug + Send + Sync + Serialize + DeserializeOwned + 'static {
	const NAME: &'static str;
	const TAIL_TTL: std::time::Duration;
}

pub fn serialize_message_nats_subject<M>(tags_str: &str) -> String
where
	M: Message,
{
	format!("chirp.workflow.msg.{}.{}", M::NAME, tags_str,)
}

/// A message received from a Chirp subscription.
#[derive(Debug)]
pub struct ReceivedMessage<M>
where
	M: Message,
{
	pub(crate) ray_id: Uuid,
	pub(crate) req_id: Uuid,
	pub(crate) ts: i64,
	pub(crate) trace: Vec<TraceEntry>,
	pub(crate) body: M,
}

impl<M> ReceivedMessage<M>
where
	M: Message,
{
	#[tracing::instrument(skip(buf))]
	pub(crate) fn deserialize(buf: &[u8]) -> WorkflowResult<Self> {
		// Deserialize the wrapper
		let message_wrapper = Self::deserialize_wrapper(buf)?;

		// Deserialize the body
		let body = serde_json::from_str::<M>(message_wrapper.body.get())
			.map_err(WorkflowError::DeserializeMessageBody)?;

		Ok(ReceivedMessage {
			ray_id: message_wrapper.ray_id,
			req_id: message_wrapper.req_id,
			ts: message_wrapper.ts,
			trace: message_wrapper.trace,
			body,
		})
	}

	// Only returns the message wrapper
	#[tracing::instrument(skip(buf))]
	pub(crate) fn deserialize_wrapper<'a>(buf: &'a [u8]) -> WorkflowResult<MessageWrapper<'a>> {
		serde_json::from_slice(buf).map_err(WorkflowError::DeserializeMessage)
	}
}

impl<M> std::ops::Deref for ReceivedMessage<M>
where
	M: Message,
{
	type Target = M;

	fn deref(&self) -> &Self::Target {
		&self.body
	}
}

impl<M> ReceivedMessage<M>
where
	M: Message,
{
	pub fn ray_id(&self) -> Uuid {
		self.ray_id
	}

	pub fn req_id(&self) -> Uuid {
		self.req_id
	}

	/// Timestamp at which the message was created.
	pub fn msg_ts(&self) -> i64 {
		self.ts
	}

	pub fn body(&self) -> &M {
		&self.body
	}

	pub fn trace(&self) -> &[TraceEntry] {
		&self.trace
	}
}

#[derive(Serialize, Deserialize)]
pub(crate) struct MessageWrapper<'a> {
	pub(crate) ray_id: Uuid,
	pub(crate) req_id: Uuid,
	pub(crate) tags: serde_json::Value,
	pub(crate) ts: i64,
	pub(crate) trace: Vec<TraceEntry>,
	#[serde(borrow)]
	pub(crate) body: &'a serde_json::value::RawValue,
	pub(crate) allow_recursive: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceEntry {
	context_name: String,
	pub(crate) req_id: Uuid,
	ts: i64,
}

impl TryFrom<chirp::TraceEntry> for TraceEntry {
	type Error = WorkflowError;

	fn try_from(value: chirp::TraceEntry) -> WorkflowResult<Self> {
		Ok(TraceEntry {
			context_name: value.context_name.clone(),
			req_id: value
				.req_id
				.map(|id| id.as_uuid())
				.ok_or(WorkflowError::MissingMessageData)?,
			ts: value.ts,
		})
	}
}

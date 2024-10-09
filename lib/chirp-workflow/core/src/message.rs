use std::fmt::Debug;

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{WorkflowError, WorkflowResult};

pub const WORKER_WAKE_SUBJECT: &str = "chirp.workflow.worker.wake";

pub trait Message: Debug + Send + Sync + Serialize + DeserializeOwned + 'static {
	const NAME: &'static str;
	const TAIL_TTL: std::time::Duration;

	fn nats_subject() -> String {
		format!("chirp.workflow.msg.{}", Self::NAME)
	}
}

/// A message received from a NATS subscription.
#[derive(Debug)]
pub struct NatsMessage<M>
where
	M: Message,
{
	pub(crate) ray_id: Uuid,
	pub(crate) req_id: Uuid,
	pub(crate) ts: i64,
	pub(crate) body: M,
}

impl<M> NatsMessage<M>
where
	M: Message,
{
	#[tracing::instrument(skip(buf))]
	pub(crate) fn deserialize(buf: &[u8]) -> WorkflowResult<Self> {
		let message_wrapper = Self::deserialize_wrapper(buf)?;

		Self::deserialize_from_wrapper(message_wrapper)
	}

	#[tracing::instrument(skip(wrapper))]
	pub(crate) fn deserialize_from_wrapper(
		wrapper: NatsMessageWrapper<'_>,
	) -> WorkflowResult<Self> {
		// Deserialize the body
		let body = serde_json::from_str(wrapper.body.get())
			.map_err(WorkflowError::DeserializeMessageBody)?;

		Ok(NatsMessage {
			ray_id: wrapper.ray_id,
			req_id: wrapper.req_id,
			ts: wrapper.ts,
			body,
		})
	}

	// Only returns the message wrapper
	#[tracing::instrument(skip(buf))]
	pub(crate) fn deserialize_wrapper<'a>(buf: &'a [u8]) -> WorkflowResult<NatsMessageWrapper<'a>> {
		serde_json::from_slice(buf).map_err(WorkflowError::DeserializeMessage)
	}
}

impl<M> std::ops::Deref for NatsMessage<M>
where
	M: Message,
{
	type Target = M;

	fn deref(&self) -> &Self::Target {
		&self.body
	}
}

impl<M> NatsMessage<M>
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
}

#[derive(Serialize, Deserialize)]
pub(crate) struct NatsMessageWrapper<'a> {
	pub(crate) ray_id: Uuid,
	pub(crate) req_id: Uuid,
	pub(crate) tags: serde_json::Value,
	pub(crate) ts: i64,
	#[serde(borrow)]
	pub(crate) body: &'a serde_json::value::RawValue,
	pub(crate) allow_recursive: bool,
}

pub mod redis_keys {
	use std::{
		collections::hash_map::DefaultHasher,
		hash::{Hash, Hasher},
	};

	use super::Message;

	/// HASH
	pub fn message_tail<M>(tags_str: &str) -> String
	where
		M: Message,
	{
		// Get hash of the tags
		let mut hasher = DefaultHasher::new();
		tags_str.hash(&mut hasher);

		format!("{{topic:{}:{:x}}}:tail", M::NAME, hasher.finish())
	}

	pub mod message_tail {
		pub const REQUEST_ID: &str = "r";
		pub const TS: &str = "t";
		pub const BODY: &str = "b";
	}
}

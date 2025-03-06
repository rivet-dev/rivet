use std::fmt::{Debug, Display};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{WorkflowError, WorkflowResult};

pub trait Message: Debug + Send + Sync + Serialize + DeserializeOwned + 'static {
	const NAME: &'static str;
	const TAIL_TTL: std::time::Duration;

	fn nats_subject() -> String {
		format!("chirp.workflow.msg.{}", Self::NAME)
	}
}

pub trait AsTags: Send + Sync {
	fn as_tags(&self) -> WorkflowResult<serde_json::Value>;
	fn as_cjson_tags(&self) -> WorkflowResult<String>;
}

impl<T: Display + Send + Sync, U: Serialize + Send + Sync> AsTags for (T, U) {
	fn as_tags(&self) -> WorkflowResult<serde_json::Value> {
		let (k, v) = self;
		Ok(serde_json::Value::Object(
			IntoIterator::into_iter([(
				k.to_string(),
				serde_json::to_value(v).map_err(WorkflowError::SerializeTags)?,
			)])
			.collect(),
		))
	}

	fn as_cjson_tags(&self) -> WorkflowResult<String> {
		cjson::to_string(&self.as_tags()?).map_err(WorkflowError::CjsonSerializeTags)
	}
}

impl AsTags for serde_json::Value {
	fn as_tags(&self) -> WorkflowResult<serde_json::Value> {
		match self {
			serde_json::Value::Object(_) => Ok(self.clone()),
			_ => Err(WorkflowError::InvalidTags("must be an object".to_string())),
		}
	}

	fn as_cjson_tags(&self) -> WorkflowResult<String> {
		match self {
			serde_json::Value::Object(_) => {
				cjson::to_string(&self).map_err(WorkflowError::CjsonSerializeTags)
			}
			_ => Err(WorkflowError::InvalidTags("must be an object".to_string())),
		}
	}
}

impl<T: AsTags> AsTags for &T {
	fn as_tags(&self) -> WorkflowResult<serde_json::Value> {
		(*self).as_tags()
	}

	fn as_cjson_tags(&self) -> WorkflowResult<String> {
		(*self).as_cjson_tags()
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

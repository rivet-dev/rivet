use rivet_connection::Connection;
use rivet_operation::OperationContext;
use rivet_pools::prelude::*;
use std::fmt::{self, Debug};
use uuid::Uuid;

#[derive(Debug)]
pub(crate) struct RedisMessageMeta {
	/// Key of the topic's stream that the message is from.
	pub topic_key: String,

	/// Group used to consume the message.
	pub group: String,

	/// ID of the message.
	pub id: String,

	/// Parameters for the message.
	///
	/// Only provided if decoding succeeds.
	pub parameters: Option<Vec<String>>,
}

/// Used internally in the manager to resolve the request's state.
pub struct Request<B>
where
	B: Debug + Clone,
{
	pub(crate) conn: Connection,

	pub(crate) nats_message: Option<nats::Message>,
	pub(crate) redis_message_meta: Option<RedisMessageMeta>,
	pub(crate) req_id: Uuid,
	pub(crate) ray_id: Uuid,
	pub(crate) ts: i64,
	pub(crate) req_ts: i64,
	pub(crate) op_ctx: OperationContext<B>,
	pub(crate) dont_log_body: bool,
}

impl<B> Request<B>
where
	B: Debug + Clone,
{
	pub fn req_ts(&self) -> i64 {
		self.req_ts
	}

	pub fn chirp(&self) -> &chirp_client::Client {
		self.conn.chirp()
	}

	pub fn op_ctx(&self) -> &OperationContext<B> {
		&self.op_ctx
	}
}

impl<B> std::ops::Deref for Request<B>
where
	B: Debug + Clone,
{
	type Target = B;

	fn deref(&self) -> &Self::Target {
		self.op_ctx.deref()
	}
}

impl<B> Debug for Request<B>
where
	B: Debug + Clone,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Request")
			.field("req_id", &self.req_id)
			.field("ray_id", &self.ray_id)
			.field("trace", &self.op_ctx.trace())
			.field("ts", &self.ts)
			.finish()
	}
}

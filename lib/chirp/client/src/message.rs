use std::convert::{TryFrom, TryInto};

use chirp_types::message::Message;
use std::fmt::Debug;
use uuid::Uuid;

use crate::error::ClientError;

/// The bucket size in ms for messages written to the Scylla log.
///
/// The bucket size is intentionally very small in order to ensure that we do
/// not have any hot partitions. This is optimized for a write-heavy workload,
/// but we can read these later if needed.
pub const MESSAGE_LOG_BUCKET_MS: i64 = rivet_util::duration::hours(1);

/// Serialize message parameters in to a single string. This is usually
/// appended with another string.
pub fn serialize_message_params(parameters: &[impl AsRef<str>], join: &str) -> String {
	parameters
		.iter()
		.map(|x| x.as_ref())
		.map(|x| {
			if x == "*" {
				x.to_string()
			} else {
				urlencoding::encode(x).to_string()
			}
		})
		.collect::<Vec<String>>()
		.join(join)
}

pub fn serialize_message_nats_subject<M, S>(parameters: &[S]) -> String
where
	M: Message,
	S: AsRef<str>,
{
	format!(
		"chirp.msg.{}.{}",
		M::NAME,
		serialize_message_params(parameters, ".")
	)
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
	pub(crate) fn decode(buf: &[u8]) -> Result<Self, ClientError> {
		// Decode the message
		let (message, trace) = ReceivedMessage::<M>::decode_inner(buf)?;

		let (ray_id, req_id) =
			if let (Some(ray_id), Some(req_id)) = (message.ray_id, message.req_id) {
				(ray_id.as_uuid(), req_id.as_uuid())
			} else {
				return Err(ClientError::MissingMessageData);
			};

		// Decode the body
		let body = M::decode(message.body.as_slice()).map_err(ClientError::DecodeMessageBody)?;

		Ok(ReceivedMessage {
			ray_id,
			req_id,
			ts: message.ts,
			trace: trace,
			body,
		})
	}

	// Only returns the message and trace stack, does not decode anything else
	#[tracing::instrument(skip(buf))]
	pub(crate) fn decode_inner(
		buf: &[u8],
	) -> Result<(types::rivet::chirp::Message, Vec<TraceEntry>), ClientError> {
		// Decode the message and trace
		let message = <types::rivet::chirp::Message as prost::Message>::decode(buf)
			.map_err(ClientError::DecodeMessage)?;

		let trace = message
			.trace
			.clone()
			.into_iter()
			.map(TryInto::try_into)
			.collect::<Result<Vec<_>, ClientError>>()?;

		Ok((message, trace))
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

#[derive(Debug)]
pub struct TraceEntry {
	svc_name: String,
	req_id: Uuid,
	_ts: i64,
}

impl TraceEntry {
	pub fn svc_name(&self) -> &str {
		&self.svc_name
	}

	pub fn req_id(&self) -> Uuid {
		self.req_id
	}
}

impl TryFrom<types::rivet::chirp::TraceEntry> for TraceEntry {
	type Error = ClientError;

	fn try_from(value: types::rivet::chirp::TraceEntry) -> Result<Self, ClientError> {
		Ok(TraceEntry {
			svc_name: value.svc_name.clone(),
			req_id: value
				.req_id
				.map(|id| id.as_uuid())
				.ok_or(ClientError::MissingMessageData)?,
			_ts: value.ts,
		})
	}
}

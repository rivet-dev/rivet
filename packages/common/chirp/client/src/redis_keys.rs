use chirp_types::message::Message;

use crate::message;

/// STREAM
pub fn message_topic(name: &str) -> String {
	format!("{{topic:{name}}}:topic")
}

/// HASH
pub fn message_tail<M, S>(parameters: &[S]) -> String
where
	M: Message,
	S: AsRef<str>,
{
	format!(
		"{{topic:{}:{}}}:tail",
		M::NAME,
		message::serialize_message_params(parameters, ":")
	)
}

pub mod message_tail {
	pub const REQUEST_ID: &str = "r";
	pub const TS: &str = "t";
	pub const BODY: &str = "b";
}

/// ZSET<ts, body>
pub fn message_history<M, S>(parameters: &[S]) -> String
where
	M: Message,
	S: AsRef<str>,
{
	format!(
		"{{topic:{}:{}}}:history",
		M::NAME,
		message::serialize_message_params(parameters, ":")
	)
}

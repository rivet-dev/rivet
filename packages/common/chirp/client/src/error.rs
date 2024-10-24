use rivet_pools::prelude::*;

/// Represents errors from the worker client.
#[derive(thiserror::Error, Debug)]
pub enum ClientError {
	#[error("missing environment variable: {0}")]
	MissingEnvVar(String),

	#[error("join error: {0}")]
	JoinError(#[from] tokio::task::JoinError),

	#[error("tokio spawn: {0}")]
	TokioSpawn(std::io::Error),

	#[error("pools: {0}")]
	Pools(#[from] rivet_pools::Error),

	#[error("flush nats: {0}")]
	FlushNats(nats::Error),

	#[error("nats response status: {0}")]
	NatsResponseStatus(nats::StatusCode),

	#[error("encode message: {0}")]
	EncodeRequest(prost::EncodeError),

	#[error("encode request body: {0}")]
	EncodeRequestBody(prost::EncodeError),

	#[error("encode message body: {0}")]
	EncodeMessageBody(prost::EncodeError),

	#[error("encode message: {0}")]
	EncodeMessage(prost::EncodeError),

	#[error("decode response body: {0}")]
	DecodeResponseBody(prost::DecodeError),

	#[error("decode response: {0}")]
	DecodeResponse(prost::DecodeError),

	#[error("decode message body: {0}")]
	DecodeMessageBody(prost::DecodeError),

	#[error("decode message: {0}")]
	DecodeMessage(prost::DecodeError),

	#[error("publish request: {0}")]
	PublishRequest(nats::PublishError),

	#[error("create subscription: {0}")]
	CreateSubscription(nats::Error),

	#[error("subscription unsubscribed")]
	SubscriptionUnsubscribed,

	#[error("rpc ack timed out")]
	RpcAckTimedOut,

	#[error("rpc timed out")]
	RpcTimedOut,

	#[error("rpc subscription unsubscribed")]
	RpcSubscriptionUnsubscribed,

	#[error("missing message data")]
	MissingMessageData,

	#[error("rpc response: {0}")]
	GlobalError(global_error::GlobalError),

	#[error("parse int error: {source}")]
	ParseIntError {
		#[from]
		source: std::num::ParseIntError,
	},

	#[error("system time error: {source}")]
	SystemTimeError {
		#[from]
		source: std::time::SystemTimeError,
	},

	#[error("malformed response")]
	MalformedResponse,

	#[error("mismatched message parameter count")]
	MismatchedMessageParameterCount,

	#[error("cannot tail message without tail ttl: {name}")]
	CannotTailMessage { name: &'static str },

	#[error("redis: {source}")]
	Redis {
		#[from]
		source: redis::RedisError,
	},
}

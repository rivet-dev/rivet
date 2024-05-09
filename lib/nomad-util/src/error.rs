use std::fmt::Debug;

use rivet_pools::prelude::*;

#[derive(Debug, thiserror::Error)]
pub enum NomadError {
	#[error("missing environment variable: {0}")]
	MissingEnvVar(String),

	#[error("missing job id")]
	MissingJobId,

	#[error("request error: {message}")]
	RequestError { message: String },

	#[error("reqwest: {source}")]
	Reqwest {
		#[from]
		source: reqwest::Error,
	},

	#[error("serde: {source}")]
	Serde {
		#[from]
		source: serde_json::Error,
	},

	#[error("unwrap null api response")]
	UnwrapApiNull,

	#[error("invalid eval status: {0}")]
	EvalStatus(String),

	#[error("too many eval allocs")]
	TooManyEvalAllocs,

	#[error("stream response status: {status}")]
	StreamResponseStatus { status: u16 },

	#[error("event stream closed")]
	EventStreamClosed,

	#[error("redis: {source}")]
	Redis {
		#[from]
		source: redis::RedisError,
	},

	#[error("base64 decode: {source}")]
	Base64Decode {
		#[from]
		source: base64::DecodeError,
	},
}

impl<T> From<nomad_client::apis::Error<T>> for NomadError
where
	T: Debug,
{
	fn from(err: nomad_client::apis::Error<T>) -> Self {
		NomadError::RequestError {
			message: format!("{}", err),
		}
	}
}

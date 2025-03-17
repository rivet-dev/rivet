use global_error::{GlobalError, GlobalResult};

pub type GetterResult<T> = GlobalResult<T>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("missing environment variable: {0}")]
	MissingEnvVar(String),

	#[error("config: {0}")]
	Config(GlobalError),

	#[error("pools: {0}")]
	Pools(#[from] rivet_pools::Error),

	#[error("connect redis: {0}")]
	ConnectRedis(redis::RedisError),

	#[error("redis decode: {0}")]
	RedisDecode(redis::RedisError),

	#[error("getter: {0}")]
	Getter(GlobalError),

	#[error("proto decode: {0}")]
	ProtoDecode(prost::DecodeError),

	#[error("proto encode: {0}")]
	ProtoEncode(prost::EncodeError),

	#[error("serde decode: {0}")]
	SerdeDecode(serde_json::Error),

	#[error("serde encode: {0}")]
	SerdeEncode(serde_json::Error),

	#[error("serialize error: {0}")]
	SerializeError(String),

	#[error("optimistic lock failed too many times")]
	OptimisticLockFailedTooManyTimes,
}

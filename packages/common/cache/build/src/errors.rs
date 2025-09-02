#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("missing environment variable: {0}")]
	MissingEnvVar(String),

	#[error("config error: {0}")]
	Config(anyhow::Error),

	#[error("pools: {0}")]
	Pools(#[from] rivet_pools::Error),

	#[error("getter: {0}")]
	Getter(anyhow::Error),

	#[error("serde decode: {0}")]
	SerdeDecode(serde_json::Error),

	#[error("serde encode: {0}")]
	SerdeEncode(serde_json::Error),

	#[error("optimistic lock failed too many times")]
	OptimisticLockFailedTooManyTimes,
}

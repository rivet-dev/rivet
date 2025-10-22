use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
	#[error("failed to send event to ClickHouse inserter")]
	ChannelSendError,

	#[error("serialization error: {0}")]
	SerializationError(#[source] serde_json::Error),

	#[error("failed to build reqwest client: {0}")]
	ReqwestBuildError(#[source] reqwest::Error),

	#[error("failed to spawn background task")]
	TaskSpawnError,
}

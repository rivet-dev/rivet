#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("missing ups pool")]
	MissingUpsPool,

	#[error("missing clickhouse pool")]
	MissingClickHousePool,

	#[error("missing kafka pool")]
	MissingKafkaPool,

	#[error("missing udb pool")]
	MissingUdbPool,

	#[error("tokio join: {0}")]
	TokioJoin(#[source] tokio::task::JoinError),

	#[error("tokio spawn: {0}")]
	TokioSpawn(#[source] std::io::Error),

	#[error("build nats: {0}")]
	BuildNats(#[source] anyhow::Error),

	#[error("build nats (io): {0}")]
	BuildNatsIo(#[source] std::io::Error),

	#[error("build udb: {0:?}")]
	BuildUdb(#[source] anyhow::Error),

	#[error("build udb connection file: {0}")]
	BuildUdbConnectionFile(#[source] std::io::Error),

	#[error("build clickhouse: {0}")]
	BuildClickHouse(#[source] clickhouse::error::Error),

	#[error("build clickhouse url: {0}")]
	BuildClickHouseUrl(#[source] url::ParseError),

	#[error("io error: {0}")]
	Io(#[source] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("missing nats pool")]
	MissingNatsPool,

	#[error("missing crdb pool")]
	MissingCrdbPool,

	#[error("missing redis pool: {key:?}")]
	MissingRedisPool { key: Option<String> },

	#[error("missing clickhouse pool")]
	MissingClickHousePool,

	#[error("missing clickhouse inserter")]
	MissingClickHouseInserter,

	#[error("missing fdb pool")]
	MissingFdbPool,

	#[error("tokio join: {0}")]
	TokioJoin(tokio::task::JoinError),

	#[error("tokio spawn: {0}")]
	TokioSpawn(std::io::Error),

	#[error("build nats: {0}")]
	BuildNats(async_nats::ConnectError),

	#[error("build nats (io): {0}")]
	BuildNatsIo(std::io::Error),

	#[error("build nats (addresses): {0}")]
	BuildNatsAddresses(anyhow::Error),

	#[error("build redis: {0}")]
	BuildRedis(redis::RedisError),

	#[error("build redis url: {0}")]
	BuildRedisUrl(url::ParseError),

	#[error("build fdb: {0:?}")]
	BuildFdb(anyhow::Error),

	#[error("build fdb connection file: {0}")]
	BuildFdbConnectionFile(std::io::Error),

	#[error("modify redis url")]
	ModifyRedisUrl,

	#[error("redis initial connection timeout")]
	RedisInitialConnectionTimeout,

	#[error("build sqlx: {0}")]
	BuildSqlx(sqlx::Error),

	#[error("build clickhouse: {0}")]
	BuildClickHouse(clickhouse::error::Error),

	#[error("build clickhouse url: {0}")]
	BuildClickHouseUrl(url::ParseError),

	#[error("build clickhouse inserter: {0}")]
	BuildClickHouseInserter(global_error::GlobalError),

	#[error("io error: {0}")]
	Io(std::io::Error),

	#[error("{0}")]
	Global(global_error::GlobalError),

	#[error("lz4: {0}")]
	Lz4(lz4_flex::frame::Error),
}

impl From<global_error::GlobalError> for Error {
    fn from(err: global_error::GlobalError) -> Self {
        Error::Global(err)
    }
}

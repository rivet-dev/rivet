#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("missing nats pool")]
	MissingNatsPool,

	#[error("missing crdb pool: {key:?}")]
	MissingCrdbPool { key: Option<String> },

	#[error("missing redis pool: {key:?}")]
	MissingRedisPool { key: Option<String> },

	#[error("tokio join: {0}")]
	TokioJoin(tokio::task::JoinError),

	#[error("tokio spawn: {0}")]
	TokioSpawn(std::io::Error),

	#[error("build nats: {0}")]
	BuildNats(async_nats::ConnectError),

	#[error("build nats (io): {0}")]
	BuildNatsIo(std::io::Error),

	#[error("build redis: {0}")]
	BuildRedis(redis::RedisError),

	#[error("build sqlx: {0}")]
	BuildSqlx(sqlx::Error),
}

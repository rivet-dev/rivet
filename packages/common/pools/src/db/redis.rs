use redis::aio::ConnectionManager;
use rivet_config::Config;
use std::collections::HashMap;
use tokio::task::JoinSet;
use tokio::time::Duration;

use crate::Error;

pub type RedisPool = ConnectionManager;

/// Connection timeout for the first connection.
///
/// This does not affect reconnection attempts.
const INITIAL_CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);

#[tracing::instrument(skip(config))]
pub async fn setup(config: Config) -> Result<HashMap<String, RedisPool>, Error> {
	// Create Redis connections
	let mut join_set: JoinSet<Result<(&'static str, ConnectionManager), Error>> = JoinSet::new();
	let redis_types = &config.server().map_err(Error::Global)?.redis;
	for (key, redis_config) in [
		("ephemeral", redis_types.ephemeral.clone()),
		("persistent", redis_types.persistent.clone()),
	] {
		let mut url = redis_config.url.clone();
		if let Some(username) = &redis_config.username {
			url.set_username(username)
				.map_err(|()| Error::ModifyRedisUrl)?;
		}
		if let Some(password) = &redis_config.password {
			url.set_password(Some(password.read()))
				.map_err(|()| Error::ModifyRedisUrl)?;
		}

		join_set
			.build_task()
			.name("redis_from_env")
			.spawn(async move {
				tracing::debug!("redis connecting");
				let client = redis::Client::open(url).map_err(Error::BuildRedis)?;

				// Add timeout for initial connection
				let conn = tokio::time::timeout(
					INITIAL_CONNECTION_TIMEOUT,
					redis::aio::ConnectionManager::new_with_backoff(client, 2, 100, usize::MAX),
				)
				.await
				.map_err(|_| Error::RedisInitialConnectionTimeout)?
				.map_err(Error::BuildRedis)?;

				tracing::debug!("redis connected");

				Ok((key, conn))
			})
			.map_err(Error::TokioSpawn)?;
	}

	// Join connections
	let mut redis = HashMap::new();
	while let Some(res) = join_set.join_next().await {
		let (key, conn) = res.map_err(Error::TokioJoin)??;
		redis.insert(key.to_string(), conn.clone());
	}

	tracing::debug!("redis connected");

	Ok(redis)
}
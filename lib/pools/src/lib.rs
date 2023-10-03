mod error;
mod metrics;
mod pools;
pub mod utils;

use rand::prelude::SliceRandom;
use std::{collections::HashMap, env, fmt::Debug, str::FromStr, sync::Arc, time::Duration};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::pools::{CrdbPool, NatsPool, PoolsInner, RedisPool};

pub mod prelude {
	pub use async_nats as nats;
	pub use redis;
	pub use sqlx;

	pub use crate::pools::{CrdbPool, NatsPool, RedisPool};
}

pub use crate::{error::Error, pools::Pools};

// TODO: implement this as a once so we don't do this multiple times
#[tracing::instrument]
pub async fn from_env(client_name: impl ToString + Debug) -> Result<Pools, Error> {
	let client_name = client_name.to_string();
	let token = CancellationToken::new();

	let (nats, crdb, redis) = tokio::try_join!(
		nats_from_env(client_name.clone()),
		crdb_from_env(client_name.clone()),
		redis_from_env(),
	)?;

	let pool = Arc::new(PoolsInner {
		_guard: token.clone().drop_guard(),
		nats,
		crdb,
		redis,
	});
	pool.clone().start(token);

	tokio::task::Builder::new()
		.name("rivet_pools::runtime")
		.spawn(runtime(pool.clone(), client_name.clone()))
		.map_err(Error::TokioSpawn)?;

	Ok(pool)
}

#[tracing::instrument]
async fn nats_from_env(client_name: String) -> Result<Option<NatsPool>, Error> {
	if let Ok(urls) = env::var("NATS_URL") {
		// Randomize the URLs in order to randomize the node priority and load
		// balance connections across nodes.
		let mut shuffled_urls = urls.split(",").collect::<Vec<_>>();
		shuffled_urls.shuffle(&mut rand::thread_rng());

		// Parse nodes
		let server_addrs = shuffled_urls
			.iter()
			.map(|url| async_nats::ServerAddr::from_str(url))
			.collect::<Result<Vec<_>, _>>()
			.map_err(Error::BuildNatsIo)?;

		let mut options = if let (Ok(username), Ok(password)) =
			(env::var("NATS_USERNAME"), env::var("NATS_PASSWORD"))
		{
			async_nats::ConnectOptions::with_user_and_password(username, password.clone())
		} else {
			async_nats::ConnectOptions::new()
		};
		options = options
			// Flush frequently since we don't flush messages
			// TODO: Lower this interval in development to reduce overhead
			.flush_interval(Duration::from_millis(50))
			.client_capacity(256)
			.subscription_capacity(8192)
			.event_callback({
				let server_addrs = server_addrs.clone();
				move |event| {
					let server_addrs = server_addrs.clone();
					async move {
						match event {
							async_nats::Event::Connected => {
								tracing::info!(?server_addrs, "nats reconnected");
							}
							async_nats::Event::Disconnected => {
								tracing::error!(?server_addrs, "nats disconnected");
							}
							async_nats::Event::LameDuckMode => {
								tracing::warn!(?server_addrs, "nats lame duck mode");
							}
							async_nats::Event::SlowConsumer(_) => {
								tracing::warn!(?server_addrs, "nats slow consumer");
							}
							async_nats::Event::ServerError(err) => {
								tracing::error!(?server_addrs, ?err, "nats server error");
							}
							async_nats::Event::ClientError(err) => {
								tracing::error!(?server_addrs, ?err, "nats client error");
							}
						}
					}
				}
			});

		// NATS has built in backoff with jitter (with max of 4s), so
		// once the connection is established, we never have to worry
		// about disconnections that aren't handled by NATS.
		tracing::info!(?server_addrs, "nats connecting");
		let conn = options
			.connect(&server_addrs[..])
			.await
			.map_err(Error::BuildNats)?;
		tracing::info!(?server_addrs, "nats connected");

		Ok(Some(conn))
	} else {
		Ok(None)
	}
}

#[tracing::instrument]
async fn crdb_from_env(client_name: String) -> Result<Option<CrdbPool>, Error> {
	if let Some(url) = std::env::var("CRDB_URL").ok() {
		tracing::info!(%url, "crdb connecting");

		// let client_name = client_name.clone();
		let pool = sqlx::postgres::PgPoolOptions::new()
			// The default connection timeout is too high
			.acquire_timeout(Duration::from_secs(15))
			.max_lifetime(Duration::from_secs(60 * 5))
			// Remove connections after a while in order to reduce load
			// on CRDB after bursts
			.idle_timeout(Some(Duration::from_secs(60)))
			// Open a connection
			// immediately on startup
			.min_connections(1)
			// Raise the cap, since this is effectively the amount of
			// simultaneous requests we can handle. See
			// https://www.cockroachlabs.com/docs/stable/connection-pooling.html
			.max_connections(4096)
			// Speeds up requests at the expense of potential
			// failures
			.test_before_acquire(false)
			.after_connect({
				let url = url.clone();
				move |_, _| {
					// let client_name = client_name.clone();
					let url = url.clone();
					Box::pin(async move {
						tracing::info!(%url, "crdb connected");
						// sqlx::query("SET application_name = $1;")
						// 	.bind(&client_name)
						// 	.execute(conn)
						// 	.await?;
						Ok(())
					})
				}
			})
			.connect(&url)
			.await
			.map_err(Error::BuildSqlx)?;

		Ok(Some(pool))
	} else {
		Ok(None)
	}
}

#[tracing::instrument]
async fn redis_from_env() -> Result<HashMap<String, RedisPool>, Error> {
	// Create Redis connections
	let mut join_set = JoinSet::new();
	for (key, url) in env::vars() {
		if let Some(svc_name_screaming) = key.strip_prefix("REDIS_URL_") {
			let svc_name = svc_name_screaming.to_lowercase().replace("_", "-");

			join_set
				.build_task()
				.name("redis_from_env")
				.spawn(async move {
					tracing::info!(%url, "redis connecting");
					let conn = redis::cluster::ClusterClient::builder(vec![url.as_str()])
						// Keep trying to reconnect indefinitely
						.retries(u32::MAX)
						.min_retry_wait(250)
						.max_retry_wait(30_000)
						.build()
						.map_err(Error::BuildRedis)?
						.get_async_connection()
						.await
						.map_err(Error::BuildRedis)?;

					tracing::info!(%url, "redis connected");

					Ok((svc_name, conn))
				})
				.map_err(Error::TokioSpawn)?;
		}
	}

	// Join connections
	let mut redis = HashMap::new();
	while let Some(res) = join_set.join_next().await {
		let (svc_name, conn) = res.map_err(Error::TokioJoin)??;
		redis.insert(svc_name, conn.clone());
	}

	Ok(redis)
}

#[tracing::instrument(level = "trace", skip(pools))]
async fn runtime(pools: Pools, client_name: String) {
	// TODO: Delete this once confirmed this is no longer an issue

	// We have to manually ping the Redis connection since `ConnectionManager`
	// doesn't do this for us. If we don't make a request on a Redis connection
	// for a long time, we'll get a broken pipe error, so this keeps the
	// connection alive.

	// let mut interval = tokio::time::interval(Duration::from_secs(15));
	// loop {
	// 	interval.tick().await;

	// 	// TODO: This will ping the same pool multiple times if it shares the
	// 	// same URL
	// 	for (db, conn) in &pools.redis {
	// 		// HACK: Instead of sending `PING`, we test the connection by
	// 		// updating the client's name. We do this because
	// 		// `ConnectionManager` doesn't let us hook in to new connections, so
	// 		// we have to manually update the client's name.
	// 		let mut conn = conn.clone();
	// 		let res = redis::cmd("CLIENT")
	// 			.arg("SETNAME")
	// 			.arg(&client_name)
	// 			.query_async::<_, ()>(&mut conn)
	// 			.await;
	// 		match res {
	// 			Ok(_) => {
	// 				tracing::trace!(%db, "ping success");
	// 			}
	// 			Err(err) => {
	// 				tracing::error!(%db, ?err, "redis ping failed");
	// 			}
	// 		}
	// 	}
	// }
}

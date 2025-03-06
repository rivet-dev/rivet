use std::{collections::HashMap, sync::Arc, time::Duration};

use global_error::{ensure_with, prelude::*, GlobalResult};
use rivet_config::Config;
use tokio_util::sync::{CancellationToken, DropGuard};

use crate::{
	db::sqlite::SqlitePoolManager, ClickHousePool, CrdbPool, Error, FdbPool, NatsPool, RedisPool,
	SqlitePool,
};

// TODO: Automatically shutdown all pools on drop
pub(crate) struct PoolsInner {
	pub(crate) _guard: DropGuard,
	pub(crate) nats: Option<NatsPool>,
	pub(crate) crdb: Option<CrdbPool>,
	pub(crate) redis: HashMap<String, RedisPool>,
	pub(crate) clickhouse: Option<clickhouse::Client>,
	pub(crate) fdb: Option<FdbPool>,
	pub(crate) sqlite: SqlitePoolManager,
	clickhouse_enabled: bool,
}

#[derive(Clone)]
pub struct Pools(Arc<PoolsInner>);

impl Pools {
	#[tracing::instrument(skip(config))]
	pub async fn new(config: Config) -> Result<Pools, Error> {
		// TODO: Choose client name for this service
		let client_name = "rivet".to_string();
		let token = CancellationToken::new();

		let (nats, crdb, redis, fdb) = tokio::try_join!(
			crate::db::nats::setup(config.clone(), client_name.clone()),
			crate::db::crdb::setup(config.clone()),
			crate::db::redis::setup(config.clone()),
			crate::db::fdb::setup(config.clone()),
		)?;
		let clickhouse = crate::db::clickhouse::setup(config.clone())?;

		let pool = Pools(Arc::new(PoolsInner {
			_guard: token.clone().drop_guard(),
			nats: Some(nats),
			crdb: Some(crdb),
			redis,
			clickhouse,
			fdb: Some(fdb),
			sqlite: SqlitePoolManager::new(),
			clickhouse_enabled: config
				.server
				.as_ref()
				.map_or(false, |x| x.clickhouse.is_some()),
		}));
		pool.clone().start(token);

		tokio::task::Builder::new()
			.name("rivet_pools::runtime")
			.spawn(runtime(pool.clone(), client_name.clone()))
			.map_err(Error::TokioSpawn)?;

		Ok(pool)
	}

	// Only for tests
	#[tracing::instrument(skip(config))]
	pub async fn test(config: Config) -> Result<Pools, Error> {
		// TODO: Choose client name for this service
		let client_name = "rivet".to_string();
		let token = CancellationToken::new();

		let (nats, redis, fdb) = tokio::try_join!(
			crate::db::nats::setup(config.clone(), client_name.clone()),
			crate::db::redis::setup(config.clone()),
			crate::db::fdb::setup(config.clone()),
		)?;

		let pool = Pools(Arc::new(PoolsInner {
			_guard: token.clone().drop_guard(),
			nats: Some(nats),
			crdb: None,
			redis,
			clickhouse: None,
			fdb: Some(fdb),
			sqlite: SqlitePoolManager::new(),
			clickhouse_enabled: config
				.server
				.as_ref()
				.map_or(false, |x| x.clickhouse.is_some()),
		}));
		pool.clone().start(token);

		tokio::task::Builder::new()
			.name("rivet_pools::runtime")
			.spawn(runtime(pool.clone(), client_name.clone()))
			.map_err(Error::TokioSpawn)?;

		Ok(pool)
	}

	/// Spawn background tasks required to operate the pool.
	pub(crate) fn start(self, token: CancellationToken) {
		let spawn_res = tokio::task::Builder::new()
			.name("PoolsInner::record_metrics")
			.spawn(self.clone().record_metrics_loop(token));
		if let Err(err) = spawn_res {
			tracing::error!(?err, "failed to spawn record_metrics task");
		}
	}

	// MARK: Getters
	pub fn nats_option(&self) -> &Option<NatsPool> {
		&self.0.nats
	}

	pub fn redis_map(&self) -> &HashMap<String, RedisPool> {
		&self.0.redis
	}

	// MARK: Pool lookups
	pub fn nats(&self) -> Result<NatsPool, Error> {
		self.0.nats.clone().ok_or(Error::MissingNatsPool)
	}

	pub fn crdb(&self) -> Result<CrdbPool, Error> {
		self.0.crdb.clone().ok_or(Error::MissingCrdbPool)
	}

	pub fn redis(&self, key: &str) -> Result<RedisPool, Error> {
		self.0
			.redis
			.get(key)
			.cloned()
			.ok_or_else(|| Error::MissingRedisPool {
				key: Some(key.to_string()),
			})
	}

	pub fn redis_chirp(&self) -> Result<RedisPool, Error> {
		self.redis("persistent")
	}

	pub fn redis_chirp_ephemeral(&self) -> Result<RedisPool, Error> {
		self.redis("ephemeral")
	}

	pub fn redis_cache(&self) -> Result<RedisPool, Error> {
		self.redis("ephemeral")
	}

	pub fn clickhouse_enabled(&self) -> bool {
		self.0.clickhouse_enabled
	}

	pub fn clickhouse(&self) -> GlobalResult<ClickHousePool> {
		ensure_with!(
			self.clickhouse_enabled(),
			FEATURE_DISABLED,
			feature = "Clickhouse"
		);

		let ch = unwrap!(self.0.clickhouse.clone(), "missing clickhouse pool");
		Ok(ch)
	}

	pub fn fdb(&self) -> Result<FdbPool, Error> {
		self.0.fdb.clone().ok_or(Error::MissingFdbPool)
	}

	pub async fn sqlite(&self, key: impl AsRef<str>, read_only: bool) -> Result<SqlitePool, Error> {
		self.0.sqlite.get(key.as_ref(), read_only).await
	}

	#[tracing::instrument(skip_all)]
	async fn record_metrics_loop(self, token: CancellationToken) {
		let cancelled = token.cancelled();
		tokio::pin!(cancelled);

		let mut interval = tokio::time::interval(Duration::from_secs(5));
		loop {
			tokio::select! {
				_ = &mut cancelled => {
					tracing::info!("record metrics cancelled");
					break
				}
				_ = interval.tick() => {
					self.record_metrics().await;
				}
			}
		}
	}

	#[tracing::instrument(skip_all)]
	async fn record_metrics(&self) {
		use crate::metrics::*;

		if let Some(pool) = &self.0.crdb {
			CRDB_POOL_SIZE.set(pool.size() as i64);
			CRDB_POOL_NUM_IDLE.set(pool.num_idle() as i64);
		}
	}
}

#[tracing::instrument(level = "trace", skip(pools))]
async fn runtime(pools: Pools, client_name: String) {
	// We have to manually ping the Redis connection since `ConnectionManager`
	// doesn't do this for us. If we don't make a request on a Redis connection
	// for a long time, we'll get a broken pipe error, so this keeps the
	// connection alive.

	let mut interval = tokio::time::interval(Duration::from_secs(15));
	loop {
		interval.tick().await;

		for (db, conn) in &pools.0.redis {
			// HACK: Instead of sending `PING`, we test the connection by
			// updating the client's name. We do this because
			// `ConnectionManager` doesn't let us hook in to new connections, so
			// we have to manually update the client's name.
			let mut conn = conn.clone();
			let res = redis::cmd("CLIENT")
				.arg("SETNAME")
				.arg(&client_name)
				.query_async::<_, ()>(&mut conn)
				.await;
			match res {
				Result::Ok(_) => {
					tracing::trace!(%db, "ping success");
				}
				Err(err) => {
					tracing::error!(%db, ?err, "redis ping failed");
				}
			}
		}
	}
}

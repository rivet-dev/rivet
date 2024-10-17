use std::{collections::HashMap, sync::Arc, time::Duration};

use global_error::{ensure_with, GlobalResult};
use tokio_util::sync::{CancellationToken, DropGuard};

use crate::Error;

pub type NatsPool = async_nats::Client;
pub type CrdbPool = sqlx::PgPool;
pub type RedisPool = redis::aio::ConnectionManager;
pub type ClickHousePool = clickhouse::Client;

pub type Pools = Arc<PoolsInner>;

pub struct PoolsInner {
	pub(crate) _guard: DropGuard,
	pub(crate) nats: Option<NatsPool>,
	pub(crate) crdb: Option<CrdbPool>,
	pub(crate) redis: HashMap<String, RedisPool>,
	pub(crate) clickhouse: Option<clickhouse::Client>,
}

impl PoolsInner {
	/// Spawn background tasks required to operate the pool.
	pub(crate) fn start(self: Arc<Self>, token: CancellationToken) {
		let spawn_res = tokio::task::Builder::new()
			.name("PoolsInner::record_metrics")
			.spawn(self.record_metrics_loop(token));
		if let Err(err) = spawn_res {
			tracing::error!(?err, "failed to spawn record_metrics task");
		}
	}
}

impl PoolsInner {
	// MARK: Getters
	pub fn nats_option(&self) -> &Option<NatsPool> {
		&self.nats
	}

	pub fn redis_map(&self) -> &HashMap<String, RedisPool> {
		&self.redis
	}

	// MARK: Pool lookups
	pub fn nats(&self) -> Result<NatsPool, Error> {
		self.nats.clone().ok_or(Error::MissingNatsPool)
	}

	pub fn crdb(&self) -> Result<CrdbPool, Error> {
		self.crdb.clone().ok_or(Error::MissingCrdbPool)
	}

	pub fn redis(&self, key: &str) -> Result<RedisPool, Error> {
		self.redis
			.get(key)
			.cloned()
			.ok_or_else(|| Error::MissingRedisPool {
				key: Some(key.to_owned()),
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
		std::env::var("CLICKHOUSE_DISABLED").is_err()
	}

	pub fn clickhouse(&self) -> GlobalResult<ClickHousePool> {
		ensure_with!(
			self.clickhouse_enabled(),
			FEATURE_DISABLED,
			feature = "Clickhouse"
		);

		self.clickhouse
			.clone()
			.ok_or(Error::MissingClickHousePool)
			.map_err(Into::into)
	}
}

impl PoolsInner {
	#[tracing::instrument(skip_all)]
	async fn record_metrics_loop(self: Arc<Self>, token: CancellationToken) {
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

		if let Some(pool) = &self.crdb {
			CRDB_POOL_SIZE.set(pool.size() as i64);
			CRDB_POOL_NUM_IDLE.set(pool.num_idle() as i64);
		}
	}
}

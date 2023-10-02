use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio_util::sync::{CancellationToken, DropGuard};

use crate::Error;

pub type NatsPool = async_nats::Client;
pub type CrdbPool = sqlx::PgPool;
pub type RedisPool = redis::cluster_async::ClusterConnection;

// #[derive(Clone)]
// struct RedisPool {
// 	connection: redis::cluster_async::ClusterConnection,
// }

// impl RedisPool {
// 	pub async fn pipe<T, F>(&mut self, f: F) -> RedisResult<T> where F: FnOnce(redis::cluster::ClusterPipeline) -> redis::cluster::ClusterPipeline {
// 		let mut pipe = redis::cluster::cluster_pipe();

// 		f(pipe);

// 		thread::spawn(|| async {
// 			pipe.query(&mut self)
// 		}).await
// 	}

// 	pub async fn cmd<T, F>(&mut self, cmd: &str, f: F) -> RedisResult<T> where F: FnOnce(redis::cluster::ClusterPipeline) -> redis::cluster::ClusterPipeline {
// 		let mut cmd = redis::cluster::cmd(cmd);

// 		f(cmd);

// 		thread::spawn(|| async {
// 			pipe.query(&mut self)
// 		}).await
// 	}
// }

pub type Pools = Arc<PoolsInner>;

pub struct PoolsInner {
	pub(crate) _guard: DropGuard,
	pub(crate) nats: Option<NatsPool>,
	pub(crate) crdb: HashMap<String, CrdbPool>,
	pub(crate) redis: HashMap<String, RedisPool>,
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

	pub fn crdb_map(&self) -> &HashMap<String, CrdbPool> {
		&self.crdb
	}

	pub fn redis_map(&self) -> &HashMap<String, RedisPool> {
		&self.redis
	}

	// MARK: Pool lookups
	pub fn nats(&self) -> Result<NatsPool, Error> {
		self.nats.clone().ok_or(Error::MissingNatsPool)
	}

	pub fn crdb(&self, key: &str) -> Result<CrdbPool, Error> {
		self.crdb
			.get(key)
			.cloned()
			.ok_or_else(|| Error::MissingCrdbPool {
				key: Some(key.to_owned()),
			})
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
		self.redis("chirp")
	}

	pub fn redis_cache(&self) -> Result<RedisPool, Error> {
		self.redis("cache")
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

		// CRDB
		for (db_name, pool) in self.crdb_map() {
			let label = &[db_name.as_str()];
			CRDB_POOL_SIZE
				.with_label_values(label)
				.set(pool.size() as i64);
			CRDB_POOL_NUM_IDLE
				.with_label_values(label)
				.set(pool.num_idle() as i64);
		}
	}
}

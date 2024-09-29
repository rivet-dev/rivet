use std::time::Duration;

use anyhow::*;
use sqlx::{pool::PoolConnection, PgPool, Postgres};

pub async fn build_pool(url: &str) -> Result<PgPool> {
	sqlx::postgres::PgPoolOptions::new()
		.acquire_timeout(Duration::from_secs(60))
		.max_lifetime(Duration::from_secs(15 * 60))
		.max_lifetime_jitter(Duration::from_secs(90))
		.idle_timeout(Some(Duration::from_secs(10 * 60)))
		.min_connections(1)
		.max_connections(4096)
		.connect(url)
		.await
		.map_err(Into::into)
}

pub async fn get_conn(pool: &PgPool) -> Result<PoolConnection<Postgres>> {
	// Attempt to use an existing connection
	if let Some(conn) = pool.try_acquire() {
		Ok(conn)
	} else {
		// Create a new connection
		pool.acquire().await.map_err(Into::into)
	}
}

use rivet_config::Config;
use std::time::Duration;

use crate::Error;

pub type CrdbPool = sqlx::PgPool;

#[tracing::instrument(skip(config))]
pub async fn setup(config: Config) -> Result<CrdbPool, Error> {
	let crdb = &config.server().map_err(Error::Global)?.cockroachdb;
	tracing::debug!("crdb connecting");

	let mut opts: sqlx::postgres::PgConnectOptions =
		crdb.url.to_string().parse().map_err(Error::BuildSqlx)?;
	opts = opts.username(&crdb.username);
	if let Some(password) = &crdb.password {
		opts = opts.password(password.read());
	}

	let pool = sqlx::postgres::PgPoolOptions::new()
		// Reduced from 30s to allow retries within API timeout (50s).
		// With 10s acquire + 8s query = 18s per attempt, allowing 2-3 retries.
		.acquire_timeout(Duration::from_secs(10))
		// Increase lifetime to mitigate: https://github.com/launchbadge/sqlx/issues/2854
		//
		// See max lifetime https://www.cockroachlabs.com/docs/stable/connection-pooling#set-the-maximum-lifetime-of-connections
		//
		// Reduce this to < 10 minutes since GCP has a 10 minute idle TCP timeout that causes
		// problems. Unsure if idle_timeout is actually working correctly, so we're being cautious
		// here.
		.max_lifetime(Duration::from_secs(8 * 60))
		.max_lifetime_jitter(Duration::from_secs(90))
		// Remove connections after a while in order to reduce load on CRDB after bursts.
		//
		// IMPORTANT: Must be less than 10 minutes due to GCP's connection tracking timeout.
		// See https://cloud.google.com/compute/docs/troubleshooting/general-tips
		.idle_timeout(Some(Duration::from_secs(5 * 60)))
		// Open connections immediately on startup
		.min_connections(crdb.min_connections)
		// Raise the cap, since this is effectively the amount of
		// simultaneous requests we can handle. See
		// https://www.cockroachlabs.com/docs/stable/connection-pooling.html
		.max_connections(crdb.max_connections)
		// Ping connections before use to validate they're still alive.
		// This catches stale connections that may have been dropped by load balancers
		// or firewalls (e.g., GCP's 10-minute idle timeout, AWS NAT gateway timeout).
		.test_before_acquire(true)
		// NOTE: Server-side statement_timeout is not reliable for cross-cloud connections
		// because if the network is dead, CockroachDB can't send the timeout error back.
		// Instead, we use client-side timeout (tokio::time::timeout) in the SQL macros.
		// See QUERY_TIMEOUT_SECS in sql_query_macros.rs.
		.connect_with(opts)
		.await
		.map_err(Error::BuildSqlx)?;

	tracing::debug!("crdb connected");

	Ok(pool)
}

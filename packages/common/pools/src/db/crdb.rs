use rivet_config::Config;
use std::time::Duration;

use crate::Error;

pub type CrdbPool = sqlx::PgPool;

#[tracing::instrument(skip(config))]
pub async fn setup(config: Config) -> Result<CrdbPool, Error> {
	let crdb = &config.server().map_err(Error::Global)?.cockroachdb;
	tracing::debug!("crdb connecting");

	// let client_name = client_name.clone();

	let mut opts: sqlx::postgres::PgConnectOptions =
		crdb.url.to_string().parse().map_err(Error::BuildSqlx)?;
	opts = opts.username(&crdb.username);
	if let Some(password) = &crdb.password {
		opts = opts.password(password.read());
	}

	let pool = sqlx::postgres::PgPoolOptions::new()
		// The default connection timeout is too high
		.acquire_timeout(Duration::from_secs(60))
		// Increase lifetime to mitigate: https://github.com/launchbadge/sqlx/issues/2854
		//
		// See max lifetime https://www.cockroachlabs.com/docs/stable/connection-pooling#set-the-maximum-lifetime-of-connections
		.max_lifetime(Duration::from_secs(15 * 60))
		.max_lifetime_jitter(Duration::from_secs(90))
		// Remove connections after a while in order to reduce load
		// on CRDB after bursts
		.idle_timeout(Some(Duration::from_secs(10 * 60)))
		// Open connections immediately on startup
		.min_connections(crdb.min_connections)
		// Raise the cap, since this is effectively the amount of
		// simultaneous requests we can handle. See
		// https://www.cockroachlabs.com/docs/stable/connection-pooling.html
		.max_connections(crdb.max_connections)
		// NOTE: This is disabled until we can ensure that TCP connections stop getting dropped
		// on AWS.
		// // Speeds up requests at the expense of potential
		// // failures. See `before_acquire`.
		// .test_before_acquire(false)
		// // Ping once per minute to validate the connection is still alive
		// .before_acquire(|conn, meta| {
		// 	Box::pin(async move {
		// 		if meta.idle_for.as_secs() < 60 {
		// 			Ok(true)
		// 		} else {
		// 			match sqlx::Connection::ping(conn).await {
		// 				Ok(_) => Ok(true),
		// 				Err(err) => {
		// 					// See https://docs.aws.amazon.com/vpc/latest/userguide/nat-gateway-troubleshooting.html#nat-gateway-troubleshooting-timeout
		// 					tracing::warn!(
		// 						?err,
		// 						"crdb ping failed, potential idle tcp connection drop"
		// 					);
		// 					Ok(false)
		// 				}
		// 			}
		// 		}
		// 	})
		// })
		.connect_with(opts)
		.await
		.map_err(Error::BuildSqlx)?;

	tracing::debug!("crdb connected");

	Ok(pool)
}

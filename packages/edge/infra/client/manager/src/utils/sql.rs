use std::{
	future::Future,
	result::Result::{Err, Ok},
	time::Duration,
};

use anyhow::*;
use sqlx::{pool::PoolConnection, Sqlite, SqlitePool};

use crate::metrics;

const MAX_QUERY_RETRIES: usize = 16;
const QUERY_RETRY: Duration = Duration::from_millis(500);

pub trait SqlitePoolExt {
	fn conn(&self) -> impl Future<Output = sqlx::Result<PoolConnection<Sqlite>>>;
}

impl SqlitePoolExt for SqlitePool {
	async fn conn(&self) -> sqlx::Result<PoolConnection<Sqlite>> {
		// Attempt to use an existing connection
		if let Some(conn) = self.try_acquire() {
			Ok(conn)
		} else {
			// Create a new connection
			self.acquire().await
		}
	}
}


/// Executes queries and explicitly handles retry errors.
pub async fn query<'a, F, Fut, T>(mut cb: F) -> Result<T>
where
	F: FnMut() -> Fut,
	Fut: Future<Output = std::result::Result<T, sqlx::Error>> + 'a,
	T: 'a,
{
	let mut i = 0;

	loop {
		match cb().await {
			Ok(x) => return Ok(x),
			Err(err) => {
				use sqlx::Error::*;

				metrics::SQL_ERROR
					.with_label_values(&[&err.to_string()])
					.inc();

				if i > MAX_QUERY_RETRIES {
					bail!("max sql retries: {err:?}");
				}
				i += 1;

				match &err {
					// Retry internal errors with a backoff
					Database(_) | Io(_) | Tls(_) | Protocol(_) | PoolTimedOut | PoolClosed
					| WorkerCrashed => {
						tracing::info!(?err, "query retry");
						tokio::time::sleep(QUERY_RETRY).await;
					}
					// Throw error
					_ => return Err(err.into()),
				}
			}
		}
	}
}

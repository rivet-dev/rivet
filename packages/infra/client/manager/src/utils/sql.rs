use std::{
	future::Future,
	ops::{Deref, DerefMut},
	result::Result::{Err, Ok},
	time::Duration,
};

use anyhow::*;
use sqlx::sqlite::SqliteQueryResult;
use sqlx::{Executor, SqliteConnection};

use crate::metrics;

const MAX_QUERY_RETRIES: usize = 16;
const QUERY_RETRY: Duration = Duration::from_millis(500);

pub(crate) trait SqliteConnectionExt {
	fn begin_immediate(&mut self) -> impl Future<Output = sqlx::Result<Transaction>>;
}

impl SqliteConnectionExt for SqliteConnection {
	async fn begin_immediate(&mut self) -> sqlx::Result<Transaction> {
		let conn = &mut *self;

		conn.execute("BEGIN IMMEDIATE;").await?;

		Ok(Transaction {
			conn,
			is_open: true,
		})
	}
}

pub(crate) struct Transaction<'c> {
	conn: &'c mut SqliteConnection,
	/// is the transaction open?
	is_open: bool,
}

impl<'c> Transaction<'c> {
	pub(crate) async fn commit(mut self) -> sqlx::Result<SqliteQueryResult> {
		let res = self.conn.execute("COMMIT;").await;

		if res.is_ok() {
			self.is_open = false;
		}

		res
	}
}

impl<'c> Drop for Transaction<'c> {
	fn drop(&mut self) {
		if self.is_open {
			let handle = tokio::runtime::Handle::current();
			handle.block_on(async move {
				let _ = self.execute("ROLLBACK").await;
			});
		}
	}
}

impl<'c> Deref for Transaction<'c> {
	type Target = SqliteConnection;

	fn deref(&self) -> &Self::Target {
		self.conn
	}
}

impl<'c> DerefMut for Transaction<'c> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.conn
	}
}

/// Executes queries and explicitly handles retry errors.
pub async fn query<'a, F, Fut, T>(mut cb: F) -> Result<T>
where
	F: FnMut() -> Fut,
	Fut: std::future::Future<Output = std::result::Result<T, sqlx::Error>> + 'a,
	T: 'a,
{
	let mut i = 0;

	loop {
		match cb().await {
			std::result::Result::Ok(x) => return Ok(x),
			std::result::Result::Err(err) => {
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

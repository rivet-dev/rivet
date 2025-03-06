use std::future::Future;

use sqlx::{pool::PoolConnection, Sqlite, SqlitePool};

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

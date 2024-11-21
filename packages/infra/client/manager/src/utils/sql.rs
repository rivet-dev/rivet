use std::{
	future::Future,
	ops::{Deref, DerefMut},
};

use sqlx::sqlite::SqliteQueryResult;
use sqlx::{Executor, SqliteConnection};

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

use std::future::Future;

use anyhow::{Context, Result, anyhow};
use futures_util::FutureExt;

use crate::{
	driver::{DatabaseDriverHandle, Erased},
	options::DatabaseOption,
	transaction::{RetryableTransaction, Transaction},
};

#[derive(Clone)]
pub struct Database {
	driver: DatabaseDriverHandle,
}

impl Database {
	pub fn new(driver: DatabaseDriverHandle) -> Self {
		Database { driver }
	}

	/// Run a closure with automatic retry logic
	pub async fn run<'a, F, Fut, T>(&'a self, closure: F) -> Result<T>
	where
		F: Fn(RetryableTransaction) -> Fut + Send + Sync,
		Fut: Future<Output = Result<T>> + Send,
		T: Send + 'a + 'static,
	{
		let closure = &closure;
		self.driver
			.run(Box::new(|tx| {
				async move { closure(tx).await.map(|value| Box::new(value) as Erased) }.boxed()
			}))
			.await
			.and_then(|res| {
				res.downcast::<T>()
					.map(|x| *x)
					.map_err(|_| anyhow!("failed to downcast `run` return type"))
			})
			.context("transaction failed")
	}

	/// Creates a new txn instance.
	pub fn create_trx(&self) -> Result<Transaction> {
		self.driver.create_trx()
	}

	/// Set a database option
	pub fn set_option(&self, opt: DatabaseOption) -> Result<()> {
		self.driver.set_option(opt)
	}
}

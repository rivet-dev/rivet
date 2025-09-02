use std::future::Future;

use futures_util::FutureExt;

use crate::{FdbBindingError, FdbResult, driver::Erased};

use crate::{
	MaybeCommitted, RetryableTransaction, Transaction, driver::DatabaseDriverHandle,
	options::DatabaseOption,
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
	pub async fn run<'a, F, Fut, T>(&'a self, closure: F) -> Result<T, FdbBindingError>
	where
		F: Fn(RetryableTransaction, MaybeCommitted) -> Fut + Send + Sync,
		Fut: Future<Output = Result<T, FdbBindingError>> + Send,
		T: Send + 'a + 'static,
	{
		let closure = &closure;
		self.driver
			.run(Box::new(|tx, mc| {
				async move { closure(tx, mc).await.map(|value| Box::new(value) as Erased) }.boxed()
			}))
			.await
			.and_then(|res| {
				res.downcast::<T>().map(|x| *x).map_err(|_| {
					FdbBindingError::CustomError("failed to downcast `run` return type".into())
				})
			})
	}

	/// Creates a new txn instance.
	pub fn create_trx(&self) -> FdbResult<Transaction> {
		self.driver.create_trx()
	}

	/// Set a database option
	pub fn set_option(&self, opt: DatabaseOption) -> FdbResult<()> {
		self.driver.set_option(opt)
	}
}

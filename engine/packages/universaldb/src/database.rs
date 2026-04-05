use std::time::Instant;
use std::{
	future::Future,
	sync::atomic::{AtomicUsize, Ordering},
};

use anyhow::{Context, Result, anyhow};
use futures_util::FutureExt;
use rivet_tracing_utils::CustomInstrumentExt;
use tracing::Instrument;

use crate::{
	driver::{DatabaseDriverHandle, Erased},
	metrics,
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

	/// Run a closure with automatic retry logic.
	#[tracing::instrument(skip_all)]
	pub async fn run<'a, F, Fut, T>(&'a self, closure: F) -> Result<T>
	where
		F: Fn(RetryableTransaction) -> Fut + Send + Sync,
		Fut: Future<Output = Result<T>> + Send,
		T: Send + 'a + 'static,
	{
		self.txn("unnamed", closure).in_current_span().await
	}

	/// Run a closure with automatic retry logic and a name.
	#[tracing::instrument(skip_all)]
	pub async fn txn<'a, F, Fut, T>(&'a self, name: &'static str, closure: F) -> Result<T>
	where
		F: Fn(RetryableTransaction) -> Fut + Send + Sync,
		Fut: Future<Output = Result<T>> + Send,
		T: Send + 'a + 'static,
	{
		let start = Instant::now();
		let attempts = AtomicUsize::new(0);
		metrics::TRANSACTION_TOTAL.with_label_values(&[name]).inc();
		metrics::TRANSACTION_PENDING
			.with_label_values(&[name])
			.inc();

		let closure = &closure;
		let res = self
			.driver
			.run(Box::new(|tx| {
				attempts.fetch_add(1, Ordering::AcqRel);

				async move { closure(tx).await.map(|value| Box::new(value) as Erased) }
					.custom_instrument(tracing::info_span!("txn_attempt"))
					.boxed()
			}))
			.await
			.and_then(|res| {
				res.downcast::<T>()
					.map(|x| *x)
					.map_err(|_| anyhow!("failed to downcast `run` return type"))
			})
			.context("transaction failed");

		metrics::TRANSACTION_ATTEMPTS
			.with_label_values(&[name])
			.observe(attempts.load(Ordering::Acquire) as f64);
		metrics::TRANSACTION_PENDING
			.with_label_values(&[name])
			.dec();
		metrics::TRANSACTION_DURATION
			.with_label_values(&[name])
			.observe(start.elapsed().as_secs_f64());

		res
	}

	/// Creates a new txn instance.
	pub fn create_trx(&self) -> Result<Transaction> {
		self.driver.create_trx()
	}

	/// Set a database option
	pub fn set_option(&self, opt: DatabaseOption) -> Result<()> {
		self.driver.set_option(opt)
	}

	pub fn checkpoint(&self, path: &std::path::Path) -> Result<()> {
		self.driver.checkpoint(path)
	}
}

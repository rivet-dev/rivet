// Helpful links:
// - https://www.cockroachlabs.com/docs/v22.2/transaction-retry-error-reference
// - https://www.cockroachlabs.com/docs/v22.2/advanced-client-side-transaction-retries
// - https://www.cockroachlabs.com/docs/v22.2/transactions#client-side-intervention
// - https://github.com/cockroachdb/docs/blob/1250d113dcb6de3a885eef1f3b2dfbc6d7eba5fa/_includes/v2.0/app/txn-sample.rs#L10

use crate::CrdbPool;
use global_error::prelude::*;
use std::{future::Future, pin::Pin};

const MAX_TX_RETRIES: usize = 16;

pub type AsyncResult<'a, T> = Pin<Box<dyn Future<Output = GlobalResult<T>> + Send + 'a>>;

/// Runs a transaction. This explicitly handles retry errors.
///
/// See
/// https://www.cockroachlabs.com/docs/v22.2/advanced-client-side-transaction-retries
/// 
/// **NOTE** The transaction will be rolled back if the future is cancelled. See
/// https://docs.rs/sqlx/0.7.4/sqlx/struct.Transaction.html
#[tracing::instrument(skip_all)]
pub async fn tx<T, F>(crdb: &CrdbPool, f: F) -> GlobalResult<T>
where
	for<'t> F: Fn(&'t mut sqlx::Transaction<'_, sqlx::Postgres>) -> AsyncResult<'t, T>,
{
	for _ in 0..MAX_TX_RETRIES {
		let mut tx = crdb.begin().await?;

		match f(&mut tx).await {
			// TODO: Hack. We should be downcasting the error.
			Err(GlobalError::Internal { ty, message, .. })
				if ty == "sqlx_core::error::Error::Database"
					&& message.contains("TransactionRetryWithProtoRefreshError") =>
			{
				tracing::info!(%message, "transaction retry");
				match tx.rollback().await {
					Ok(_) => {}
					Err(err) => {
						tracing::error!(?err, "failed to roll back transaction");
					}
				}
			}
			Err(err) => {
				tx.rollback().await?;
				return Err(err);
			}
			Ok(x) => {
				tx.commit().await?;
				return Ok(x);
			}
		}
	}

	bail!("transaction failed with retry too many times");
}

/// Runs a transaction without retrying.
#[tracing::instrument(skip_all)]
pub async fn tx_no_retry<T, F>(crdb: &CrdbPool, f: F) -> GlobalResult<T>
where
	for<'t> F: Fn(&'t mut sqlx::Transaction<'_, sqlx::Postgres>) -> AsyncResult<'t, T>,
{
	let mut tx = crdb.begin().await?;

	match f(&mut tx).await {
		Err(err) => {
			tx.rollback().await?;
			Err(err)
		}
		Ok(x) => {
			tx.commit().await?;
			Ok(x)
		}
	}
}

// TODO: This seems to leak connections on retries, even though it matches the
// CRDB spec. This is likely because of odd behavior in the sqlx driver.
///// Runs a transaction. This explicitly handles retry errors.
/////
///// See
///// https://www.cockroachlabs.com/docs/v22.2/advanced-client-side-transaction-retries
//#[tracing::instrument(skip_all)]
//pub async fn tx<T, F>(crdb: &CrdbPool, f: F) -> WorkerResult<T>
//where
//	for<'t> F: Fn(&'t mut sqlx::Transaction<'_, sqlx::Postgres>) -> AsyncResult<'t, T>,
//{
//	let mut tx = crdb.begin().await?;

//	sqlx::query("SAVEPOINT cockroach_restart")
//		.execute(&mut tx)
//		.await?;

//	for _ in 0..MAX_TX_RETRIES {
//		let (tx_returned, res) = execute_fn(tx, &f).await;
//		tx = tx_returned;

//		match res {
//			// TODO: Hack. We should be downcasting the error.
//			Err(WorkerError::Internal { ty, message, .. })
//				if ty == "sqlx_core::error::Error::Database"
//					&& message.contains("TransactionRetryWithProtoRefreshError") =>
//			{
//				tracing::info!(%message, "transaction retry");
//				sqlx::query("ROLLBACK TO SAVEPOINT cockroach_restart")
//					.execute(&mut tx)
//					.await?;
//			}
//			Err(err) => {
//				tx.rollback().await?;
//				return Err(err);
//			}
//			Ok(x) => {
//				tx.commit().await?;
//				return Ok(x);
//			}
//		}
//	}

//	bail!("transaction failed with retry too many times");
//}

///// Executes the transaction inside of `tx. Anything in this function can throw
///// a retry error and will be retired accordingly.
//#[tracing::instrument(skip_all)]
//async fn execute_fn<'a, T, F>(
//	mut tx: sqlx::Transaction<'a, sqlx::Postgres>,
//	f: &F,
//) -> (sqlx::Transaction<'a, sqlx::Postgres>, WorkerResult<T>)
//where
//	for<'t> F: Fn(&'t mut sqlx::Transaction<'_, sqlx::Postgres>) -> AsyncResult<'t, T>,
//{
//	let res = f(&mut tx).await;

//	if res.is_ok() {
//		// Attempt to apply savepoint. This can also throw a retry error.
//		let res = sqlx::query("RELEASE SAVEPOINT cockroach_restart")
//			.execute(&mut tx)
//			.await;
//		match res {
//			Ok(_) => {}
//			Err(err) => return (tx, Err(err.into())),
//		}
//	}

//	(tx, res)
//}

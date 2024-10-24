use std::future::Future;

use futures_util::future;

/// Attempts to create a new future to select over a list of futures.
/// Non-panicking version of [futures_util::future::select_all](https://docs.rs/futures/0.3.15/futures/future/fn.select_all.html).
///
/// If `iter` is empty, a `Pending` future is returned.
pub async fn select_all_or_wait<I>(iter: I) -> <I::Item as Future>::Output
where
	I: IntoIterator,
	I::Item: Future + Unpin,
{
	let futs = iter.into_iter().collect::<Vec<I::Item>>();

	if !futs.is_empty() {
		future::select_all(futs).await.0
	} else {
		std::future::pending().await
	}
}

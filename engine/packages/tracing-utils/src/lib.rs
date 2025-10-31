use std::{
	future::Future,
	pin::Pin,
	task::{Context, Poll},
	time::Instant,
};

use futures_util::future;
use tracing::{Instrument, instrument::Instrumented};

use rivet_metrics::KeyValue;

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

pub trait CustomInstrumentExt: Sized {
	fn custom_instrument(self, span: tracing::Span) -> CustomInstrumented<Self> {
		CustomInstrumented {
			inner: self.instrument(span),
			start: Instant::now(),
		}
	}
}

impl<F: Sized> CustomInstrumentExt for F {}

pub struct CustomInstrumented<T> {
	inner: Instrumented<T>,
	start: Instant,
}

impl<T: Future> Future for CustomInstrumented<T> {
	type Output = T::Output;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let this = unsafe { self.get_unchecked_mut() };
		let inner = unsafe { Pin::new_unchecked(&mut this.inner) };

		let metadata = inner.span().metadata().clone();

		match inner.poll(cx) {
			Poll::Ready(val) => {
				if let Some(metadata) = metadata {
					if let (Some(file), Some(line)) = (metadata.file(), metadata.line()) {
						metrics::INSTRUMENTED_FUTURE_DURATION.record(
							this.start.elapsed().as_secs_f64(),
							&[
								KeyValue::new("location", format!("{file}:{line}")),
								KeyValue::new("name", metadata.name()),
							],
						);
					}
				}
				Poll::Ready(val)
			}
			Poll::Pending => Poll::Pending,
		}
	}
}

mod metrics {
	use rivet_metrics::{
		MICRO_BUCKETS,
		otel::{global::*, metrics::*},
	};

	lazy_static::lazy_static! {
		static ref METER: Meter = meter("rivet-util-core");

		/// Expected attributes: "location", "name"
		pub static ref INSTRUMENTED_FUTURE_DURATION: Histogram<f64> = METER.f64_histogram("rivet_instrumented_future_duration")
			.with_description("Duration of a future.")
			.with_boundaries(MICRO_BUCKETS.to_vec())
			.build();
	}
}

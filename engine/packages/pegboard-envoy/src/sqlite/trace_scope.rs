//! SQLite diagnostic context across polls and cancellation.
use std::{
	future::Future,
	pin::Pin,
	task::{Context, Poll},
};

use futures_util::future::Either;

/// Runs `future` under `span` while a subscriber records it. A disabled span hands the request
/// future back as it is, so it neither boxes nor retains any subscriber. Enabled capture owns
/// one bounded request context and restores it while polling AND dropping the request.
///
/// This is a plain function on purpose. An `async fn` wrapper keeps the request it was handed
/// in its own state beside the copy it awaits, so each nested wrapper doubles the request's
/// size; the request path nests three of them, and unoptimized builds then overflow the thread
/// stack on actor start.
pub(crate) fn scope<F: Future>(span: tracing::Span, future: F) -> Either<F, Scoped<F>> {
	match span.with_subscriber(|(_, dispatch)| dispatch.clone()) {
		None => Either::Left(future),
		Some(dispatch) => Either::Right(Scoped {
			future: Some(Box::pin(future)),
			span,
			dispatch,
		}),
	}
}

pub(crate) struct Scoped<F> {
	future: Option<Pin<Box<F>>>,
	span: tracing::Span,
	dispatch: tracing::Dispatch,
}
impl<F: Future> Future for Scoped<F> {
	type Output = F::Output;
	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let this = self.get_mut();
		tracing::dispatcher::with_default(&this.dispatch, || {
			this.span.in_scope(|| {
				this.future
					.as_mut()
					.expect("owned until drop")
					.as_mut()
					.poll(cx)
			})
		})
	}
}
impl<F> Drop for Scoped<F> {
	fn drop(&mut self) {
		// WithDispatch only changes polling. An Instrumented inner future also enters
		// its span during Drop, so dropping outside this scope can mix registries.
		tracing::dispatcher::with_default(&self.dispatch, || {
			// Move the span inside the scope before dropping user work. If an inner
			// destructor panics, unwinding still closes this span under its owner.
			let span = std::mem::replace(&mut self.span, tracing::Span::none());
			span.in_scope(|| drop(self.future.take()));
		});
	}
}



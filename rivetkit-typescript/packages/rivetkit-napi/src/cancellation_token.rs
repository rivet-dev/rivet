use napi::JsFunction;
use napi::threadsafe_function::{
	ErrorStrategy, ThreadSafeCallContext, ThreadsafeFunction, ThreadsafeFunctionCallMode,
};
use napi_derive::napi;
use tokio_util::sync::CancellationToken as CoreCancellationToken;

#[napi]
pub struct CancellationToken {
	inner: CoreCancellationToken,
}

impl CancellationToken {
	pub(crate) fn new(inner: CoreCancellationToken) -> Self {
		tracing::debug!(class = "CancellationToken", "constructed napi class");
		Self { inner }
	}

	pub(crate) fn inner(&self) -> &CoreCancellationToken {
		&self.inner
	}
}

#[napi]
impl CancellationToken {
	#[napi(constructor)]
	pub fn constructor() -> Self {
		Self::new(CoreCancellationToken::new())
	}

	#[napi]
	pub fn aborted(&self) -> bool {
		self.inner.is_cancelled()
	}

	#[napi]
	pub fn cancel(&self) {
		tracing::debug!(
			class = "CancellationToken",
			"abort signal cancelled native cancellation token"
		);
		self.inner.cancel();
	}

	/// Calls `callback` once when the token is cancelled.
	///
	/// The returned listener must be released when the caller stops caring
	/// about cancellation. Releasing ends the waiting task and frees the
	/// threadsafe function, which otherwise stays alive (with its libuv handle
	/// and JS closure) until the token is cancelled.
	#[napi]
	pub fn on_cancelled(&self, callback: JsFunction) -> napi::Result<CancellationListener> {
		let token = self.inner.clone();
		let released = CoreCancellationToken::new();
		let listener = CancellationListener {
			released: released.clone(),
		};
		let tsfn: ThreadsafeFunction<(), ErrorStrategy::CalleeHandled> = callback
			.create_threadsafe_function(0, |_ctx: ThreadSafeCallContext<()>| {
				Ok(Vec::<napi::JsUnknown>::new())
			})?;

		napi::bindgen_prelude::spawn(async move {
			tokio::select! {
				biased;
				_ = released.cancelled() => {
					tracing::debug!(
						kind = "cancellationToken.onCancelled",
						"cancellation listener released before cancellation"
					);
				}
				_ = token.cancelled() => {
					tracing::debug!(
						kind = "cancellationToken.onCancelled",
						payload_summary = "cancelled=true",
						"invoking napi TSF callback"
					);
					let status = tsfn.call(Ok(()), ThreadsafeFunctionCallMode::NonBlocking);
					tracing::debug!(
						kind = "cancellationToken.onCancelled",
						?status,
						"napi TSF callback returned"
					);
					if status != napi::Status::Ok {
						tracing::warn!(?status, "failed to deliver cancellation callback");
					}
				}
			}
		});

		Ok(listener)
	}
}

/// Registration returned by `CancellationToken.onCancelled`.
///
/// Dropping the JS object does not release the listener, because the callback
/// must still fire if the caller keeps no reference to the registration.
#[napi]
pub struct CancellationListener {
	released: CoreCancellationToken,
}

#[napi]
impl CancellationListener {
	/// Stops waiting for cancellation without calling the callback.
	/// Idempotent, and a no-op after the callback has fired.
	#[napi]
	pub fn release(&self) {
		self.released.cancel();
	}
}

impl Drop for CancellationToken {
	fn drop(&mut self) {
		tracing::debug!(class = "CancellationToken", "dropped napi class");
	}
}

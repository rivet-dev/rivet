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

	#[napi]
	pub fn on_cancelled(&self, callback: JsFunction) -> napi::Result<()> {
		let token = self.inner.clone();
		let tsfn: ThreadsafeFunction<(), ErrorStrategy::CalleeHandled> = callback
			.create_threadsafe_function(0, |_ctx: ThreadSafeCallContext<()>| {
				Ok(Vec::<napi::JsUnknown>::new())
			})?;

		napi::bindgen_prelude::spawn(async move {
			token.cancelled().await;
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
		});

		Ok(())
	}
}

impl Drop for CancellationToken {
	fn drop(&mut self) {
		tracing::debug!(class = "CancellationToken", "dropped napi class");
	}
}

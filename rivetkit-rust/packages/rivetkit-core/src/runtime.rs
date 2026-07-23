use std::future::Future;
use std::pin::Pin;

use tokio::task::JoinHandle;

#[cfg(feature = "native-runtime")]
pub type RuntimeBoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

#[cfg(not(any(feature = "native-runtime", feature = "wasm-runtime")))]
pub type RuntimeBoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

#[cfg(feature = "wasm-runtime")]
pub type RuntimeBoxFuture<T> = Pin<Box<dyn Future<Output = T>>>;

#[cfg(feature = "native-runtime")]
pub trait RuntimeFuture: Future + Send + 'static {}

#[cfg(feature = "native-runtime")]
impl<F> RuntimeFuture for F where F: Future + Send + 'static {}

#[cfg(not(any(feature = "native-runtime", feature = "wasm-runtime")))]
pub trait RuntimeFuture: Future + Send + 'static {}

#[cfg(not(any(feature = "native-runtime", feature = "wasm-runtime")))]
impl<F> RuntimeFuture for F where F: Future + Send + 'static {}

#[cfg(feature = "wasm-runtime")]
pub trait RuntimeFuture: Future + 'static {}

#[cfg(feature = "wasm-runtime")]
impl<F> RuntimeFuture for F where F: Future + 'static {}

#[cfg(feature = "native-runtime")]
pub trait RuntimeFutureOutput: Send + 'static {}

#[cfg(feature = "native-runtime")]
impl<T> RuntimeFutureOutput for T where T: Send + 'static {}

#[cfg(not(any(feature = "native-runtime", feature = "wasm-runtime")))]
pub trait RuntimeFutureOutput: Send + 'static {}

#[cfg(not(any(feature = "native-runtime", feature = "wasm-runtime")))]
impl<T> RuntimeFutureOutput for T where T: Send + 'static {}

#[cfg(feature = "wasm-runtime")]
pub trait RuntimeFutureOutput: 'static {}

#[cfg(feature = "wasm-runtime")]
impl<T> RuntimeFutureOutput for T where T: 'static {}

#[derive(Clone, Copy, Debug, Default)]
pub struct RuntimeSpawner;

impl RuntimeSpawner {
	#[cfg(feature = "native-runtime")]
	pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
	where
		F: RuntimeFuture,
		F::Output: RuntimeFutureOutput,
	{
		tokio::spawn(future)
	}

	#[cfg(not(any(feature = "native-runtime", feature = "wasm-runtime")))]
	pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
	where
		F: RuntimeFuture,
		F::Output: RuntimeFutureOutput,
	{
		tokio::spawn(future)
	}

	#[cfg(feature = "wasm-runtime")]
	pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
	where
		F: RuntimeFuture,
		F::Output: RuntimeFutureOutput,
	{
		#[cfg(target_arch = "wasm32")]
		{
			let local = tokio::task::LocalSet::new();
			let handle = local.spawn_local(future);
			wasm_bindgen_futures::spawn_local(async move {
				local.await;
			});
			handle
		}

		#[cfg(not(target_arch = "wasm32"))]
		{
			tokio::task::spawn_local(future)
		}
	}
}

#[cfg(feature = "native-runtime")]
pub fn boxed_runtime_future<F, T>(future: F) -> RuntimeBoxFuture<T>
where
	F: Future<Output = T> + Send + 'static,
{
	Box::pin(future)
}

#[cfg(not(any(feature = "native-runtime", feature = "wasm-runtime")))]
pub fn boxed_runtime_future<F, T>(future: F) -> RuntimeBoxFuture<T>
where
	F: Future<Output = T> + Send + 'static,
{
	Box::pin(future)
}

#[cfg(feature = "wasm-runtime")]
pub fn boxed_runtime_future<F, T>(future: F) -> RuntimeBoxFuture<T>
where
	F: Future<Output = T> + 'static,
{
	Box::pin(future)
}

#[cfg(all(test, feature = "wasm-runtime"))]
mod tests {
	use std::cell::RefCell;
	use std::rc::Rc;

	use super::{RuntimeBoxFuture, boxed_runtime_future};

	fn accepts_wasm_local_callback(
		callback: impl Fn() -> RuntimeBoxFuture<()> + 'static,
	) -> impl Fn() -> RuntimeBoxFuture<()> {
		callback
	}

	#[test]
	fn wasm_runtime_box_future_accepts_local_callbacks() {
		let state = Rc::new(RefCell::new(0));
		let callback = accepts_wasm_local_callback({
			let state = state.clone();
			move || {
				let state = state.clone();
				boxed_runtime_future(async move {
					*state.borrow_mut() += 1;
				})
			}
		});

		let _future = callback();
	}
}

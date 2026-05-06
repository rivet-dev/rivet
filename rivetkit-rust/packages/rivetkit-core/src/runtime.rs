use std::future::Future;
use std::pin::Pin;

use tokio::task::JoinHandle;

#[cfg(rivetkit_native_runtime)]
pub type RuntimeBoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

#[cfg(rivetkit_wasm_runtime)]
pub type RuntimeBoxFuture<T> = Pin<Box<dyn Future<Output = T>>>;

#[cfg(rivetkit_native_runtime)]
pub trait RuntimeFuture: Future + Send + 'static {}

#[cfg(rivetkit_native_runtime)]
impl<F> RuntimeFuture for F where F: Future + Send + 'static {}

#[cfg(rivetkit_wasm_runtime)]
pub trait RuntimeFuture: Future + 'static {}

#[cfg(rivetkit_wasm_runtime)]
impl<F> RuntimeFuture for F where F: Future + 'static {}

#[cfg(rivetkit_native_runtime)]
pub trait RuntimeFutureOutput: Send + 'static {}

#[cfg(rivetkit_native_runtime)]
impl<T> RuntimeFutureOutput for T where T: Send + 'static {}

#[cfg(rivetkit_wasm_runtime)]
pub trait RuntimeFutureOutput: 'static {}

#[cfg(rivetkit_wasm_runtime)]
impl<T> RuntimeFutureOutput for T where T: 'static {}

#[derive(Clone, Copy, Debug, Default)]
pub struct RuntimeSpawner;

impl RuntimeSpawner {
	#[cfg(rivetkit_native_runtime)]
	pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
	where
		F: RuntimeFuture,
		F::Output: RuntimeFutureOutput,
	{
		tokio::spawn(future)
	}

	#[cfg(rivetkit_wasm_runtime)]
	pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
	where
		F: RuntimeFuture,
		F::Output: RuntimeFutureOutput,
	{
		tokio::task::spawn_local(future)
	}
}

#[cfg(rivetkit_native_runtime)]
pub fn boxed_runtime_future<F, T>(future: F) -> RuntimeBoxFuture<T>
where
	F: Future<Output = T> + Send + 'static,
{
	Box::pin(future)
}

#[cfg(rivetkit_wasm_runtime)]
pub fn boxed_runtime_future<F, T>(future: F) -> RuntimeBoxFuture<T>
where
	F: Future<Output = T> + 'static,
{
	Box::pin(future)
}

#[cfg(all(test, rivetkit_wasm_runtime))]
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

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
		tokio::task::spawn_local(future)
	}
}

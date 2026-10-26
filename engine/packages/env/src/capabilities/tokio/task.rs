//! Wraps `tokio::task::JoinHandle` for tasks spawned through `Env`, retaining the
//! spawn location for diagnostics.

pub use ::tokio::task::JoinError;
use ::tokio::task::JoinHandle as TokioJoinHandle;
use futures_util::FutureExt;
use std::panic::Location;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Yield to the owning Tokio scheduler, including the seeded simulation runtime.
/// This creates a cooperative cancellation point without advancing application time.
#[allow(clippy::disallowed_methods)] // Environment facade; both backends use Tokio.
pub async fn yield_now() {
	::tokio::task::yield_now().await;
}

pub struct TaskHandle {
	pub(crate) inner: TokioJoinHandle<()>,
	pub(crate) site: &'static Location<'static>,
}

impl TaskHandle {
	#[track_caller]
	pub fn new(inner: TokioJoinHandle<()>) -> Self {
		Self {
			inner,
			site: Location::caller(),
		}
	}

	pub fn abort(&self) {
		self.inner.abort();
	}

	pub async fn join(self) -> Result<(), JoinError> {
		self.inner.await
	}

	pub fn spawn_site(&self) -> &'static Location<'static> {
		self.site
	}
}

/// Runs a scoped task until its scope is cancelled, checking cancellation first. Spawn
/// wrappers are hand-written futures and combinators rather than `async` blocks because an
/// `async` wrapper reserves space for the wrapped future twice, once as the captured value and
/// once as the awaited one, and every long-lived task pays that for its whole lifetime.
struct Scoped<F> {
	cancelled: tokio_util::sync::WaitForCancellationFutureOwned,
	future: F,
}

impl<F: std::future::Future> std::future::Future for Scoped<F> {
	type Output = Option<F::Output>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		// SAFETY: both fields are structurally pinned. They are never moved out of `Scoped`,
		// which has no `Drop` impl and is `Unpin` only when both fields are.
		let this = unsafe { self.get_unchecked_mut() };
		let cancelled = unsafe { Pin::new_unchecked(&mut this.cancelled) };
		if cancelled.poll(cx).is_ready() {
			return Poll::Ready(None);
		}
		let future = unsafe { Pin::new_unchecked(&mut this.future) };
		future.poll(cx).map(Some)
	}
}

/// Owns a component's tasks. Dropping the scope cancels them; `shutdown` also
/// waits for their futures to be dropped. Join returned handles to observe panics.
///
/// A scope is bound to one `Env` so every task it owns shares that environment's
/// lifetime. In simulation this keeps a component's tasks on its own node, so a
/// node crash cancels exactly the tasks that node owns.
pub struct TaskScope {
	env: std::sync::Arc<dyn crate::Env>,
	cancel: tokio_util::sync::CancellationToken,
	tasks: tokio_util::task::TaskTracker,
}

impl TaskScope {
	pub fn new(env: std::sync::Arc<dyn crate::Env>) -> Self {
		Self {
			env,
			cancel: tokio_util::sync::CancellationToken::new(),
			tasks: tokio_util::task::TaskTracker::new(),
		}
	}

	pub fn env(&self) -> &std::sync::Arc<dyn crate::Env> {
		&self.env
	}

	#[track_caller]
	pub fn spawn(
		&self,
		future: impl std::future::Future<Output = ()> + Send + 'static,
	) -> TaskHandle {
		let task = self.track(future);
		self.env.spawn(Box::pin(task.map(drop)))
	}

	fn track<T, F: std::future::Future<Output = T>>(
		&self,
		future: F,
	) -> impl std::future::Future<Output = Option<T>> + use<T, F> {
		self.tasks.track_future(Scoped {
			cancelled: self.cancel.clone().cancelled_owned(),
			future,
		})
	}

	pub fn cancel(&self) {
		self.cancel.cancel();
	}

	pub async fn shutdown(&self) {
		self.cancel();
		self.tasks.close();
		self.tasks.wait().await;
	}
}

impl Drop for TaskScope {
	fn drop(&mut self) {
		self.cancel();
	}
}

impl std::future::Future for TaskHandle {
	type Output = Result<(), JoinError>;

	fn poll(
		mut self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Self::Output> {
		std::pin::Pin::new(&mut self.inner).poll(cx)
	}
}

impl TaskHandle {
	pub fn is_finished(&self) -> bool {
		self.inner.is_finished()
	}
}

/// A spawned result whose panic and cancellation are observed through the environment task.
pub struct JoinHandle<T> {
	task: TaskHandle,
	result: ::tokio::sync::oneshot::Receiver<T>,
}

#[track_caller]
pub fn spawn<T: Send + 'static>(
	env: &dyn crate::Env,
	future: impl std::future::Future<Output = T> + Send + 'static,
) -> JoinHandle<T> {
	let (send, result) = ::tokio::sync::oneshot::channel();
	let task = env.spawn(Box::pin(future.map(move |value| {
		let _ = send.send(value);
	})));
	JoinHandle { task, result }
}

impl<T> JoinHandle<T> {
	pub fn abort(&self) {
		self.task.abort();
	}

	pub fn is_finished(&self) -> bool {
		self.task.is_finished()
	}
}

impl<T> std::future::Future for JoinHandle<T> {
	type Output = Result<T, JoinError>;

	fn poll(
		mut self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Self::Output> {
		use std::task::Poll;
		match std::pin::Pin::new(&mut self.task).poll(cx) {
			Poll::Pending => Poll::Pending,
			Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
			Poll::Ready(Ok(())) => {
				// A successful task always sends its output before returning.
				Poll::Ready(Ok(self
					.result
					.try_recv()
					.expect("successful task sent its output")))
			}
		}
	}
}

impl TaskScope {
	#[track_caller]
	pub fn spawn_result<T: Send + 'static>(
		&self,
		future: impl std::future::Future<Output = T> + Send + 'static,
	) -> JoinHandle<Option<T>> {
		spawn(self.env.as_ref(), self.track(future))
	}
}

/// Join bounded CPU work through the injected production/simulation adapter.
pub async fn blocking<T: Send + 'static>(
	env: &dyn crate::Env,
	work: impl FnOnce() -> T + Send + 'static,
) -> anyhow::Result<T> {
	let (send, receive) = ::tokio::sync::oneshot::channel();
	let task = env.spawn_blocking(Box::new(move || {
		let _ = send.send(work());
	}))?;
	task.await?;
	receive.await.map_err(Into::into)
}

/// Owns and joins a synchronous adapter that can await injected I/O callbacks.
/// Running adapter work is not abortable; its I/O and computation must be bounded.
pub async fn blocking_io<T: Send + 'static>(
	env: &dyn crate::Env,
	work: impl FnOnce() -> T + Send + 'static,
) -> anyhow::Result<T> {
	let (send, receive) = ::tokio::sync::oneshot::channel();
	let task = env.spawn_blocking_io(Box::new(move || {
		let _ = send.send(work());
	}))?;
	task.await?;
	receive.await.map_err(Into::into)
}
/// Send an owned request to the async task servicing this blocking adapter and
/// wait for its reply through a channel. No runtime is entered on this thread.
/// Call only inside `blocking_io`; ordinary async/CPU tasks fail explicitly.
pub fn request_io<T: Send + 'static>(
	env: &dyn crate::Env,
	future: impl std::future::Future<Output = T> + Send + 'static,
) -> anyhow::Result<T> {
	let (send, receive) = std::sync::mpsc::sync_channel(1);
	env.submit_io(Box::pin(async move {
		let _ = send.send(future.await);
	}))?;
	receive
		.recv()
		.map_err(|_| anyhow::anyhow!("blocking I/O owner ended before replying"))
}



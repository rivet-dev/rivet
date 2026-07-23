use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming as BodyIncoming;
use tokio::sync::mpsc;

pub type ResponseBodyError = Box<dyn std::error::Error + Send + Sync>;

#[doc(hidden)]
pub struct CompletionGuard(Option<Box<dyn FnOnce() + Send + 'static>>);

impl CompletionGuard {
	fn complete(&mut self) {
		if let Some(callback) = self.0.take() {
			callback();
		}
	}
}

impl std::fmt::Debug for CompletionGuard {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CompletionGuard").finish_non_exhaustive()
	}
}

impl Drop for CompletionGuard {
	fn drop(&mut self) {
		self.complete();
	}
}

/// Response body type that can handle both streaming and buffered responses
#[derive(Debug)]
pub enum ResponseBody {
	/// Buffered response body
	Full(Full<Bytes>),
	/// Streaming response body
	Incoming(BodyIncoming),
	/// Channel-backed streaming response body
	Channel(mpsc::Receiver<Result<Bytes, ResponseBodyError>>),
	/// Body carrying a callback that runs at EOF, error, or drop.
	WithCompletion {
		body: Box<ResponseBody>,
		completion: CompletionGuard,
	},
}

impl ResponseBody {
	pub(crate) fn with_completion(self, callback: impl FnOnce() + Send + 'static) -> Self {
		Self::WithCompletion {
			body: Box::new(self),
			completion: CompletionGuard(Some(Box::new(callback))),
		}
	}
}

impl http_body::Body for ResponseBody {
	type Data = Bytes;
	type Error = ResponseBodyError;

	fn poll_frame(
		self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
		match self.get_mut() {
			ResponseBody::Full(body) => {
				let pin = std::pin::Pin::new(body);
				match pin.poll_frame(cx) {
					std::task::Poll::Ready(Some(Ok(frame))) => {
						std::task::Poll::Ready(Some(Ok(frame)))
					}
					std::task::Poll::Ready(Some(Err(e))) => {
						std::task::Poll::Ready(Some(Err(Box::new(e))))
					}
					std::task::Poll::Ready(None) => std::task::Poll::Ready(None),
					std::task::Poll::Pending => std::task::Poll::Pending,
				}
			}
			ResponseBody::Incoming(body) => {
				let pin = std::pin::Pin::new(body);
				match pin.poll_frame(cx) {
					std::task::Poll::Ready(Some(Ok(frame))) => {
						std::task::Poll::Ready(Some(Ok(frame)))
					}
					std::task::Poll::Ready(Some(Err(e))) => {
						std::task::Poll::Ready(Some(Err(Box::new(e))))
					}
					std::task::Poll::Ready(None) => std::task::Poll::Ready(None),
					std::task::Poll::Pending => std::task::Poll::Pending,
				}
			}
			ResponseBody::Channel(rx) => match rx.poll_recv(cx) {
				std::task::Poll::Ready(Some(Ok(bytes))) => {
					std::task::Poll::Ready(Some(Ok(http_body::Frame::data(bytes))))
				}
				std::task::Poll::Ready(Some(Err(err))) => std::task::Poll::Ready(Some(Err(err))),
				std::task::Poll::Ready(None) => std::task::Poll::Ready(None),
				std::task::Poll::Pending => std::task::Poll::Pending,
			},
			ResponseBody::WithCompletion { body, completion } => {
				let result = std::pin::Pin::new(body.as_mut()).poll_frame(cx);
				if matches!(
					&result,
					std::task::Poll::Ready(None)
						| std::task::Poll::Ready(Some(Err(_)))
				) {
					completion.complete();
				}
				result
			}
		}
	}

	fn is_end_stream(&self) -> bool {
		match self {
			ResponseBody::Full(body) => body.is_end_stream(),
			ResponseBody::Incoming(body) => body.is_end_stream(),
			ResponseBody::Channel(rx) => rx.is_closed() && rx.is_empty(),
			ResponseBody::WithCompletion { body, .. } => body.is_end_stream(),
		}
	}

	fn size_hint(&self) -> http_body::SizeHint {
		match self {
			ResponseBody::Full(body) => body.size_hint(),
			ResponseBody::Incoming(body) => body.size_hint(),
			ResponseBody::Channel(_) => http_body::SizeHint::default(),
			ResponseBody::WithCompletion { body, .. } => body.size_hint(),
		}
	}
}

#[cfg(test)]
mod tests {
	use std::sync::{
		Arc,
		atomic::{AtomicUsize, Ordering},
	};

	use bytes::Bytes;
	use http_body_util::{BodyExt, Full};
	use tokio::sync::mpsc;

	use super::{ResponseBody, ResponseBodyError};

	fn completion_counter() -> (Arc<AtomicUsize>, impl FnOnce() + Send + 'static) {
		let count = Arc::new(AtomicUsize::new(0));
		let callback_count = count.clone();
		(count, move || {
			callback_count.fetch_add(1, Ordering::SeqCst);
		})
	}

	#[tokio::test]
	async fn completion_runs_once_at_eof() {
		let (count, callback) = completion_counter();
		let body = ResponseBody::Full(Full::new(Bytes::from_static(b"done")))
			.with_completion(callback);

		assert_eq!(
			body.collect().await.unwrap().to_bytes(),
			Bytes::from_static(b"done")
		);
		assert_eq!(count.load(Ordering::SeqCst), 1);
	}

	#[tokio::test]
	async fn completion_runs_once_on_error() {
		let (tx, rx) = mpsc::channel(1);
		tx.send(Err::<Bytes, ResponseBodyError>("stream failed".into()))
			.await
			.unwrap();
		drop(tx);

		let (count, callback) = completion_counter();
		let mut body = ResponseBody::Channel(rx).with_completion(callback);

		assert!(body.frame().await.unwrap().is_err());
		assert_eq!(count.load(Ordering::SeqCst), 1);
		drop(body);
		assert_eq!(count.load(Ordering::SeqCst), 1);
	}

	#[test]
	fn completion_runs_once_on_drop() {
		let (_tx, rx) = mpsc::channel(1);
		let (count, callback) = completion_counter();
		let body = ResponseBody::Channel(rx).with_completion(callback);

		drop(body);
		assert_eq!(count.load(Ordering::SeqCst), 1);
	}
}

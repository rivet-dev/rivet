use futures::stream::{self, BoxStream};
use futures::StreamExt;
use tokio_util::sync::CancellationToken;

use rivetkit_core::{Queue, QueueMessage, QueueNextOpts};

#[derive(Clone, Debug, Default)]
pub struct QueueStreamOpts {
	pub names: Option<Vec<String>>,
	pub signal: Option<CancellationToken>,
}

pub type QueueStream<'a> = BoxStream<'a, QueueMessage>;

pub trait QueueStreamExt {
	fn stream(&self, opts: QueueStreamOpts) -> QueueStream<'_>;
}

impl QueueStreamExt for Queue {
	fn stream(&self, opts: QueueStreamOpts) -> QueueStream<'_> {
		stream::unfold(
			QueueStreamState {
				queue: self,
				opts,
			},
			|state| async move { state.next().await },
		)
		.boxed()
	}
}

struct QueueStreamState<'a> {
	queue: &'a Queue,
	opts: QueueStreamOpts,
}

impl<'a> QueueStreamState<'a> {
	async fn next(self) -> Option<(QueueMessage, Self)> {
		if self
			.opts
			.signal
			.as_ref()
			.is_some_and(CancellationToken::is_cancelled)
		{
			return None;
		}

		match self
			.queue
			.next(QueueNextOpts {
				names: self.opts.names.clone(),
				timeout: None,
				signal: self.opts.signal.clone(),
				completable: false,
			})
			.await
		{
			Ok(Some(message)) => Some((message, self)),
			Ok(None) => None,
			Err(error) => {
				if self
					.opts
					.signal
					.as_ref()
					.is_some_and(CancellationToken::is_cancelled)
				{
					return None;
				}

				tracing::warn!(?error, "queue stream terminated");
				None
			}
		}
	}
}

#[cfg(test)]
#[path = "../tests/modules/queue.rs"]
mod tests;

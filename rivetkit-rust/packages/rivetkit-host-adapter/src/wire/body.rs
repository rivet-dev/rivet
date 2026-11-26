//! Outbound response bodies (the host streams an HTTP response). Every op on one
//! body (write, end, abort) goes into its FIFO when the host calls, and one task
//! applies them in that order: a later write, `fin` or error can never overtake an
//! earlier one, however the runtime schedules tasks.

use std::sync::Arc;

use anyhow::Result;
use futures::FutureExt;
use tokio::sync::mpsc;

use super::runtime::{Answer, Pending, Wire};
use crate::http::ResponseBody;

enum BodyOp {
	Write {
		data: Vec<u8>,
		fin: bool,
		done: Pending,
	},
	Error {
		message: String,
		done: Pending,
	},
}

/// A response body behind its FIFO. Dropping the last handle ends the queue; the
/// task then aborts the body if the host never ended it.
pub(crate) struct OrderedBody {
	queue: mpsc::UnboundedSender<BodyOp>,
}

impl OrderedBody {
	pub(crate) fn new(wire: &Arc<Wire>, body: ResponseBody) -> Self {
		// Unbounded is safe: the host waits for each op before its next one on that
		// thread, and each write itself waits for room in core's bounded channel.
		let (queue, mut ops) = mpsc::unbounded_channel();
		wire.tokio.spawn(async move {
			while let Some(op) = ops.recv().await {
				// A panic must abort the response, not end it as if it were complete.
				// The op's `Pending`, dropped while unwinding, fails its call.
				let applied = std::panic::AssertUnwindSafe(apply(&body, op))
					.catch_unwind()
					.await;
				if applied.is_err() {
					body.error("response body failed".to_owned()).await;
					// Dropping `ops` fails every op still queued.
					return;
				}
			}
			// No-op when the body already ended or failed.
			body.error("response abandoned by the host".to_owned())
				.await;
		});
		Self { queue }
	}

	pub(crate) fn write(&self, data: Vec<u8>, fin: bool, done: Pending) {
		self.push(BodyOp::Write { data, fin, done });
	}

	pub(crate) fn error(&self, message: String, done: Pending) {
		self.push(BodyOp::Error { message, done });
	}

	fn push(&self, op: BodyOp) {
		// The task only stops after every handle is gone, so this cannot fail while
		// `self` exists; if it did, dropping the op's `Pending` still completes it.
		let _ = self.queue.send(op);
	}
}

async fn apply(body: &ResponseBody, op: BodyOp) {
	match op {
		BodyOp::Write { data, fin, done } => {
			done.complete(write(body, &data, fin).await.map(|()| Answer::empty()));
		}
		BodyOp::Error { message, done } => {
			body.error(message).await;
			done.complete(Ok(Answer::empty()));
		}
	}
}

async fn write(body: &ResponseBody, data: &[u8], fin: bool) -> Result<()> {
	if !data.is_empty() {
		body.write(data).await?;
	}
	if fin {
		body.end().await?;
	}
	Ok(())
}

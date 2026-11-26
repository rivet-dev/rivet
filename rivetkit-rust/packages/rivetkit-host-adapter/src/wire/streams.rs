//! Inbound byte streams (HTTP request bodies, fetch responses) pushed to the host
//! with credit-based backpressure: every chunk is leased, and the next chunk
//! waits until enough leased bytes are released. A host that stops reading stops
//! the source, which stops pulling from the network.

use std::sync::Arc;

use bytes::Bytes;
use futures::StreamExt;
use futures::stream::BoxStream;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use super::EventKind;
use super::messages::{StreamEndEvent, encode};
use super::runtime::{Handle, Wire, error_body};

/// Unreleased bytes allowed per stream.
pub const STREAM_WINDOW: usize = 256 * 1024;

/// A stream id the host can be told about before the first chunk is sent.
pub(crate) struct Reserved {
	pub id: u64,
	owner: u64,
	cancel: CancellationToken,
}

pub(crate) fn reserve(wire: &Arc<Wire>, owner: u64) -> Reserved {
	let cancel = CancellationToken::new();
	let id = wire.insert(owner, Handle::InStream(cancel.clone()));
	Reserved { id, owner, cancel }
}

/// Pumps `source` to the host as `STREAM_CHUNK` events on `reserved.id`, then one
/// `STREAM_END`. Releasing the stream handle cancels it.
pub(crate) fn start(
	wire: &Arc<Wire>,
	actor_type: u64,
	reserved: Reserved,
	mut source: BoxStream<'static, anyhow::Result<Bytes>>,
) {
	let wire = wire.clone();
	let Reserved { id, owner, cancel } = reserved;
	let credit = Arc::new(Semaphore::new(STREAM_WINDOW));
	wire.tokio.clone().spawn(async move {
		let end = loop {
			let next = tokio::select! {
				_ = cancel.cancelled() => return,
				next = source.next() => next,
			};
			let chunk = match next {
				Some(Ok(chunk)) if chunk.is_empty() => continue,
				Some(Ok(chunk)) => chunk,
				Some(Err(error)) => break Some(error_body(&error)),
				None => break None,
			};
			let need = u32::try_from(chunk.len().min(STREAM_WINDOW)).unwrap_or(u32::MAX);
			let permit = tokio::select! {
				_ = cancel.cancelled() => return,
				permit = credit.clone().acquire_many_owned(need) => match permit {
					Ok(permit) => permit,
					Err(_) => return,
				},
			};
			wire.emit_leased(
				EventKind::StreamChunk,
				actor_type,
				owner,
				id,
				&[],
				chunk,
				Some(permit),
			);
		};
		if cancel.is_cancelled() {
			return;
		}
		wire.remove(id);
		wire.notify(
			EventKind::StreamEnd,
			actor_type,
			owner,
			id,
			&encode(&StreamEndEvent { error: end }),
		);
	});
}

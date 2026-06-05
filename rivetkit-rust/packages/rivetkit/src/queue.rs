use std::marker::PhantomData;

use anyhow::{Context as _, Result};
use rivetkit_core::{
	ActorContext, EnqueueAndWaitOpts, QueueMessage, QueueNextBatchOpts, QueueNextOpts,
	QueueTryNextBatchOpts, QueueTryNextOpts,
};
use serde::Serialize;

use crate::actor::Actor;

/// Typed handle over the actor message queue, returned by [`crate::Ctx::queue`].
///
/// This is a thin typed facade over the core queue API. Send helpers CBOR-encode
/// the message body; the `*_raw` variants pass bytes through unchanged. Received
/// [`QueueMessage`] bodies are raw bytes that the caller decodes against their own
/// schema, matching how queued messages arrive through the event loop as
/// `Event::QueueSend`.
pub struct Queue<'a, A: Actor> {
	inner: &'a ActorContext,
	_p: PhantomData<fn() -> A>,
}

impl<'a, A: Actor> Queue<'a, A> {
	pub(crate) fn new(inner: &'a ActorContext) -> Self {
		Self {
			inner,
			_p: PhantomData,
		}
	}

	/// Enqueues a message with a CBOR-encoded body.
	pub async fn send<T: Serialize>(&self, name: &str, body: &T) -> Result<QueueMessage> {
		self.send_raw(name, &encode_cbor(body, "queue message body")?)
			.await
	}

	/// Enqueues a message with a raw byte body.
	pub async fn send_raw(&self, name: &str, body: &[u8]) -> Result<QueueMessage> {
		self.inner.send(name, body).await
	}

	/// Enqueues a message with a CBOR-encoded body and waits for the consumer to
	/// complete it, returning the raw completion response if any.
	pub async fn enqueue_and_wait<T: Serialize>(
		&self,
		name: &str,
		body: &T,
		opts: EnqueueAndWaitOpts,
	) -> Result<Option<Vec<u8>>> {
		self.enqueue_and_wait_raw(name, &encode_cbor(body, "queue message body")?, opts)
			.await
	}

	/// Enqueues a raw-body message and waits for its completion response.
	pub async fn enqueue_and_wait_raw(
		&self,
		name: &str,
		body: &[u8],
		opts: EnqueueAndWaitOpts,
	) -> Result<Option<Vec<u8>>> {
		self.inner.enqueue_and_wait(name, body, opts).await
	}

	/// Awaits the next queued message, optionally bounded by the opts timeout.
	pub async fn next(&self, opts: QueueNextOpts) -> Result<Option<QueueMessage>> {
		self.inner.next(opts).await
	}

	/// Awaits up to `opts.count` queued messages.
	pub async fn next_batch(&self, opts: QueueNextBatchOpts) -> Result<Vec<QueueMessage>> {
		self.inner.next_batch(opts).await
	}

	/// Returns the next queued message if one is immediately available.
	pub fn try_next(&self, opts: QueueTryNextOpts) -> Result<Option<QueueMessage>> {
		self.inner.try_next(opts)
	}

	/// Returns immediately-available queued messages up to `opts.count`.
	pub fn try_next_batch(&self, opts: QueueTryNextBatchOpts) -> Result<Vec<QueueMessage>> {
		self.inner.try_next_batch(opts)
	}

	/// Lists the currently persisted queue messages without consuming them.
	pub async fn inspect_messages(&self) -> Result<Vec<QueueMessage>> {
		self.inner.inspect_messages().await
	}

	/// Returns the configured maximum queue size.
	pub fn max_size(&self) -> u32 {
		self.inner.max_size()
	}
}

fn encode_cbor<T: Serialize>(value: &T, label: &str) -> Result<Vec<u8>> {
	let mut encoded = Vec::new();
	ciborium::into_writer(value, &mut encoded)
		.with_context(|| format!("encode {label} as cbor"))?;
	Ok(encoded)
}

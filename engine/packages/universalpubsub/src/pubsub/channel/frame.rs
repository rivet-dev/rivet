//! Channel frames: length-prefixed versioned messages, written in batches.

use anyhow::Result;
use bytes::{Bytes, BytesMut};
use futures_util::{StreamExt, stream};
use rivet_data::{
	UPS_CHANNEL_VERSION,
	generated::ups_channel_v1::{ChannelMessage, Frame},
	versioned::UpsChannelData,
};
use tokio::sync::mpsc;
use vbare::Versioned;

use crate::{errors, pubsub::stream::HttpBody};

/// One frame, sealed behind its length prefix.
pub(super) fn encode(frame: Frame) -> Result<Bytes> {
	let mut framed = BytesMut::new();
	crate::pubsub::stream::seal_frame::<UpsChannelData>(
		&mut framed,
		&ChannelMessage { frame },
		UPS_CHANNEL_VERSION,
	)?;
	Ok(framed.freeze())
}

/// Decodes in place: payloads and subjects are slices of `bytes`, not copies.
pub(super) fn decode(bytes: Bytes) -> Result<Frame> {
	let message = UpsChannelData::decode(bytes).map_err(|_| errors::InvalidMessage.build())?;
	Ok(message.frame)
}

/// The `Frame::ChannelRequest` union tag.
const REQUEST_TAG: u64 = 1;

/// Frames a request straight from the caller's subject and payload, copying each once into
/// the frame. The bytes are those `encode(Frame::ChannelRequest(..))` writes; building the owned
/// message would first need its own copy of both. A unit test pins the equivalence.
pub(super) fn encode_request(
	id: u64,
	class: rivet_data::generated::ups_request_v1::RequestClass,
	subject: &[u8],
	remaining_ms: u64,
	payload: &[u8],
) -> Bytes {
	use bytes::BufMut;
	let mut framed = BytesMut::with_capacity(48 + subject.len() + payload.len());
	framed.put_u32(0);
	framed.put_u16_le(UPS_CHANNEL_VERSION);
	vbare::varint::put_uint(&mut framed, REQUEST_TAG);
	framed.put_u64_le(id);
	vbare::varint::put_uint(&mut framed, class as u64);
	vbare::varint::put_uint(&mut framed, subject.len() as u64);
	framed.put_slice(subject);
	framed.put_u64_le(remaining_ms);
	vbare::varint::put_uint(&mut framed, payload.len() as u64);
	framed.put_slice(payload);
	let len = (framed.len() - 4) as u32;
	framed[..4].copy_from_slice(&len.to_be_bytes());
	framed.freeze()
}

/// Most bytes a frame adds around its payload: the length prefix, version, tags, id, class,
/// deadline, varints and a subject of up to 1024 bytes. A reader allows a frame this much larger
/// than the largest payload, so a request carrying a full payload and subject always fits.
pub(super) const ENVELOPE_BYTES: usize = 2048;

/// An entry in a channel's write queue.
pub(super) trait Queued: Send + 'static {
	/// The frame to write, or `None` when nobody waits for it any more, so the writer skips it.
	/// Each entry type counts its own skipped frames.
	fn into_frame(self) -> Option<Bytes>;
}

/// The body that writes queued frames. Each write takes everything already queued, up to
/// `max_write_bytes`, and never waits for more: frames batch only when they queued while the
/// previous write was in flight, so an idle channel adds no latency. A frame with nothing queued
/// behind it, or one that would overflow the batch, is written as is without a copy. Abandoned
/// entries are dropped unwritten. The body ends once every queue sender is dropped and the queue is empty.
pub(super) fn batched<T: Queued>(
	mut frames: mpsc::Receiver<T>,
	max_write_bytes: usize,
) -> HttpBody {
	// A frame that would push a batch past `max_write_bytes` waits here for the next write,
	// so it is never copied into a batch.
	let mut carried: Option<Bytes> = None;
	stream::poll_fn(move |cx| {
		let first = loop {
			if let Some(frame) = carried.take() {
				break frame;
			}
			match frames.poll_recv(cx) {
				std::task::Poll::Ready(Some(queued)) => {
					if let Some(frame) = queued.into_frame() {
						break frame;
					}
				}
				std::task::Poll::Ready(None) => return std::task::Poll::Ready(None),
				std::task::Poll::Pending => return std::task::Poll::Pending,
			}
		};
		if first.len() >= max_write_bytes {
			crate::metrics::CHANNEL_FRAMES_PER_WRITE.observe(1.0);
			return std::task::Poll::Ready(Some(Ok(first)));
		}
		let mut written: Option<BytesMut> = None;
		let mut count = 1u64;
		while written.as_ref().map_or(first.len(), BytesMut::len) < max_write_bytes {
			let frame = match frames.try_recv() {
				Ok(queued) => match queued.into_frame() {
					Some(frame) => frame,
					None => continue,
				},
				Err(_) => break,
			};
			let batched_len = written.as_ref().map_or(first.len(), BytesMut::len);
			if batched_len + frame.len() > max_write_bytes {
				carried = Some(frame);
				break;
			}
			let buffer = written.get_or_insert_with(|| {
				let mut buffer = BytesMut::with_capacity(first.len() + frame.len());
				buffer.extend_from_slice(&first);
				buffer
			});
			buffer.extend_from_slice(&frame);
			count += 1;
		}
		crate::metrics::CHANNEL_FRAMES_PER_WRITE.observe(count as f64);
		let written = match written {
			Some(buffer) => buffer.freeze(),
			None => first,
		};
		std::task::Poll::Ready(Some(Ok(written)))
	})
	.boxed()
}

/// Bytes waiting in one channel's write queue, against a cap. A frame larger than the cap is
/// still accepted into an empty queue, so every request up to the message limit has a path.
pub(super) struct ByteBudget {
	used: std::sync::atomic::AtomicUsize,
	capacity: usize,
	/// Counts reserved bytes. Budgets nested inside another leave it to the outer one, so the
	/// gauge counts each queued byte once.
	metric: Option<&'static rivet_metrics::prometheus::IntGauge>,
	/// Wakes reservations waiting for room; notified only while `waiters` is non-zero, so a
	/// budget nobody waits on pays nothing on release.
	released: tokio::sync::Notify,
	waiters: std::sync::atomic::AtomicUsize,
}

impl ByteBudget {
	pub(super) fn new(
		capacity: usize,
		metric: Option<&'static rivet_metrics::prometheus::IntGauge>,
	) -> std::sync::Arc<Self> {
		std::sync::Arc::new(Self {
			used: std::sync::atomic::AtomicUsize::new(0),
			capacity,
			metric,
			released: tokio::sync::Notify::new(),
			waiters: std::sync::atomic::AtomicUsize::new(0),
		})
	}

	/// Reserves `len` bytes until the returned guard drops, or refuses when the queue is full.
	pub(super) fn try_reserve(self: &std::sync::Arc<Self>, len: usize) -> Option<ByteReservation> {
		use std::sync::atomic::Ordering;
		// SeqCst pairs with the release in `ByteReservation::drop`: a waiter registers and then
		// reads `used`, a release lowers `used` and then reads `waiters`, and only a single
		// total order guarantees at least one of them sees the other.
		let mut used = self.used.load(Ordering::SeqCst);
		loop {
			if used > 0 && used.saturating_add(len) > self.capacity {
				return None;
			}
			match self.used.compare_exchange_weak(
				used,
				used.saturating_add(len),
				Ordering::SeqCst,
				Ordering::SeqCst,
			) {
				Ok(_) => return Some(self.reservation(len)),
				Err(current) => used = current,
			}
		}
	}

	/// Reserves `len` bytes, waiting for room as queued frames leave. The caller bounds the
	/// wait with its own deadline.
	pub(super) async fn reserve(self: &std::sync::Arc<Self>, len: usize) -> ByteReservation {
		loop {
			let released = self.released.notified();
			tokio::pin!(released);
			// Register before checking, so a release between the check and the wait still
			// wakes this reservation.
			released.as_mut().enable();
			let _waiting = WaiterGuard::new(&self.waiters);
			if let Some(reserved) = self.try_reserve(len) {
				return reserved;
			}
			released.await;
		}
	}

	/// Whether new work must be refused at admission: the budget is used up, or a reservation
	/// is already waiting for room it did not find.
	pub(super) fn is_full(&self) -> bool {
		use std::sync::atomic::Ordering;
		self.waiters.load(Ordering::Acquire) > 0
			|| self.used.load(Ordering::Acquire) >= self.capacity
	}

	fn reservation(self: &std::sync::Arc<Self>, len: usize) -> ByteReservation {
		ByteReservation {
			budget: self.clone(),
			len,
			_metric: self
				.metric
				.map(|gauge| rivet_metrics::GaugeGuardExt::add_guard(gauge, len as i64)),
		}
	}

	
}

/// Bytes held in a channel's write queue until the frame leaves it.
pub(super) struct ByteReservation {
	budget: std::sync::Arc<ByteBudget>,
	len: usize,
	/// Counts `len` on the budget's gauge until the frame leaves the queue.
	_metric: Option<rivet_metrics::IntGaugeGuard>,
}

impl Drop for ByteReservation {
	fn drop(&mut self) {
		use std::sync::atomic::Ordering;
		self.budget.used.fetch_sub(self.len, Ordering::SeqCst);
		if self.budget.waiters.load(Ordering::SeqCst) > 0 {
			self.budget.released.notify_waiters();
		}
	}
}

/// Counts one reservation waiting for room until it stops waiting, including by cancellation.
struct WaiterGuard<'a>(&'a std::sync::atomic::AtomicUsize);

impl<'a> WaiterGuard<'a> {
	fn new(waiters: &'a std::sync::atomic::AtomicUsize) -> Self {
		waiters.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
		Self(waiters)
	}
}

impl Drop for WaiterGuard<'_> {
	fn drop(&mut self) {
		self.0.fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
	}
}



/// The error a remote handler's failure code stands for. Unknown codes are transport errors.
pub(super) fn error_from_code(code: &str) -> anyhow::Error {
	match code {
		"no_responders" => errors::NoResponders.build(),
		"unknown_worker" => errors::UnknownWorker.build(),
		"wrong_worker" => errors::WrongWorker.build(),
		"timeout" => errors::Timeout.build(),
		"overloaded" => errors::Overloaded.build(),
		"consumer_closed" => errors::ConsumerClosed.build(),
		"invalid_message" => errors::InvalidMessage.build(),
		"unexpected_packet" => errors::UnexpectedPacket.build(),
		"invalid_stream" => errors::InvalidStream.build(),
		"dispatch_rejected" => errors::DispatchRejected.build(),
		_ => errors::Transport.build(),
	}
}

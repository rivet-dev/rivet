use std::{future::Future, sync::Arc, task::Poll};

use anyhow::Result;
use bytes::BufMut;
use bytes::{Bytes, BytesMut};
use futures_util::{
	StreamExt,
	stream::{self, BoxStream},
};
use rivet_data::{
	UPS_PACKET_VERSION,
	generated::ups_packet_v1::{Body, BodyRef, Data},
	versioned::UpsPacketData,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use vbare::Versioned;

use super::StreamControl;
use crate::{
	errors,
	metrics::{Buffer, BufferUsage},
};

pub(crate) type HttpBody = BoxStream<'static, Result<Bytes>>;

#[derive(Clone)]
pub(super) struct Wait {
	pub deadline: Option<u64>,
	pub clock: Arc<dyn rivet_env::Clock>,
	pub stopped: CancellationToken,
}

impl Wait {
	pub async fn run<T>(&self, future: impl Future<Output = Result<T>>) -> Result<T> {
		let now = self.clock.observe()?.boot_ms;
		if self.deadline.is_some_and(|deadline| now >= deadline) {
			return Err(errors::Timeout.build());
		}
		if self.stopped.is_cancelled() {
			return Err(errors::ConsumerClosed.build());
		}

		// Most packet sends and many receives are ready on their first poll. Polling once before
		// arming the deadline timer saves the timer's allocation, and the checks above keep the
		// same precedence: an expired deadline or a shutdown still wins over a ready packet.
		let mut future = std::pin::pin!(future);
		let first = std::future::poll_fn(|cx| Poll::Ready(future.as_mut().poll(cx))).await;
		if let Poll::Ready(result) = first {
			return result;
		}

		let timeout = async {
			if let Some(deadline) = self.deadline {
				self.clock.sleep_until(deadline).await
			} else {
				std::future::pending().await
			}
		};

		rivet_env::select_biased!("expired deadlines and shutdown win over ready packets",
			result = timeout => match result {
				Ok(()) => Err(errors::Timeout.build()),
				Err(error) => Err(error.into()),
			},
			_ = self.stopped.cancelled() => Err(errors::ConsumerClosed.build()),
			result = future.as_mut() => result,
		)
	}
}

struct QueuedPacket {
	bytes: Bytes,
	terminal: bool,
	_usage: BufferUsage,
}

/// Sending waits for packet-buffer capacity. Split a duplex stream when sending
/// and receiving concurrently; neither direction needs the other to finish first.
pub struct PacketSender {
	tx: mpsc::Sender<QueuedPacket>,
	usage: Arc<Buffer>,
	pub(super) control: Arc<StreamControl>,
	finished: bool,
}

impl PacketSender {
	pub async fn send(&mut self, packet: impl Into<Bytes>) -> Result<()> {
		let packet = packet.into();
		let bytes = self.encode_packet(&packet)?;
		let result = self
			.control
			.wait
			.run(async {
				let permit = self
					.tx
					.reserve()
					.await
					.map_err(|_| errors::ConsumerClosed.build())?;
				permit.send(QueuedPacket {
					bytes,
					terminal: false,
					_usage: self.usage.acquire(1),
				});
				Ok(())
			})
			.await;
		self.control.observe_error(result.as_ref().err()).await;
		if result.is_ok() {
			record_packet("sent", packet.len());
		}
		result
	}

	/// Ends this direction without closing the receive half. Dropping without
	/// finishing is an error for the receiver, never a successful empty response.
	pub async fn finish(self) -> Result<()> {
		self.terminal(Body::End).await
	}

	pub(in crate::pubsub) async fn fail(self, error: &anyhow::Error) -> Result<()> {
		self.terminal(Body::Error(
			rivet_error::RivetError::extract(error)
				.code()
				.to_owned()
				.into(),
		))
		.await
	}

	async fn terminal(mut self, body: Body) -> Result<()> {
		let bytes = encode(body)?;
		let result = self
			.control
			.wait
			.run(async {
				let permit = self
					.tx
					.reserve()
					.await
					.map_err(|_| errors::ConsumerClosed.build())?;
				permit.send(QueuedPacket {
					bytes,
					terminal: true,
					_usage: self.usage.acquire(1),
				});
				Ok(())
			})
			.await;
		self.finished = result.is_ok();
		self.control.observe_error(result.as_ref().err()).await;
		result
	}

	pub(in crate::pubsub) fn reply(mut self, packet: &[u8]) -> Result<()> {
		self.queue_packet_and_end(packet)
	}

	pub(in crate::pubsub) fn queue_packet_and_end(&mut self, packet: &[u8]) -> Result<()> {
		let result = (|| {
			self.validate_packet(packet)?;
			let mut bytes = BytesMut::with_capacity(packet_frame_len(packet.len()) + END_FRAME_LEN);
			encode_packet_into(&mut bytes, packet);
			encode_into(&mut bytes, Body::End)?;

			// A stalled caller may not have polled its timeout yet. Check the actual clock again
			// after encoding, rather than relying on channel closure as deadline enforcement.
			let now = self.control.wait.clock.observe()?.boot_ms;
			if self
				.control
				.wait
				.deadline
				.is_some_and(|deadline| now >= deadline)
			{
				return Err(errors::Timeout.build());
			}
			if self.control.wait.stopped.is_cancelled() {
				return Err(errors::ConsumerClosed.build());
			}
			let permit = self.tx.try_reserve().map_err(|error| match error {
				mpsc::error::TrySendError::Full(_) => errors::Overloaded.build(),
				mpsc::error::TrySendError::Closed(_) => errors::ConsumerClosed.build(),
			})?;
			permit.send(QueuedPacket {
				bytes: bytes.freeze(),
				terminal: true,
				_usage: self.usage.acquire(1),
			});
			record_packet("sent", packet.len());
			Ok(())
		})();
		if let Err(error) = &result
			&& let Ok(permit) = self.tx.try_reserve()
		{
			let bytes = encode(Body::Error(
				rivet_error::RivetError::extract(error)
					.code()
					.to_owned()
					.into(),
			))?;
			permit.send(QueuedPacket {
				bytes,
				terminal: true,
				_usage: self.usage.acquire(1),
			});
		}
		self.finished = result.is_ok();
		self.control.record(&result);
		result
	}

	fn validate_packet(&self, packet: &[u8]) -> Result<()> {
		if packet.len() > self.control.max_packet_bytes {
			return Err(errors::InvalidMessage.build());
		}
		Ok(())
	}

	fn encode_packet(&self, packet: &[u8]) -> Result<Bytes> {
		self.validate_packet(packet)?;
		let mut bytes = BytesMut::with_capacity(packet_frame_len(packet.len()));
		encode_packet_into(&mut bytes, packet);
		Ok(bytes.freeze())
	}

	/// The caller's deadline on this host's monotonic clock, as propagated with the stream.
	/// `None` means the caller set no deadline.
	pub fn deadline_boot_ms(&self) -> Option<u64> {
		self.control.wait.deadline
	}

	pub(in crate::pubsub) fn is_closed(&self) -> Result<bool> {
		let now = self.control.wait.clock.observe()?.boot_ms;
		Ok(self.tx.is_closed()
			|| self.control.wait.stopped.is_cancelled()
			|| self
				.control
				.wait
				.deadline
				.is_some_and(|deadline| now >= deadline))
	}
}

impl Drop for PacketSender {
	fn drop(&mut self) {
		if !self.finished {
			self.control
				.record::<()>(&Err(errors::ConsumerClosed.build()));
		}
	}
}

pub struct PacketReceiver {
	pub(super) reader: Reader,
	pub(super) control: Arc<StreamControl>,
	ended: bool,
	ending: bool,
}

impl PacketReceiver {
	pub(super) fn new(body: HttpBody, control: Arc<StreamControl>) -> Self {
		Self {
			reader: Reader::new(body),
			control,
			ended: false,
			ending: false,
		}
	}

	pub async fn recv(&mut self) -> Result<Option<Bytes>> {
		if self.ended {
			return Ok(None);
		}

		let result = self
			.control
			.wait
			.run(async {
				if self.ending {
					self.reader.expect_eof().await?;
					return Ok(None);
				}
				let bytes = self
					.reader
					.frame(self.control.max_packet_bytes + 32)
					.await?;
				// The returned packet is a slice of the frame buffer; nothing is copied.
				let frame =
					UpsPacketData::open(bytes).map_err(|_| errors::InvalidStream.build())?;
				let packet = match frame.get().body() {
					BodyRef::Packet(packet) => {
						if packet.len() > self.control.max_packet_bytes {
							return Err(errors::InvalidMessage.build());
						}
						frame.as_bytes().slice_ref(packet)
					}
					BodyRef::End => {
						self.ending = true;
						self.reader.expect_eof().await?;
						return Ok(None);
					}
					BodyRef::Error(code) => return Err(remote_error(code)),
				};
				record_packet("received", packet.len());
				Ok(Some(packet))
			})
			.await;
		self.control.observe_error(result.as_ref().err()).await;
		if !matches!(&result, Ok(Some(_))) {
			self.ended = true;
		}
		result
	}

	/// Requires one packet and a clean end; zero or extra packets are protocol errors.
	pub async fn receive_one(mut self) -> Result<Bytes> {
		let Some(packet) = self.recv().await? else {
			let result = Err(errors::UnexpectedPacket.build());
			self.control.record(&result);
			return result;
		};
		if self.recv().await?.is_some() {
			let result = Err(errors::UnexpectedPacket.build());
			self.control.record(&result);
			return result;
		}
		Ok(packet)
	}
}

impl Drop for PacketReceiver {
	fn drop(&mut self) {
		if !self.ended {
			self.control
				.record::<()>(&Err(errors::ConsumerClosed.build()));
		}
	}
}

pub(super) fn channel(control: Arc<StreamControl>, capacity: usize) -> (PacketSender, HttpBody) {
	let (tx, rx) = mpsc::channel::<QueuedPacket>(capacity);
	let wait = control.wait.clone();
	// Drain includes response bytes still waiting for the HTTP transport.
	let incoming = control._incoming.clone();
	let body = stream::unfold(
		(rx, wait, false, incoming),
		|(mut rx, wait, ended, incoming)| async move {
			if ended {
				return None;
			}
			let result = wait
				.run(async {
					rx.recv()
						.await
						.ok_or_else(|| errors::ConsumerClosed.build())
				})
				.await;
			let (bytes, terminal) = match result {
				Ok(packet) => (Ok(packet.bytes), packet.terminal),
				Err(error) => (
					encode(Body::Error(
						rivet_error::RivetError::extract(&error)
							.code()
							.to_owned()
							.into(),
					)),
					true,
				),
			};
			Some((bytes, (rx, wait, terminal, incoming)))
		},
	)
	.boxed();
	let sender = PacketSender {
		tx,
		usage: Buffer::new("stream_packets", capacity),
		control,
		finished: false,
	};
	(sender, body)
}

/// Writes a 4-byte big-endian length prefix, then the versioned message sealed directly behind
/// it, so the message is encoded once into the frame buffer. The prefix is filled in after
/// sealing because an older version's length is only known once it is encoded; the reservation
/// is exact at the latest version.
pub(in crate::pubsub) fn seal_frame<V: Versioned>(
	framed: &mut BytesMut,
	value: &V::Latest,
	version: u16,
) -> Result<()> {
	framed.reserve(4 + V::sealed_len(value));
	let prefix_at = framed.len();
	framed.put_u32(0);
	V::seal(value, version, framed)?;

	let len = u32::try_from(framed.len() - prefix_at - 4)?;
	framed[prefix_at..prefix_at + 4].copy_from_slice(&len.to_be_bytes());
	Ok(())
}

const VERSION_HEADER_LEN: usize = 2;

/// The `Body::Packet` union tag.
const PACKET_TAG: u64 = 0;

/// The framed size of an `End` packet: length prefix, version header and union tag.
const END_FRAME_LEN: usize = 4 + VERSION_HEADER_LEN + 1;

/// The framed size of a data packet carrying `packet_len` bytes.
fn packet_frame_len(packet_len: usize) -> usize {
	4 + packet_body_len(packet_len)
}

/// The versioned `Data { body: Body::Packet(..) }` message length for a packet of `packet_len`
/// bytes: version header, union tag, data length and the bytes.
fn packet_body_len(packet_len: usize) -> usize {
	VERSION_HEADER_LEN + 1 + uint_len(packet_len as u64) + packet_len
}

fn uint_len(value: u64) -> usize {
	let bits = (u64::BITS - value.leading_zeros()).max(1) as usize;
	bits.div_ceil(7)
}

/// Frames a data packet straight from a borrowed slice, copying it once into `framed`.
///
/// The bytes are those `seal_frame::<UpsPacketData>` writes for `Body::Packet`; building the
/// owned message would first need its own copy of the slice. The packet unit tests pin the
/// equivalence.
fn encode_packet_into(framed: &mut BytesMut, packet: &[u8]) {
	framed.reserve(packet_frame_len(packet.len()));
	framed.put_u32(packet_body_len(packet.len()) as u32);
	framed.put_u16_le(UPS_PACKET_VERSION);
	vbare::varint::put_uint(framed, PACKET_TAG);
	vbare::varint::put_uint(framed, packet.len() as u64);
	framed.put_slice(packet);
}

fn encode(body: Body) -> Result<Bytes> {
	let mut framed = BytesMut::new();
	encode_into(&mut framed, body)?;
	Ok(framed.freeze())
}

fn encode_into(framed: &mut BytesMut, body: Body) -> Result<()> {
	seal_frame::<UpsPacketData>(framed, &Data { body }, UPS_PACKET_VERSION)
}

pub(in crate::pubsub) struct Reader {
	body: HttpBody,
	chunk: Bytes,
	buffered: BytesMut,
	frame_len: Option<usize>,
}

impl Reader {
	pub fn new(body: HttpBody) -> Self {
		Self {
			body,
			chunk: Bytes::new(),
			buffered: BytesMut::new(),
			frame_len: None,
		}
	}

	pub async fn frame(&mut self, max: usize) -> Result<Bytes> {
		// Keep partial framing in the reader so cancelling recv() does not lose bytes.
		let len = match self.frame_len {
			Some(len) => len,
			None => {
				let prefix = self.exact(4).await?;
				let len = u32::from_be_bytes(prefix[..].try_into().unwrap()) as usize;
				if len == 0 || len > max {
					return Err(errors::InvalidMessage.build());
				}
				self.frame_len = Some(len);
				len
			}
		};
		let frame = self.exact(len).await?;
		self.frame_len = None;
		Ok(frame)
	}

	async fn exact(&mut self, len: usize) -> Result<Bytes> {
		// A frame wholly inside the current chunk is returned as a slice of it, without a copy.
		if self.buffered.is_empty() && self.chunk.len() >= len {
			return Ok(self.chunk.split_to(len));
		}
		// A frame that spans chunks is gathered into the reader's own buffer. Once the previous
		// gathered frame is dropped the buffer is unique again, so growing it reuses that
		// allocation. Reserving the frame length up front would allocate anew whenever the
		// previous frame is still alive, sized by the old buffer's capacity.
		while self.buffered.len() < len {
			if self.chunk.is_empty() {
				self.chunk = self
					.body
					.next()
					.await
					.ok_or_else(|| errors::InvalidStream.build())??;
				continue;
			}
			let take = (len - self.buffered.len()).min(self.chunk.len());
			self.buffered.extend_from_slice(&self.chunk.split_to(take));
		}
		Ok(self.buffered.split().freeze())
	}

	async fn expect_eof(&mut self) -> Result<()> {
		if !self.chunk.is_empty() {
			return Err(errors::InvalidStream.build());
		}
		while let Some(chunk) = self.body.next().await {
			if !chunk?.is_empty() {
				return Err(errors::InvalidStream.build());
			}
		}
		Ok(())
	}

	pub fn into_body(self) -> HttpBody {
		stream::once(async move { Ok(self.chunk) })
			.chain(self.body)
			.boxed()
	}
}

fn remote_error(code: &str) -> anyhow::Error {
	match code {
		"overloaded" => errors::Overloaded.build(),
		"unexpected_packet" => errors::UnexpectedPacket.build(),
		"invalid_message" => errors::InvalidMessage.build(),
		"timeout" => errors::Timeout.build(),
		"consumer_closed" => errors::ConsumerClosed.build(),
		_ => errors::InvalidStream.build(),
	}
}

fn record_packet(direction: &str, bytes: usize) {
	crate::metrics::STREAM_PACKETS
		.with_label_values(&[direction])
		.inc();
	crate::metrics::STREAM_BYTES
		.with_label_values(&[direction])
		.inc_by(bytes as u64);
}





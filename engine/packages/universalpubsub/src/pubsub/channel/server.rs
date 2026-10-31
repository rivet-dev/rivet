//! The serving side of peer channels: each request is admitted and dispatched exactly as a
//! stream would be, and answered on the same channel.

use std::sync::Arc;

use axum::{
	body::Body,
	extract::{Request, State},
	http::{StatusCode, Version},
	response::{IntoResponse, Response},
};
use bytes::Bytes;
use futures_util::StreamExt;
use rivet_data::generated::ups_channel_v1::{
	ChannelBody, ChannelError, ChannelOpen, ChannelRequest, ChannelResponse, Frame,
};
use rivet_error::RivetError;
use tokio::sync::mpsc;

use super::frame;
use crate::{
	errors,
	pubsub::{PubSub, WorkerId, class::RequestClass, stream::Reader},
};

/// The handler map's minimum capacity: a few dozen requests in flight on one channel.
const HANDLER_MAP_CAPACITY: usize = 64;

/// Handlers of one channel's requests, by request id, so a cancel can stop one.
type Handlers = Arc<scc::HashMap<u64, rivet_env::TaskHandle>>;

pub(in crate::pubsub) async fn receive(State(ups): State<PubSub>, request: Request) -> Response {
	if request.version() != Version::HTTP_2 {
		return StatusCode::BAD_REQUEST.into_response();
	}
	let slot = match ups.inner.channels.reserve_incoming() {
		Ok(slot) => slot,
		Err(_) => return StatusCode::TOO_MANY_REQUESTS.into_response(),
	};
	let body = request
		.into_body()
		.into_data_stream()
		.map(|result| result.map_err(anyhow::Error::from))
		.boxed();
	let mut reader = Reader::new(body);
	let clock = ups.env().clock();
	let opened = async {
		let read_deadline =
			rivet_env::deadline_after(clock.as_ref(), ups.inner.options.request_timeout)?;
		let bytes = rivet_env::timeout_at(clock.as_ref(), read_deadline, reader.frame(2048))
			.await
			.map_err(crate::pubsub::request::timeout_error)??;
		let Frame::ChannelOpen(ChannelOpen {
			source_worker_id,
			worker_id,
			instance_id,
		}) = frame::decode(bytes)?
		else {
			return Err(errors::InvalidMessage.build());
		};
		if WorkerId::from_bytes(worker_id) != *ups.worker_id()
			|| instance_id != *ups.instance_id().as_bytes()
		{
			return Err(errors::WrongWorker.build());
		}
		anyhow::Ok(WorkerId::from_bytes(source_worker_id))
	}
	.await;
	let source = match opened {
		Ok(source) => source,
		Err(error) => {
			let status = match RivetError::extract(&error).code() {
				"wrong_worker" => StatusCode::CONFLICT,
				"timeout" => StatusCode::GATEWAY_TIMEOUT,
				_ => StatusCode::BAD_REQUEST,
			};
			return status.into_response();
		}
	};
	let options = &ups.inner.options.channel;
	let (sender, queued) = mpsc::channel(options.queue_frames);
	let replies = Replies {
		sender,
		bytes: frame::ByteBudget::new(options.reply_queue_bytes, None),
	};
	let body = frame::batched(queued, options.max_write_bytes);
	let serving = ups.clone();
	ups.inner.channels.server_tasks.spawn(async move {
		let _slot = slot;
		serving.serve_channel(source, reader, replies).await;
	});
	Response::builder()
		.status(StatusCode::OK)
		.header("content-type", "application/octet-stream")
		.body(Body::from_stream(body))
		.unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

impl PubSub {
	/// Reads requests and cancels until the caller's body ends or this worker starts draining.
	/// Each request is dispatched to its subscriber as it is read, so the subscriber sees one
	/// channel's requests in the order the caller wrote them, and then waits for its reply in its
	/// own task. The reply body ends once every running request has
	/// answered, which also lets graceful listener shutdown finish: a caller keeps a healthy
	/// channel open indefinitely, so the server must end it. Requests the caller sent after
	/// draining began are never read, and the caller fails them when the body ends.
	async fn serve_channel(&self, source: WorkerId, mut reader: Reader, replies: Replies) {
		let _open = super::client::ChannelsOpenGuard::new("server");
		// A minimum capacity keeps the bucket array allocated while the channel idles between
		// requests; an empty map would free it after each request and allocate it again.
		let handlers: Handlers = Arc::new(scc::HashMap::with_capacity(HANDLER_MAP_CAPACITY));
		let max_frame = self
			.inner
			.options
			.max_message_bytes
			.saturating_add(frame::ENVELOPE_BYTES);
		let drain_started = self.inner.lifecycle.drain_started.clone();
		loop {
			let read = rivet_env::select_biased!("stop reading once draining begins",
				_ = drain_started.cancelled() => None,
				read = reader.frame(max_frame) => Some(read),
			);
			let bytes = match read {
				Some(Ok(bytes)) => bytes,
				Some(Err(_)) | None => break,
			};
			match frame::decode(bytes) {
				Ok(Frame::ChannelRequest(request)) => {
					self.start_channel_request(source.clone(), request, &replies, &handlers)
						.await;
				}
				Ok(Frame::ChannelCancel(cancel)) => {
					if let Some((_, handler)) = handlers.remove_async(&cancel.id).await {
						handler.abort();
						crate::metrics::CHANNEL_CANCELS.inc();
					}
				}
				Ok(Frame::ChannelOpen(_) | Frame::ChannelResponse(_)) | Err(_) => break,
			}
		}
	}

	/// Admits and dispatches one request as a per-request stream would, before the reader reads
	/// the next frame, then answers it on the channel from its own task.
	async fn start_channel_request(
		&self,
		source: WorkerId,
		request: ChannelRequest,
		replies: &Replies,
		handlers: &Handlers,
	) {
		let ChannelRequest {
			id,
			class,
			subject,
			remaining_ms,
			payload,
		} = request;
		let class = RequestClass::from(class);
		let admitted = (|| {
			self.validate(&subject, &payload)?;
			self.admit_reply_bytes(replies, class)?;
			// The request is already read, so it takes its class or subject permit directly and
			// never competes with stream header readers for theirs.
			let active = self.accept_request(Some((class, &subject)))?;
			let clock = self.env().clock();
			let deadline =
				crate::pubsub::peer_deadline::local_deadline_ms(remaining_ms, clock.observe()?)?;
			anyhow::Ok((active, deadline))
		})();
		let (active, deadline) = match admitted {
			Ok(admitted) => admitted,
			Err(error) => {
				self.reply_refused(replies, id, error);
				return;
			}
		};
		// The request stays admitted, and draining waits for it, until its reply is queued.
		let active = Arc::new(active);
		let response = match self.dispatch_unary(
			class,
			source,
			&subject,
			&payload,
			deadline,
			active.clone(),
		) {
			Ok(response) => response,
			Err(error) => {
				self.reply_refused(replies, id, error);
				return;
			}
		};
		let clock = self.env().clock();
		let replies = replies.clone();
		let process_bytes = self.inner.channels.reply_bytes.clone();
		let finished = handlers.clone();
		// The handler removes its own entry only after it was inserted, so a fast handler
		// cannot leave a stale entry behind.
		let (registered, inserted) = tokio::sync::oneshot::channel::<()>();
		let task = self.inner.channels.server_tasks.spawn(async move {
			let result = response.receive_one().await;
			let _ = inserted.await;
			finished.remove_async(&id).await;
			let Some(frame) = encode_reply(id, result) else {
				return;
			};
			// The reply waits for room in both byte budgets and then for a queue slot, holding
			// its admission. A caller that stopped reading gets nothing after its deadline, so a
			// stalled channel cannot hold a reply and its admission forever.
			let queue = async {
				let channel = replies.bytes.reserve(frame.len()).await;
				let process = process_bytes.reserve(frame.len()).await;
				let (reply, queued) = Reply::answer(frame, channel, process);
				replies.sender.send(reply).await.ok()?;
				Some(queued)
			};
			let sent = rivet_env::timeout_at(clock.as_ref(), deadline, queue).await;
			drop(replies);
			drop(active);
			let queued = match sent {
				Ok(Some(queued)) => queued,
				// The caller's channel ended, so nobody reads this reply.
				Ok(None) => return,
				Err(_) => {
					crate::metrics::CHANNEL_REPLIES_DROPPED.inc();
					return;
				}
			};
			queued.expire_at(clock.as_ref(), deadline).await;
		});
		let _ = handlers.insert_async(id, task).await;
		let _ = registered.send(());
	}
}

impl PubSub {
	/// Refuses a request whose reply could not be queued, before any work is done: its own
	/// channel or this worker's served channels together hold as many reply bytes as allowed.
	/// The caller owns any retry.
	fn admit_reply_bytes(&self, replies: &Replies, class: RequestClass) -> anyhow::Result<()> {
		let channels = &self.inner.channels;
		let channel_full = replies.bytes.is_full();
		let process_full = channels.reply_bytes.is_full();
		// The flag changes only when the budget crosses its cap, so the shared line is written
		// once per episode rather than once per request.
		let full_flag = &channels.reply_bytes_full;
		if full_flag.load(std::sync::atomic::Ordering::Acquire) != process_full {
			let was_full = full_flag.swap(process_full, std::sync::atomic::Ordering::AcqRel);
			if process_full && !was_full {
				tracing::warn!(
					capacity = self.inner.options.channel.total_reply_queue_bytes,
					"served peer channel replies reached their byte cap; callers are reading replies slower than this worker produces them"
				);
			}
		}
		let (reason, capacity) = match (channel_full, process_full) {
			(false, false) => return Ok(()),
			(true, _) => ("reply_bytes_channel", "channel_reply_bytes"),
			(false, true) => ("reply_bytes_process", "process_reply_bytes"),
		};
		crate::metrics::CHANNEL_QUEUE_FULL
			.with_label_values(&[reason])
			.inc();
		Err(crate::metrics::overloaded(capacity, Some(class.as_str())))
	}

	/// Answers a request that was refused at admission. The reader never waits on a full reply
	/// queue, so cancels keep flowing. A refusal that does not fit at once is dropped: the queue
	/// is full because the caller is not reading, and its request fails at its own deadline.
	fn reply_refused(&self, replies: &Replies, id: u64, error: anyhow::Error) {
		let Some(frame) = encode_reply(id, Err(error)) else {
			return;
		};
		match replies.sender.try_send(Reply::Refusal(frame)) {
			Ok(()) | Err(mpsc::error::TrySendError::Closed(_)) => {}
			Err(mpsc::error::TrySendError::Full(_)) => {
				crate::metrics::CHANNEL_REPLIES_DROPPED.inc();
			}
		}
	}
}

/// One served channel's reply queue and the bytes it holds.
#[derive(Clone)]
struct Replies {
	sender: mpsc::Sender<Reply>,
	bytes: Arc<frame::ByteBudget>,
}

/// A reply frame and the reply bytes it holds. The reservations travel with the frame's bytes
/// into the writer and the transport, so a frame taken from the queue but not yet written
/// still counts against the budgets.
struct QueuedReply {
	frame: Bytes,
	_channel: frame::ByteReservation,
	_process: frame::ByteReservation,
}

impl AsRef<[u8]> for QueuedReply {
	fn as_ref(&self) -> &[u8] {
		&self.frame
	}
}

/// An entry in a served channel's write queue.
enum Reply {
	/// A handler's answer. The writer takes the frame unless its handler took it back first
	/// because the request's deadline passed.
	Answer {
		queued: Arc<parking_lot::Mutex<Option<QueuedReply>>>,
		/// Dropped once the writer has taken or skipped this reply, which tells its handler to
		/// stop waiting for the deadline.
		_taken: tokio::sync::oneshot::Sender<()>,
	},
	/// A refusal at admission. It is a few bytes and the queue's frame cap bounds how many wait,
	/// so it takes no room in the byte budgets and never makes another request shed.
	Refusal(Bytes),
}

/// The handler's hold on a reply it queued.
struct ReplyHold {
	queued: Arc<parking_lot::Mutex<Option<QueuedReply>>>,
	taken: tokio::sync::oneshot::Receiver<()>,
}

impl Reply {
	/// Wraps a frame whose bytes are already reserved in its channel's and this worker's reply
	/// budgets.
	fn answer(
		frame: Bytes,
		channel: frame::ByteReservation,
		process: frame::ByteReservation,
	) -> (Self, ReplyHold) {
		let queued = Arc::new(parking_lot::Mutex::new(Some(QueuedReply {
			frame,
			_channel: channel,
			_process: process,
		})));
		let (taken_sender, taken) = tokio::sync::oneshot::channel();
		let reply = Self::Answer {
			queued: queued.clone(),
			_taken: taken_sender,
		};
		(reply, ReplyHold { queued, taken })
	}
}

impl frame::Queued for Reply {
	fn into_frame(self) -> Option<Bytes> {
		match self {
			Reply::Answer { queued, _taken } => {
				// An empty slot was dropped at its deadline and counted by its handler.
				let queued = queued.lock().take()?;
				Some(Bytes::from_owner(queued))
			}
			Reply::Refusal(frame) => Some(frame),
		}
	}
}

impl ReplyHold {
	/// Waits until the writer takes the reply or the request's deadline passes. At the deadline
	/// the reply is dropped unwritten, and its bytes return to the budgets at once instead of
	/// waiting for a stalled caller to read up to it.
	async fn expire_at(self, clock: &dyn rivet_env::Clock, deadline: u64) {
		let ReplyHold { queued, taken } = self;
		let expired = rivet_env::timeout_at(clock, deadline, taken).await.is_err();
		if !expired {
			return;
		}
		let dropped = queued.lock().take();
		if dropped.is_some() {
			crate::metrics::CHANNEL_REPLIES_DROPPED.inc();
		}
	}
}

/// The response frame for one request, or `None` if it cannot be encoded.
fn encode_reply(id: u64, result: anyhow::Result<Bytes>) -> Option<Bytes> {
	let body = match result {
		Ok(payload) => ChannelBody::ChannelPayload(payload),
		Err(error) => ChannelBody::ChannelError(ChannelError {
			code: RivetError::extract(&error).code().to_owned().into(),
		}),
	};
	frame::encode(Frame::ChannelResponse(ChannelResponse { id, body })).ok()
}

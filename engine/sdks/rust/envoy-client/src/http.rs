use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use anyhow::{Context, Result as AnyhowResult, ensure};
use parking_lot::Mutex as SyncMutex;
use rivet_envoy_protocol as protocol;
use tokio::sync::{Mutex, Notify, mpsc};

pub const HTTP_BODY_STREAM_CHANNEL_CAPACITY: usize = 16;
pub const HTTP_BODY_MAX_CHUNK_SIZE: usize = 64 * 1024;

#[derive(Clone, Debug)]
pub struct HttpRequestBodyError {
	pub reason: protocol::HttpStreamAbortReason,
}

impl std::fmt::Display for HttpRequestBodyError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match &self.reason.detail {
			Some(detail) => write!(f, "{:?}: {detail}", self.reason.kind),
			None => write!(f, "{:?}", self.reason.kind),
		}
	}
}

impl std::error::Error for HttpRequestBodyError {}

#[derive(Debug)]
pub struct HttpRequestBodyStream {
	queue: Arc<HttpRequestBodyQueue>,
	event_tx: Option<mpsc::UnboundedSender<RequestBodyEvent>>,
	terminal: bool,
}

#[derive(Debug)]
struct HttpRequestBodyQueueState {
	chunks: VecDeque<Vec<u8>>,
	finished: bool,
	abort: Option<HttpRequestBodyError>,
	receiver_closed: bool,
}

/// Byte-windowed request bodies use a coalescing queue instead of a frame-count bounded channel.
/// The producer is the synchronous actor inbox, so this is a deliberately short forced-sync lock;
/// it is never held across an await. Byte credit separately caps the total queued payload.
#[derive(Debug)]
pub struct HttpRequestBodyQueue {
	state: SyncMutex<HttpRequestBodyQueueState>,
	ready: Notify,
}

impl HttpRequestBodyQueue {
	pub fn new() -> Arc<Self> {
		Arc::new(Self {
			state: SyncMutex::new(HttpRequestBodyQueueState {
				chunks: VecDeque::new(),
				finished: false,
				abort: None,
				receiver_closed: false,
			}),
			ready: Notify::new(),
		})
	}

	/// Queues `data`, copying it once into the coalesced chunks.
	pub fn push(&self, data: impl AsRef<[u8]>) -> bool {
		let mut data = data.as_ref();
		let mut state = self.state.lock();
		if state.finished || state.abort.is_some() || state.receiver_closed {
			return false;
		}

		if let Some(last) = state.chunks.back_mut() {
			let available = HTTP_BODY_MAX_CHUNK_SIZE.saturating_sub(last.len());
			let append = available.min(data.len());
			last.extend_from_slice(&data[..append]);
			data = &data[append..];
		}
		for chunk in data.chunks(HTTP_BODY_MAX_CHUNK_SIZE) {
			state.chunks.push_back(chunk.to_vec());
		}
		drop(state);
		self.ready.notify_one();
		true
	}

	pub fn finish(&self) {
		let mut state = self.state.lock();
		if state.abort.is_none() {
			state.finished = true;
		}
		drop(state);
		self.ready.notify_waiters();
	}

	pub fn abort(&self, error: HttpRequestBodyError) {
		let mut state = self.state.lock();
		if state.abort.is_none() && !state.finished {
			state.chunks.clear();
			state.abort = Some(error);
		}
		drop(state);
		self.ready.notify_waiters();
	}

	fn close_receiver(&self) {
		let mut state = self.state.lock();
		state.receiver_closed = true;
		state.chunks.clear();
		drop(state);
		self.ready.notify_waiters();
	}

	async fn recv(&self) -> Result<Option<Vec<u8>>, HttpRequestBodyError> {
		loop {
			let notified = self.ready.notified();
			{
				let mut state = self.state.lock();
				if let Some(error) = &state.abort {
					return Err(error.clone());
				}
				if let Some(chunk) = state.chunks.pop_front() {
					return Ok(Some(chunk));
				}
				if state.finished || state.receiver_closed {
					return Ok(None);
				}
			}
			notified.await;
		}
	}
}

#[derive(Debug)]
pub(crate) enum RequestBodyEvent {
	Consumed(u64),
	Cancelled,
}

impl HttpRequestBodyStream {
	fn handle_chunk(
		&mut self,
		chunk: Option<Vec<u8>>,
	) -> Result<Option<Vec<u8>>, HttpRequestBodyError> {
		match chunk {
			Some(chunk) => {
				if let Some(event_tx) = &self.event_tx {
					let _ = event_tx.send(RequestBodyEvent::Consumed(chunk.len() as u64));
				}
				Ok(Some(chunk))
			}
			None => {
				self.terminal = true;
				Ok(None)
			}
		}
	}

	/// A body read from `queue` without reporting consumption to a flow-control window.
	pub fn new(queue: Arc<HttpRequestBodyQueue>) -> Self {
		Self {
			queue,
			event_tx: None,
			terminal: false,
		}
	}

	pub(crate) fn new_with_flow_control(
		queue: Arc<HttpRequestBodyQueue>,
		event_tx: mpsc::UnboundedSender<RequestBodyEvent>,
	) -> Self {
		Self {
			queue,
			event_tx: Some(event_tx),
			terminal: false,
		}
	}

	pub async fn recv(&mut self) -> Result<Option<Vec<u8>>, HttpRequestBodyError> {
		let chunk = match self.queue.recv().await {
			Ok(chunk) => chunk,
			Err(error) => {
				self.terminal = true;
				return Err(error);
			}
		};
		self.handle_chunk(chunk)
	}
}

impl Drop for HttpRequestBodyStream {
	fn drop(&mut self) {
		self.queue.close_receiver();
		if !self.terminal
			&& let Some(event_tx) = &self.event_tx
		{
			let _ = event_tx.send(RequestBodyEvent::Cancelled);
		}
	}
}

#[derive(Debug)]
struct HttpBodySendWindowState {
	sent_bytes: u64,
	consumed_bytes: u64,
}

#[derive(Debug)]
pub(crate) struct HttpBodySendWindow {
	state: Mutex<HttpBodySendWindowState>,
	credit_available: Notify,
}

impl HttpBodySendWindow {
	pub(crate) fn new() -> Arc<Self> {
		Arc::new(Self {
			state: Mutex::new(HttpBodySendWindowState {
				sent_bytes: 0,
				consumed_bytes: 0,
			}),
			credit_available: Notify::new(),
		})
	}

	pub(crate) async fn reserve(&self, bytes: u64) -> AnyhowResult<()> {
		ensure!(
			bytes <= protocol::HTTP_STREAM_INITIAL_WINDOW_BYTES,
			"HTTP body frame exceeds the flow-control window"
		);
		if bytes == 0 {
			return Ok(());
		}

		loop {
			let notified = self.credit_available.notified();
			{
				let mut state = self.state.lock().await;
				let outstanding = state
					.sent_bytes
					.checked_sub(state.consumed_bytes)
					.context("HTTP body flow-control accounting underflow")?;
				let available = protocol::HTTP_STREAM_INITIAL_WINDOW_BYTES
					.checked_sub(outstanding)
					.context("HTTP body flow-control window exceeded")?;
				if bytes <= available {
					state.sent_bytes = state
						.sent_bytes
						.checked_add(bytes)
						.context("HTTP body sent-byte counter overflow")?;
					return Ok(());
				}
			}
			notified.await;
		}
	}

	pub(crate) async fn update_consumed(&self, consumed_bytes: u64) -> AnyhowResult<()> {
		let mut state = self.state.lock().await;
		ensure!(
			consumed_bytes >= state.consumed_bytes,
			"HTTP body consumed-byte counter moved backwards"
		);
		ensure!(
			consumed_bytes <= state.sent_bytes,
			"HTTP body consumed-byte counter exceeds sent bytes"
		);
		if consumed_bytes == state.consumed_bytes {
			return Ok(());
		}
		state.consumed_bytes = consumed_bytes;
		drop(state);
		self.credit_available.notify_waiters();
		Ok(())
	}
}

/// HTTP request/response types used by the envoy client.
pub struct HttpRequest {
	pub method: String,
	pub path: String,
	pub headers: HashMap<String, String>,
	pub body: Option<Vec<u8>>,
	/// If the request is streamed, body chunks arrive on this channel.
	pub body_stream: Option<HttpRequestBodyStream>,
}

pub struct HttpResponse {
	pub status: u16,
	pub headers: HashMap<String, String>,
	pub body: Option<Vec<u8>>,
	/// If set, the response is streamed. The envoy client reads chunks and sends
	/// `ToRivetResponseChunk` for each one.
	pub body_stream: Option<HttpResponseBodyStream>,
}

/// A chunk in a streaming HTTP response.
pub enum ResponseChunk {
	Data { data: Vec<u8>, finish: bool },
	Error(String),
}

pub struct HttpResponseBodyStream {
	rx: mpsc::Receiver<ResponseChunk>,
	on_drop: Option<Box<dyn FnOnce() + Send>>,
}

impl HttpResponseBodyStream {
	pub fn set_on_drop(&mut self, on_drop: impl FnOnce() + Send + 'static) {
		self.on_drop = Some(Box::new(on_drop));
	}

	pub async fn recv(&mut self) -> Option<ResponseChunk> {
		self.rx.recv().await
	}
}

impl From<mpsc::Receiver<ResponseChunk>> for HttpResponseBodyStream {
	fn from(rx: mpsc::Receiver<ResponseChunk>) -> Self {
		Self { rx, on_drop: None }
	}
}

impl Drop for HttpResponseBodyStream {
	fn drop(&mut self) {
		if let Some(on_drop) = self.on_drop.take() {
			on_drop();
		}
	}
}



use anyhow::*;
use futures_util::{SinkExt, StreamExt, stream::Peekable};
use hyper::upgrade::Upgraded;
use hyper_tungstenite::HyperWebsocket;
use hyper_tungstenite::tungstenite::Message;
use hyper_util::rt::TokioIo;
use rivet_perf::{perf_finish, perf_start};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tokio_tungstenite::WebSocketStream;

use crate::metrics;

pub type WebSocketReceiver =
	Peekable<futures_util::stream::SplitStream<WebSocketStream<TokioIo<Upgraded>>>>;

pub type WebSocketSender =
	futures_util::stream::SplitSink<WebSocketStream<TokioIo<Upgraded>>, Message>;

#[derive(Clone)]
pub struct WebSocketHandle {
	ws_tx: Arc<Mutex<WebSocketSender>>,
	ws_rx: Arc<Mutex<WebSocketReceiver>>,
}

impl WebSocketHandle {
	#[tracing::instrument(skip_all)]
	pub async fn new(websocket: HyperWebsocket) -> Result<Self> {
		let ws_stream = websocket.await?;
		let (ws_tx, ws_rx) = ws_stream.split();

		Ok(Self {
			ws_tx: Arc::new(Mutex::new(ws_tx)),
			ws_rx: Arc::new(Mutex::new(ws_rx.peekable())),
		})
	}

	#[tracing::instrument(skip_all)]
	pub async fn send(&self, message: Message) -> Result<()> {
		let message_kind = message_kind_label(&message);
		let message_len = message.len();
		let measure = perf_start!(
			&metrics::WEBSOCKET_SEND_DURATION,
			slow_ms = 1000,
			"guard_websocket_send",
			labels: { message_kind = %message_kind },
			fields: { message_len = ?message_len },
		);
		let lock_wait_start = Instant::now();
		let mut guard = self.ws_tx.lock().await;
		let lock_wait_elapsed = lock_wait_start.elapsed();
		metrics::WEBSOCKET_SEND_LOCK_WAIT_DURATION
			.with_label_values(&[message_kind])
			.observe(lock_wait_elapsed.as_secs_f64());

		let write_start = Instant::now();
		let res = guard.send(message).await;
		let write_elapsed = write_start.elapsed();
		metrics::WEBSOCKET_SEND_WRITE_DURATION
			.with_label_values(&[message_kind])
			.observe(write_elapsed.as_secs_f64());
		drop(guard);

		self.record_write_pressure_metrics(message_kind);
		perf_finish!(measure, fields: { message_len = message_len, result = %res.is_ok() });
		res?;
		Ok(())
	}

	#[tracing::instrument(skip_all)]
	pub async fn flush(&self) -> Result<()> {
		let res = self.ws_tx.lock().await.flush().await;
		self.record_write_pressure_metrics("flush");
		res?;
		Ok(())
	}

	pub fn recv(&self) -> Arc<Mutex<WebSocketReceiver>> {
		self.ws_rx.clone()
	}

	fn record_write_pressure_metrics(&self, message_kind: &str) {
		let _ = message_kind;
	}
}

fn message_kind_label(message: &Message) -> &'static str {
	match message {
		Message::Text(_) => "text",
		Message::Binary(_) => "binary",
		Message::Ping(_) => "ping",
		Message::Pong(_) => "pong",
		Message::Close(_) => "close",
		Message::Frame(_) => "frame",
	}
}

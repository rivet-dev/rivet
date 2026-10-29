use anyhow::{Result, ensure};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt, stream::Peekable};
use hyper::upgrade::Upgraded;
use hyper_tungstenite::HyperWebsocket;
use hyper_tungstenite::tungstenite::Message;
use hyper_util::rt::TokioIo;
use rivet_perf::{perf_finish, perf_start};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio_tungstenite::WebSocketStream;
use tokio_util::task::AbortOnDropHandle;

use crate::metrics;

pub type WebSocketReceiver =
	Peekable<futures_util::stream::SplitStream<WebSocketStream<TokioIo<Upgraded>>>>;

pub type WebSocketSender =
	futures_util::stream::SplitSink<WebSocketStream<TokioIo<Upgraded>>, Message>;

#[derive(Clone)]
pub struct WebSocketHandle {
	ws_tx: Arc<Mutex<WebSocketSender>>,
	ws_rx: Arc<Mutex<WebSocketReceiver>>,
	// The last handle owns cancellation, so keepalive cannot outlive the session.
	_keepalive: Arc<AbortOnDropHandle<()>>,
}

impl WebSocketHandle {
	#[tracing::instrument(level = "debug", skip_all)]
	pub async fn new(websocket: HyperWebsocket) -> Result<Self> {
		let config = rivet_config::config::guard::Guard::default();
		Self::new_with_keepalive(
			websocket,
			config.websocket_keepalive_interval(),
			config.websocket_send_timeout(),
		)
		.await
	}

	/// Keeps quiet client connections alive through idle-timeout proxies.
	#[tracing::instrument(level = "debug", skip_all)]
	pub async fn new_with_keepalive(
		websocket: HyperWebsocket,
		interval: Duration,
		send_timeout: Duration,
	) -> Result<Self> {
		ensure!(
			!interval.is_zero(),
			"websocket keepalive interval must be positive"
		);
		ensure!(
			!send_timeout.is_zero(),
			"websocket send timeout must be positive"
		);
		let ws_stream = websocket.await?;
		let (ws_tx, ws_rx) = ws_stream.split();
		let ws_tx = Arc::new(Mutex::new(ws_tx));
		let keepalive = AbortOnDropHandle::new(tokio::spawn(run_keepalive(
			ws_tx.clone(),
			interval,
			send_timeout,
		)));

		Ok(Self {
			ws_tx,
			ws_rx: Arc::new(Mutex::new(ws_rx.peekable())),
			_keepalive: Arc::new(keepalive),
		})
	}

	#[tracing::instrument(level = "debug", skip_all)]
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

	#[tracing::instrument(level = "debug", skip_all)]
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

async fn run_keepalive(
	ws_tx: Arc<Mutex<WebSocketSender>>,
	interval: Duration,
	send_timeout: Duration,
) {
	let mut ticker = tokio::time::interval_at(tokio::time::Instant::now() + interval, interval);
	ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
	loop {
		ticker.tick().await;
		// Bound both lock acquisition and the write so a stalled peer cannot hold
		// the writer indefinitely through a background keepalive task.
		let result = tokio::time::timeout(send_timeout, async {
			ws_tx.lock().await.send(Message::Ping(Bytes::new())).await
		})
		.await;
		match result {
			Ok(Ok(())) => {
				metrics::WEBSOCKET_KEEPALIVE_PING_TOTAL
					.with_label_values(&["ok"])
					.inc();
			}
			Ok(Err(error)) => {
				tracing::debug!(?error, "stopping websocket keepalive after failed ping");
				metrics::WEBSOCKET_KEEPALIVE_PING_TOTAL
					.with_label_values(&["error"])
					.inc();
				return;
			}
			Err(_) => {
				tracing::debug!("stopping websocket keepalive after ping timeout");
				metrics::WEBSOCKET_KEEPALIVE_PING_TOTAL
					.with_label_values(&["timeout"])
					.inc();
				return;
			}
		}
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

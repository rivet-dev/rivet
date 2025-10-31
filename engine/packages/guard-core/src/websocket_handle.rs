use anyhow::*;
use futures_util::{SinkExt, StreamExt};
use hyper::upgrade::Upgraded;
use hyper_tungstenite::HyperWebsocket;
use hyper_tungstenite::tungstenite::Message as WsMessage;
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::WebSocketStream;

pub type WebSocketReceiver = futures_util::stream::SplitStream<WebSocketStream<TokioIo<Upgraded>>>;

pub type WebSocketSender =
	futures_util::stream::SplitSink<WebSocketStream<TokioIo<Upgraded>>, WsMessage>;

#[derive(Clone)]
pub struct WebSocketHandle {
	ws_tx: Arc<Mutex<WebSocketSender>>,
	ws_rx: Arc<Mutex<WebSocketReceiver>>,
}

impl WebSocketHandle {
	pub async fn new(websocket: HyperWebsocket) -> Result<Self> {
		let ws_stream = websocket.await?;
		let (ws_tx, ws_rx) = ws_stream.split();

		Ok(Self {
			ws_tx: Arc::new(Mutex::new(ws_tx)),
			ws_rx: Arc::new(Mutex::new(ws_rx)),
		})
	}

	pub async fn send(&self, message: WsMessage) -> Result<()> {
		self.ws_tx.lock().await.send(message).await?;
		Ok(())
	}

	pub async fn flush(&self) -> Result<()> {
		self.ws_tx.lock().await.flush().await?;
		Ok(())
	}

	pub fn recv(&self) -> Arc<Mutex<WebSocketReceiver>> {
		self.ws_rx.clone()
	}
}

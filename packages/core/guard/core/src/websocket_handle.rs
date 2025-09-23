use anyhow::*;
use futures_util::{SinkExt, StreamExt};
use hyper::upgrade::Upgraded;
use hyper_tungstenite::HyperWebsocket;
use hyper_tungstenite::tungstenite::Message as WsMessage;
use hyper_util::rt::TokioIo;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::WebSocketStream;

pub type WebSocketReceiver = futures_util::stream::SplitStream<WebSocketStream<TokioIo<Upgraded>>>;

pub type WebSocketSender =
	futures_util::stream::SplitSink<WebSocketStream<TokioIo<Upgraded>>, WsMessage>;

enum WebSocketState {
	Unaccepted { websocket: HyperWebsocket },
	Accepting,
	Split { ws_tx: WebSocketSender },
}

#[derive(Clone)]
pub struct WebSocketHandle(Arc<WebSocketHandleInner>);

impl WebSocketHandle {
	pub fn new(websocket: HyperWebsocket) -> Self {
		Self(Arc::new(WebSocketHandleInner {
			state: Mutex::new(WebSocketState::Unaccepted { websocket }),
		}))
	}
}

impl Deref for WebSocketHandle {
	type Target = WebSocketHandleInner;

	fn deref(&self) -> &Self::Target {
		&*self.0
	}
}

pub struct WebSocketHandleInner {
	state: Mutex<WebSocketState>,
}

impl WebSocketHandleInner {
	pub async fn accept(&self) -> Result<WebSocketReceiver> {
		let mut state = self.state.lock().await;
		Self::accept_inner(&mut *state).await
	}

	pub async fn send(&self, message: WsMessage) -> Result<()> {
		let mut state = self.state.lock().await;
		match &mut *state {
			WebSocketState::Unaccepted { .. } | WebSocketState::Accepting => {
				bail!("websocket has not been accepted")
			}
			WebSocketState::Split { ws_tx } => {
				ws_tx.send(message).await?;
				Ok(())
			}
		}
	}

	pub async fn accept_and_send(&self, message: WsMessage) -> Result<()> {
		let mut state = self.state.lock().await;
		match &mut *state {
			WebSocketState::Unaccepted { .. } => {
				let _ = Self::accept_inner(&mut *state).await?;
				let WebSocketState::Split { ws_tx } = &mut *state else {
					bail!("websocket should be accepted");
				};
				ws_tx.send(message).await?;
				Ok(())
			}
			WebSocketState::Accepting => {
				bail!("in accepting state")
			}
			WebSocketState::Split { ws_tx } => {
				ws_tx.send(message).await?;
				Ok(())
			}
		}
	}

	async fn accept_inner(state: &mut WebSocketState) -> Result<WebSocketReceiver> {
		if !matches!(*state, WebSocketState::Unaccepted { .. }) {
			bail!("websocket already accepted")
		}

		// Accept websocket
		let old_state = std::mem::replace(&mut *state, WebSocketState::Accepting);
		let WebSocketState::Unaccepted { websocket } = old_state else {
			bail!("should be in unaccepted state");
		};

		// Accept WS
		let ws_stream = websocket.await?;
		let (ws_tx, ws_rx) = ws_stream.split();
		*state = WebSocketState::Split { ws_tx };

		Ok(ws_rx)
	}
}

use std::fmt;
use std::sync::Arc;
use std::sync::RwLock;

use anyhow::{Result, anyhow};
use rivet_envoy_client::config::WebSocketSender;

use crate::types::WsMessage;

pub(crate) type WebSocketSendCallback =
	Arc<dyn Fn(WsMessage) -> Result<()> + Send + Sync>;
pub(crate) type WebSocketCloseCallback =
	Arc<dyn Fn(Option<u16>, Option<String>) -> Result<()> + Send + Sync>;

#[derive(Clone)]
pub struct WebSocket(Arc<WebSocketInner>);

struct WebSocketInner {
	send_callback: RwLock<Option<WebSocketSendCallback>>,
	close_callback: RwLock<Option<WebSocketCloseCallback>>,
}

impl WebSocket {
	pub fn new() -> Self {
		Self(Arc::new(WebSocketInner {
			send_callback: RwLock::new(None),
			close_callback: RwLock::new(None),
		}))
	}

	pub fn from_sender(sender: WebSocketSender) -> Self {
		let websocket = Self::new();
		let send_sender = sender.clone();
		let close_sender = sender;
		websocket.configure_send_callback(Some(Arc::new(move |message| {
			match message {
				WsMessage::Text(text) => send_sender.send_text(&text),
				WsMessage::Binary(bytes) => send_sender.send(bytes, true),
			}
			Ok(())
		})));
		websocket.configure_close_callback(Some(Arc::new(move |code, reason| {
			close_sender.close(code, reason);
			Ok(())
		})));
		websocket
	}

	pub fn send(&self, msg: WsMessage) {
		if let Err(error) = self.try_send(msg) {
			tracing::error!(?error, "failed to send websocket message");
		}
	}

	pub fn close(&self, code: Option<u16>, reason: Option<String>) {
		if let Err(error) = self.try_close(code, reason) {
			tracing::error!(?error, "failed to close websocket");
		}
	}

	pub(crate) fn configure_send_callback(
		&self,
		send_callback: Option<WebSocketSendCallback>,
	) {
		*self
			.0
			.send_callback
			.write()
			.expect("websocket send callback lock poisoned") = send_callback;
	}

	pub(crate) fn configure_close_callback(
		&self,
		close_callback: Option<WebSocketCloseCallback>,
	) {
		*self
			.0
			.close_callback
			.write()
			.expect("websocket close callback lock poisoned") = close_callback;
	}

	pub(crate) fn try_send(&self, msg: WsMessage) -> Result<()> {
		let callback = self.send_callback()?;
		callback(msg)
	}

	pub(crate) fn try_close(
		&self,
		code: Option<u16>,
		reason: Option<String>,
	) -> Result<()> {
		let callback = self.close_callback()?;
		callback(code, reason)
	}

	fn send_callback(&self) -> Result<WebSocketSendCallback> {
		self.0
			.send_callback
			.read()
			.expect("websocket send callback lock poisoned")
			.clone()
			.ok_or_else(|| anyhow!("websocket send callback is not configured"))
	}

	fn close_callback(&self) -> Result<WebSocketCloseCallback> {
		self.0
			.close_callback
			.read()
			.expect("websocket close callback lock poisoned")
			.clone()
			.ok_or_else(|| anyhow!("websocket close callback is not configured"))
	}
}

impl Default for WebSocket {
	fn default() -> Self {
		Self::new()
	}
}

impl fmt::Debug for WebSocket {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("WebSocket")
			.field(
				"send_configured",
				&self
					.0
					.send_callback
					.read()
					.expect("websocket send callback lock poisoned")
					.is_some(),
			)
			.field(
				"close_configured",
				&self
					.0
					.close_callback
					.read()
					.expect("websocket close callback lock poisoned")
					.is_some(),
			)
			.finish()
	}
}

#[cfg(test)]
mod tests {
	use std::sync::Arc;
	use std::sync::Mutex;

	use super::{WebSocket, WebSocketCloseCallback, WebSocketSendCallback};
	use crate::types::WsMessage;

	#[test]
	fn send_uses_configured_callback() {
		let sent = Arc::new(Mutex::new(Vec::<WsMessage>::new()));
		let sent_clone = sent.clone();
		let ws = WebSocket::new();
		let send_callback: WebSocketSendCallback = Arc::new(move |message| {
			sent_clone
				.lock()
				.expect("sent websocket messages lock poisoned")
				.push(message);
			Ok(())
		});

		ws.configure_send_callback(Some(send_callback));
		ws.send(WsMessage::Text("hello".to_owned()));

		assert_eq!(
			*sent.lock().expect("sent websocket messages lock poisoned"),
			vec![WsMessage::Text("hello".to_owned())]
		);
	}

	#[test]
	fn close_uses_configured_callback() {
		let closed = Arc::new(Mutex::new(None::<(Option<u16>, Option<String>)>));
		let closed_clone = closed.clone();
		let ws = WebSocket::new();
		let close_callback: WebSocketCloseCallback = Arc::new(move |code, reason| {
			*closed_clone
				.lock()
				.expect("closed websocket lock poisoned") = Some((code, reason));
			Ok(())
		});

		ws.configure_close_callback(Some(close_callback));
		ws.close(Some(1000), Some("bye".to_owned()));

		assert_eq!(
			*closed.lock().expect("closed websocket lock poisoned"),
			Some((Some(1000), Some("bye".to_owned())))
		);
	}
}

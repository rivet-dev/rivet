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
pub(crate) type WebSocketMessageEventCallback =
	Arc<dyn Fn(WsMessage, Option<u16>) -> Result<()> + Send + Sync>;
pub(crate) type WebSocketCloseEventCallback =
	Arc<dyn Fn(u16, String, bool) -> Result<()> + Send + Sync>;

#[derive(Clone)]
pub struct WebSocket(Arc<WebSocketInner>);

struct WebSocketInner {
	send_callback: RwLock<Option<WebSocketSendCallback>>,
	close_callback: RwLock<Option<WebSocketCloseCallback>>,
	message_event_callback: RwLock<Option<WebSocketMessageEventCallback>>,
	close_event_callback: RwLock<Option<WebSocketCloseEventCallback>>,
}

impl WebSocket {
	pub fn new() -> Self {
		Self(Arc::new(WebSocketInner {
			send_callback: RwLock::new(None),
			close_callback: RwLock::new(None),
			message_event_callback: RwLock::new(None),
			close_event_callback: RwLock::new(None),
		}))
	}

	pub fn from_sender(sender: WebSocketSender) -> Self {
		let websocket = Self::new();
		websocket.configure_sender(sender);
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

	pub fn dispatch_message_event(&self, msg: WsMessage, message_index: Option<u16>) {
		if let Err(error) = self.try_dispatch_message_event(msg, message_index) {
			tracing::error!(?error, "failed to dispatch websocket message event");
		}
	}

	pub fn dispatch_close_event(&self, code: u16, reason: String, was_clean: bool) {
		if let Err(error) = self.try_dispatch_close_event(code, reason, was_clean) {
			tracing::error!(?error, "failed to dispatch websocket close event");
		}
	}

	pub fn configure_sender(&self, sender: WebSocketSender) {
		let send_sender = sender.clone();
		let close_sender = sender;
		self.configure_send_callback(Some(Arc::new(move |message| {
			match message {
				WsMessage::Text(text) => send_sender.send_text(&text),
				WsMessage::Binary(bytes) => send_sender.send(bytes, true),
			}
			Ok(())
		})));
		self.configure_close_callback(Some(Arc::new(move |code, reason| {
			close_sender.close(code, reason);
			Ok(())
		})));
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

	pub fn configure_message_event_callback(
		&self,
		message_event_callback: Option<WebSocketMessageEventCallback>,
	) {
		*self
			.0
			.message_event_callback
			.write()
			.expect("websocket message event callback lock poisoned") = message_event_callback;
	}

	pub fn configure_close_event_callback(
		&self,
		close_event_callback: Option<WebSocketCloseEventCallback>,
	) {
		*self
			.0
			.close_event_callback
			.write()
			.expect("websocket close event callback lock poisoned") = close_event_callback;
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

	pub(crate) fn try_dispatch_message_event(
		&self,
		msg: WsMessage,
		message_index: Option<u16>,
	) -> Result<()> {
		let callback = self.message_event_callback()?;
		callback(msg, message_index)
	}

	pub(crate) fn try_dispatch_close_event(
		&self,
		code: u16,
		reason: String,
		was_clean: bool,
	) -> Result<()> {
		let callback = self.close_event_callback()?;
		callback(code, reason, was_clean)
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

	fn message_event_callback(&self) -> Result<WebSocketMessageEventCallback> {
		self.0
			.message_event_callback
			.read()
			.expect("websocket message event callback lock poisoned")
			.clone()
			.ok_or_else(|| anyhow!("websocket message event callback is not configured"))
	}

	fn close_event_callback(&self) -> Result<WebSocketCloseEventCallback> {
		self.0
			.close_event_callback
			.read()
			.expect("websocket close event callback lock poisoned")
			.clone()
			.ok_or_else(|| anyhow!("websocket close event callback is not configured"))
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
			.field(
				"message_event_configured",
				&self
					.0
					.message_event_callback
					.read()
					.expect("websocket message event callback lock poisoned")
					.is_some(),
			)
			.field(
				"close_event_configured",
				&self
					.0
					.close_event_callback
					.read()
					.expect("websocket close event callback lock poisoned")
					.is_some(),
			)
			.finish()
	}
}

#[cfg(test)]
#[path = "../tests/modules/websocket.rs"]
mod tests;

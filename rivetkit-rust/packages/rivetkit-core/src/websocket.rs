use std::fmt;
use std::sync::Arc;

use anyhow::Result;
use futures::future::BoxFuture;
use parking_lot::RwLock;
use rivet_envoy_client::config::WebSocketSender;

use crate::actor::context::WebSocketCallbackRegion;
use crate::error::ActorRuntime;
use crate::types::WsMessage;

// Rivet supports a non-standard async close-listener extension for actor
// WebSockets. Core tracks close-event delivery with the websocket callback
// region instead of reusing disconnect callbacks because close listeners are
// WebSocket event work, while `onDisconnect` is connection lifecycle work.
pub(crate) type WebSocketSendCallback = Arc<dyn Fn(WsMessage) -> Result<()> + Send + Sync>;
pub(crate) type WebSocketCloseCallback =
	Arc<dyn Fn(Option<u16>, Option<String>) -> BoxFuture<'static, Result<()>> + Send + Sync>;
pub(crate) type WebSocketMessageEventCallback =
	Arc<dyn Fn(WsMessage, Option<u16>) -> Result<()> + Send + Sync>;
pub(crate) type WebSocketCloseEventCallback =
	Arc<dyn Fn(u16, String, bool) -> BoxFuture<'static, Result<()>> + Send + Sync>;
pub(crate) type WebSocketCallbackRegionFactory =
	Arc<dyn Fn() -> WebSocketCallbackRegion + Send + Sync>;

#[derive(Clone)]
pub struct WebSocket(Arc<WebSocketInner>);

struct WebSocketInner {
	// Forced-sync: WebSocket configuration and event dispatch are synchronous
	// public APIs, so callbacks are cloned out before any async close work.
	send_callback: RwLock<Option<WebSocketSendCallback>>,
	close_callback: RwLock<Option<WebSocketCloseCallback>>,
	message_event_callback: RwLock<Option<WebSocketMessageEventCallback>>,
	close_event_callback: RwLock<Option<WebSocketCloseEventCallback>>,
	close_event_callback_region: RwLock<Option<WebSocketCallbackRegionFactory>>,
}

impl WebSocket {
	pub fn new() -> Self {
		Self(Arc::new(WebSocketInner {
			send_callback: RwLock::new(None),
			close_callback: RwLock::new(None),
			message_event_callback: RwLock::new(None),
			close_event_callback: RwLock::new(None),
			close_event_callback_region: RwLock::new(None),
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

	pub async fn close(&self, code: Option<u16>, reason: Option<String>) {
		if let Err(error) = self.try_close(code, reason).await {
			tracing::error!(?error, "failed to close websocket");
		}
	}

	pub fn dispatch_message_event(&self, msg: WsMessage, message_index: Option<u16>) {
		if let Err(error) = self.try_dispatch_message_event(msg, message_index) {
			tracing::error!(?error, "failed to dispatch websocket message event");
		}
	}

	pub async fn dispatch_close_event(&self, code: u16, reason: String, was_clean: bool) {
		if let Err(error) = self.try_dispatch_close_event(code, reason, was_clean).await {
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
			let close_sender = close_sender.clone();
			Box::pin(async move {
				close_sender.close(code, reason);
				Ok(())
			})
		})));
	}

	pub(crate) fn configure_send_callback(&self, send_callback: Option<WebSocketSendCallback>) {
		*self.0.send_callback.write() = send_callback;
	}

	pub(crate) fn configure_close_callback(&self, close_callback: Option<WebSocketCloseCallback>) {
		*self.0.close_callback.write() = close_callback;
	}

	pub fn configure_message_event_callback(
		&self,
		message_event_callback: Option<WebSocketMessageEventCallback>,
	) {
		*self.0.message_event_callback.write() = message_event_callback;
	}

	pub fn configure_close_event_callback(
		&self,
		close_event_callback: Option<WebSocketCloseEventCallback>,
	) {
		*self.0.close_event_callback.write() = close_event_callback;
	}

	pub(crate) fn configure_close_event_callback_region(
		&self,
		close_event_callback_region: Option<WebSocketCallbackRegionFactory>,
	) {
		*self.0.close_event_callback_region.write() = close_event_callback_region;
	}

	pub(crate) fn try_send(&self, msg: WsMessage) -> Result<()> {
		let callback = self.send_callback()?;
		callback(msg)
	}

	pub(crate) async fn try_close(&self, code: Option<u16>, reason: Option<String>) -> Result<()> {
		let callback = self.close_callback()?;
		callback(code, reason).await
	}

	pub(crate) fn try_dispatch_message_event(
		&self,
		msg: WsMessage,
		message_index: Option<u16>,
	) -> Result<()> {
		let callback = self.message_event_callback()?;
		callback(msg, message_index)
	}

	pub(crate) async fn try_dispatch_close_event(
		&self,
		code: u16,
		reason: String,
		was_clean: bool,
	) -> Result<()> {
		let callback = self.close_event_callback()?;
		let _region = self.close_event_callback_region().map(|create| create());
		callback(code, reason, was_clean).await
	}

	fn send_callback(&self) -> Result<WebSocketSendCallback> {
		self.0
			.send_callback
			.read()
			.clone()
			.ok_or_else(|| websocket_not_configured("send callback"))
	}

	fn close_callback(&self) -> Result<WebSocketCloseCallback> {
		self.0
			.close_callback
			.read()
			.clone()
			.ok_or_else(|| websocket_not_configured("close callback"))
	}

	fn message_event_callback(&self) -> Result<WebSocketMessageEventCallback> {
		self.0
			.message_event_callback
			.read()
			.clone()
			.ok_or_else(|| websocket_not_configured("message event callback"))
	}

	fn close_event_callback(&self) -> Result<WebSocketCloseEventCallback> {
		self.0
			.close_event_callback
			.read()
			.clone()
			.ok_or_else(|| websocket_not_configured("close event callback"))
	}

	fn close_event_callback_region(&self) -> Option<WebSocketCallbackRegionFactory> {
		self.0.close_event_callback_region.read().clone()
	}
}

fn websocket_not_configured(component: &str) -> anyhow::Error {
	ActorRuntime::NotConfigured {
		component: format!("websocket {component}"),
	}
	.build()
}

impl Default for WebSocket {
	fn default() -> Self {
		Self::new()
	}
}

impl fmt::Debug for WebSocket {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("WebSocket")
			.field("send_configured", &self.0.send_callback.read().is_some())
			.field("close_configured", &self.0.close_callback.read().is_some())
			.field(
				"message_event_configured",
				&self.0.message_event_callback.read().is_some(),
			)
			.field(
				"close_event_configured",
				&self.0.close_event_callback.read().is_some(),
			)
			.field(
				"close_event_region_configured",
				&self.0.close_event_callback_region.read().is_some(),
			)
			.finish()
	}
}

#[cfg(test)]
#[path = "../tests/modules/websocket.rs"]
mod tests;

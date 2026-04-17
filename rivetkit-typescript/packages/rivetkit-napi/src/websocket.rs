use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use rivetkit_core::{WebSocket as CoreWebSocket, WsMessage};

#[napi]
pub struct WebSocket {
	inner: CoreWebSocket,
}

impl WebSocket {
	#[allow(dead_code)]
	pub(crate) fn new(inner: CoreWebSocket) -> Self {
		Self { inner }
	}
}

#[napi]
impl WebSocket {
	#[napi]
	pub fn send(&self, data: Buffer, binary: bool) -> napi::Result<()> {
		let message = if binary {
			WsMessage::Binary(data.to_vec())
		} else {
			WsMessage::Text(String::from_utf8(data.to_vec()).map_err(|error| {
				napi::Error::from_reason(format!(
					"websocket text message must be valid utf-8: {error}"
				))
			})?)
		};
		self.inner.send(message);
		Ok(())
	}

	#[napi]
	pub fn close(&self, code: Option<u16>, reason: Option<String>) {
		self.inner.close(code, reason);
	}
}

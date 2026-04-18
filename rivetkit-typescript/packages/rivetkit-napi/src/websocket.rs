use napi::bindgen_prelude::Buffer;
use napi::threadsafe_function::{
	ErrorStrategy, ThreadSafeCallContext, ThreadsafeFunction, ThreadsafeFunctionCallMode,
};
use napi_derive::napi;
use rivetkit_core::{WebSocket as CoreWebSocket, WsMessage};

#[derive(Clone)]
enum WebSocketEvent {
	Message {
		data: WsMessage,
		message_index: Option<u16>,
	},
	Close {
		code: u16,
		reason: String,
		was_clean: bool,
	},
}

type EventCallback = ThreadsafeFunction<WebSocketEvent, ErrorStrategy::Fatal>;

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

	#[napi]
	pub fn set_event_callback(&self, callback: napi::JsFunction) -> napi::Result<()> {
		let tsfn: EventCallback = callback.create_threadsafe_function(
			0,
			|ctx: ThreadSafeCallContext<WebSocketEvent>| {
				let env = ctx.env;
				let mut object = env.create_object()?;
				match ctx.value {
					WebSocketEvent::Message {
						data,
						message_index,
					} => {
						object.set("kind", "message")?;
						if let Some(message_index) = message_index {
							object.set("messageIndex", message_index)?;
						}
						match data {
							WsMessage::Text(text) => {
								object.set("binary", false)?;
								object.set("data", text)?;
							}
							WsMessage::Binary(bytes) => {
								object.set("binary", true)?;
								object.set("data", Buffer::from(bytes))?;
							}
						}
					}
					WebSocketEvent::Close {
						code,
						reason,
						was_clean,
					} => {
						object.set("kind", "close")?;
						object.set("code", code)?;
						object.set("reason", reason)?;
						object.set("wasClean", was_clean)?;
					}
				}
				Ok(vec![object.into_unknown()])
			},
		)?;

		let message_tsfn = tsfn.clone();
		self.inner
			.configure_message_event_callback(Some(std::sync::Arc::new(
				move |data, message_index| {
					message_tsfn.call(
						WebSocketEvent::Message {
							data,
							message_index,
						},
						ThreadsafeFunctionCallMode::NonBlocking,
					);
					Ok(())
				},
			)));
		self.inner
			.configure_close_event_callback(Some(std::sync::Arc::new(
				move |code, reason, was_clean| {
					tsfn.call(
						WebSocketEvent::Close {
							code,
							reason,
							was_clean,
						},
						ThreadsafeFunctionCallMode::NonBlocking,
					);
					Ok(())
				},
			)));

		Ok(())
	}
}

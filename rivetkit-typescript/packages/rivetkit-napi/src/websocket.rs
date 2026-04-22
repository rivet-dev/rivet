use napi::bindgen_prelude::Buffer;
use napi::threadsafe_function::{
	ErrorStrategy, ThreadSafeCallContext, ThreadsafeFunction, ThreadsafeFunctionCallMode,
};
use napi_derive::napi;
use rivetkit_core::{WebSocket as CoreWebSocket, WsMessage};

use crate::{NapiInvalidArgument, napi_anyhow_error};

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
		tracing::debug!(class = "WebSocket", "constructed napi class");
		Self { inner }
	}
}

impl Drop for WebSocket {
	fn drop(&mut self) {
		tracing::debug!(class = "WebSocket", "dropped napi class");
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
				napi_anyhow_error(
					NapiInvalidArgument {
						argument: "data".to_owned(),
						reason: format!("websocket text message must be valid utf-8: {error}"),
					}
					.build(),
				)
			})?)
		};
		self.inner.send(message);
		Ok(())
	}

	#[napi]
	pub async fn close(&self, code: Option<u16>, reason: Option<String>) -> napi::Result<()> {
		self.inner.close(code, reason).await;
		Ok(())
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
					let event = WebSocketEvent::Message {
						data,
						message_index,
					};
					log_websocket_event_invocation(&event);
					let status = message_tsfn.call(event, ThreadsafeFunctionCallMode::NonBlocking);
					tracing::debug!(
						kind = "websocket.message",
						?status,
						"napi TSF callback returned"
					);
					Ok(())
				},
			)));
		self.inner
			.configure_close_event_callback(Some(std::sync::Arc::new(
				move |code, reason, was_clean| {
					let tsfn = tsfn.clone();
					Box::pin(async move {
						let event = WebSocketEvent::Close {
							code,
							reason,
							was_clean,
						};
						log_websocket_event_invocation(&event);
						let status = tsfn.call(event, ThreadsafeFunctionCallMode::NonBlocking);
						tracing::debug!(
							kind = "websocket.close",
							?status,
							"napi TSF callback returned"
						);
						Ok(())
					})
				},
			)));

		Ok(())
	}
}

fn log_websocket_event_invocation(event: &WebSocketEvent) {
	let (kind, payload_summary) = match event {
		WebSocketEvent::Message {
			data,
			message_index,
		} => {
			let (encoding, bytes) = match data {
				WsMessage::Text(text) => ("text", text.len()),
				WsMessage::Binary(bytes) => ("binary", bytes.len()),
			};
			(
				"websocket.message",
				format!("encoding={encoding} bytes={bytes} message_index={message_index:?}"),
			)
		}
		WebSocketEvent::Close {
			code,
			reason,
			was_clean,
		} => (
			"websocket.close",
			format!(
				"code={code} reason_bytes={} was_clean={was_clean}",
				reason.len()
			),
		),
	};
	tracing::debug!(
		kind,
		payload_summary = %payload_summary,
		"invoking napi TSF callback"
	);
}

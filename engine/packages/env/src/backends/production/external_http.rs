//! Production reqwest and `tokio-tungstenite` clients for HTTP and WebSocket servers outside the
//! cluster.

use crate::capabilities::{
	http::{
		ClientWebSocket, ExternalHttpClient, HttpBody, HttpFuture, Request, Response,
		WebSocketSink, WebSocketStream,
	},
	websocket::{CloseFrame, Message},
};
use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio_tungstenite::tungstenite::{
	self, client::IntoClientRequest, protocol::frame::coding::CloseCode,
};

/// How long a connection attempt to an external server may take. The caller's deadline still
/// bounds the whole request.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Handshake headers the WebSocket adapter writes itself.
const WEBSOCKET_HANDSHAKE_HEADERS: [&str; 6] = [
	"host",
	"connection",
	"upgrade",
	"sec-websocket-version",
	"sec-websocket-key",
	"sec-websocket-extensions",
];

pub struct ExternalReqwestClient {
	client: reqwest::Client,
}

impl ExternalReqwestClient {
	/// A client with no redirects and no request or read timeout. HTTP/2 is used only when TLS
	/// negotiates it.
	pub fn new(user_agent: &str) -> Result<Self> {
		let client = reqwest::Client::builder()
			.user_agent(user_agent)
			.redirect(reqwest::redirect::Policy::none())
			.connect_timeout(CONNECT_TIMEOUT)
			.build()?;
		Ok(Self { client })
	}
}

impl ExternalHttpClient for ExternalReqwestClient {
	fn execute(&self, request: Request<HttpBody>) -> HttpFuture<'_, Response<HttpBody>> {
		Box::pin(async move {
			let mut request = reqwest::Request::try_from(request.map(reqwest::Body::wrap_stream))?;
			*request.timeout_mut() = None;
			let response = self.client.execute(request).await?;

			let mut builder = Response::builder()
				.status(response.status())
				.version(response.version());
			if let Some(headers) = builder.headers_mut() {
				*headers = response.headers().clone();
			}
			let body = response
				.bytes_stream()
				.map(|chunk| chunk.map_err(anyhow::Error::from))
				.boxed();
			Ok(builder.body(body)?)
		})
	}

	fn connect_websocket(&self, request: Request<()>) -> HttpFuture<'_, ClientWebSocket> {
		Box::pin(async move {
			let (parts, ()) = request.into_parts();
			let uri = websocket_uri(&parts.uri)?;
			let mut handshake = uri.into_client_request()?;
			for (name, value) in &parts.headers {
				if WEBSOCKET_HANDSHAKE_HEADERS.contains(&name.as_str()) {
					continue;
				}
				handshake.headers_mut().append(name.clone(), value.clone());
			}

			let (socket, response) =
				tokio_tungstenite::connect_async_with_config(handshake, None, true)
					.await
					.context("external WebSocket handshake failed")?;
			let response = response.map(|_| ());
			let (sink, stream) = socket.split();
			let sink: WebSocketSink = Box::pin(
				sink.sink_map_err(anyhow::Error::from)
					.with(|message: Message| async move { Ok(to_tungstenite(message)) }),
			);
			let stream: WebSocketStream = stream
				.filter_map(|message| async move {
					match message {
						Ok(message) => from_tungstenite(message).map(Ok),
						Err(error) => Some(Err(anyhow::Error::from(error))),
					}
				})
				.boxed();
			Ok(ClientWebSocket {
				response,
				sink,
				stream,
			})
		})
	}
}

/// The `ws` or `wss` form of `uri`, so callers may address a WebSocket by its HTTP URL.
fn websocket_uri(uri: &http::Uri) -> Result<http::Uri> {
	let scheme = match uri.scheme_str() {
		Some("http") | Some("ws") => "ws",
		Some("https") | Some("wss") => "wss",
		Some(other) => bail!("unsupported WebSocket scheme {other}"),
		None => bail!("WebSocket URI has no scheme"),
	};
	let mut parts = uri.clone().into_parts();
	parts.scheme = Some(scheme.parse()?);
	Ok(http::Uri::from_parts(parts)?)
}

fn to_tungstenite(message: Message) -> tungstenite::Message {
	match message {
		Message::Text(text) => tungstenite::Message::text(text.as_str()),
		Message::Binary(bytes) => tungstenite::Message::Binary(bytes),
		Message::Ping(bytes) => tungstenite::Message::Ping(bytes),
		Message::Pong(bytes) => tungstenite::Message::Pong(bytes),
		Message::Close(frame) => {
			tungstenite::Message::Close(frame.map(|frame| tungstenite::protocol::CloseFrame {
				code: CloseCode::from(frame.code),
				reason: frame.reason.as_str().into(),
			}))
		}
	}
}

/// Raw frames are never produced while reading, so they map to nothing.
fn from_tungstenite(message: tungstenite::Message) -> Option<Message> {
	match message {
		tungstenite::Message::Text(text) => Some(Message::Text(text.as_str().into())),
		tungstenite::Message::Binary(bytes) => Some(Message::Binary(bytes)),
		tungstenite::Message::Ping(bytes) => Some(Message::Ping(bytes)),
		tungstenite::Message::Pong(bytes) => Some(Message::Pong(bytes)),
		tungstenite::Message::Close(frame) => Some(Message::Close(frame.map(|frame| CloseFrame {
			code: u16::from(frame.code),
			reason: frame.reason.as_str().into(),
		}))),
		tungstenite::Message::Frame(_) => None,
	}
}

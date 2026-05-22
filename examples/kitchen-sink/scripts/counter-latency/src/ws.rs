// WebSocket connect helper for raw rivet gateway routing. Wraps
// tokio-tungstenite and sets the protocol subheaders the gateway
// expects (`rivet`, `rivet_encoding.json`).

use anyhow::{Context, Result};
use http::HeaderValue;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

pub const RIVET_PROTOCOLS: &[&str] = &["rivet", "rivet_encoding.json"];

pub type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub async fn open_raw_ws(url: &str) -> Result<Ws> {
	let mut req = url.into_client_request().context("invalid websocket URL")?;
	req.headers_mut().insert(
		"Sec-WebSocket-Protocol",
		HeaderValue::from_static("rivet, rivet_encoding.json"),
	);
	let (ws, _resp) = connect_async(req).await.context("websocket connect failed")?;
	Ok(ws)
}

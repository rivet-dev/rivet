use futures::sink::SinkExt;
use futures::stream::StreamExt;
use futures_util::{future, pin_mut};
use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::{Request, Response};
use hyper_tungstenite::{HyperWebsocket, tungstenite};
use hyper_util::rt::TokioIo;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::connect_async;
use tungstenite::Message;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Handle a HTTP or WebSocket request.
async fn handle_request(mut request: Request<Incoming>) -> Result<Response<Full<Bytes>>, Error> {
	// Check if the request is a websocket upgrade request.
	if hyper_tungstenite::is_upgrade_request(&request) {
		let (response, websocket) = hyper_tungstenite::upgrade(&mut request, None)?;

		// Spawn a task to handle the websocket connection.
		tokio::spawn(async move {
			if let Err(e) = serve_websocket(websocket).await {
				eprintln!("Error in websocket connection: {e}");
			}
		});

		// Return the response so the spawned future can continue.
		Ok(response)
	} else {
		// Handle regular HTTP requests here.
		Ok(Response::new(Full::<Bytes>::from("Hello HTTP!")))
	}
}

/// Handle a websocket connection.
async fn serve_websocket(websocket: HyperWebsocket) -> Result<(), Error> {
	let mut websocket = websocket.await?;
	while let Some(message) = websocket.next().await {
		match message? {
			Message::Text(msg) => {
				println!("Received text message: {msg}");
				websocket.send(Message::text(msg)).await?;
			}
			Message::Binary(msg) => {
				println!("Received binary message: {msg:02X?}");
				websocket.send(Message::binary(msg)).await?;
			}
			Message::Ping(msg) => {
				// No need to send a reply: tungstenite takes care of this for you.
				println!("Received ping message: {msg:02X?}");
			}
			Message::Pong(msg) => {
				println!("Received pong message: {msg:02X?}");
			}
			Message::Close(msg) => {
				// No need to send a reply: tungstenite takes care of this for you.
				if let Some(msg) = &msg {
					println!(
						"Received close message with code {} and message: {}",
						msg.code, msg.reason
					);
				} else {
					println!("Received close message");
				}
			}
			Message::Frame(_msg) => {
				unreachable!();
			}
		}
	}

	Ok(())
}

async fn run_server() -> Result<(), Error> {
	let addr: std::net::SocketAddr = "[::1]:3000".parse()?;
	let listener = tokio::net::TcpListener::bind(&addr).await?;
	println!("Listening on http://{addr}");

	let mut http = hyper::server::conn::http1::Builder::new();
	http.keep_alive(true);

	loop {
		let (stream, _) = listener.accept().await?;
		let connection = http
			.serve_connection(
				TokioIo::new(stream),
				hyper::service::service_fn(handle_request),
			)
			.with_upgrades();
		tokio::spawn(async move {
			if let Err(err) = connection.await {
				println!("Error serving HTTP connection: {err:?}");
			}
		});
	}
}

async fn run_client() {
	let (ws_stream, _) = tokio_tungstenite::connect_async("ws://localhost:3000")
		.await
		.expect("Failed to connect");
	println!("WebSocket handshake has been successfully completed");

	let (mut write, mut read) = ws_stream.split();

	write
		.send(tungstenite::protocol::Message::Text("Test".into()))
		.await
		.unwrap();

	let msg = read.next().await.unwrap().unwrap();
	assert_eq!(msg, tungstenite::protocol::Message::Text("Test".into()))
}

#[tokio::test]
async fn ws_e2e() {
	tokio::spawn(async move { run_server().await.unwrap() });

	tokio::time::sleep(std::time::Duration::from_millis(100)).await;
	run_client().await;
}

use std::time::Duration;

use rivetkit_client::{Client, ClientConfig, GetOptions};
use tokio::{net::TcpListener, time::timeout};

/// Accepts TCP connections and holds them open without ever writing.
async fn silent_server() -> (String, tokio::task::JoinHandle<()>) {
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let endpoint = format!("http://{}", listener.local_addr().unwrap());
	let task = tokio::spawn(async move {
		let mut held = Vec::new();
		loop {
			let (stream, _) = listener.accept().await.unwrap();
			held.push(stream);
		}
	});
	(endpoint, task)
}

#[tokio::test]
async fn http_action_errors_when_server_never_responds() {
	let (endpoint, server) = silent_server().await;
	let client = Client::new(
		ClientConfig::new(endpoint)
			.disable_metadata_lookup(true)
			.request_timeout(Duration::from_millis(200)),
	);
	let handle = client
		.get("counter", vec!["a".to_string()], GetOptions::default())
		.unwrap();

	// The outer timeout only exists so a regression fails instead of hanging.
	let result = timeout(Duration::from_secs(3), handle.action("increment", vec![])).await;

	server.abort();
	let result = result.expect("action hung instead of failing with a timeout");
	assert!(result.is_err());
}

#[tokio::test]
async fn websocket_handshake_errors_when_server_never_responds() {
	let (endpoint, server) = silent_server().await;
	let client = Client::new(
		ClientConfig::new(endpoint)
			.disable_metadata_lookup(true)
			.connect_timeout(Duration::from_millis(200)),
	);
	let handle = client
		.get("counter", vec!["a".to_string()], GetOptions::default())
		.unwrap();

	let result = timeout(Duration::from_secs(3), handle.web_socket("/", None)).await;

	server.abort();
	let result = result.expect("websocket handshake hung instead of failing with a timeout");
	assert!(result.is_err());
}

#[test]
fn timeout_defaults_keep_long_requests_working() {
	let config = ClientConfig::new("http://localhost");
	assert_eq!(config.connect_timeout, Duration::from_secs(10));
	assert_eq!(config.request_timeout, None);
}

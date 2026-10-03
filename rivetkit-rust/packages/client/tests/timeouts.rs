use std::time::Duration;

use rivetkit_client::{Client, ClientConfig, EncodingKind, GetOptions};
use tokio::{
	io::{AsyncReadExt, AsyncWriteExt},
	net::TcpListener,
	time::timeout,
};

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

/// Answers every HTTP request with a JSON action output after `delay`.
async fn delayed_action_server(delay: Duration) -> (String, tokio::task::JoinHandle<()>) {
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let endpoint = format!("http://{}", listener.local_addr().unwrap());
	let task = tokio::spawn(async move {
		loop {
			let (mut stream, _) = listener.accept().await.unwrap();
			tokio::spawn(async move {
				let mut buf = [0u8; 4096];
				let _ = stream.read(&mut buf).await;
				tokio::time::sleep(delay).await;
				let body = r#"{"output":42}"#;
				let response = format!(
					"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
					body.len()
				);
				stream.write_all(response.as_bytes()).await.unwrap();
				// Keep the connection open until the client closes it so the
				// response is never reset mid-read.
				while matches!(stream.read(&mut buf).await, Ok(read) if read > 0) {}
			});
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

/// Neither `request_timeout` nor metadata lookup is touched here, so this
/// covers the default path. The peer completes the TCP connect, which ends the
/// connect timeout, and then never answers `/metadata`.
#[tokio::test]
async fn websocket_errors_by_default_when_metadata_never_responds() {
	let (endpoint, server) = silent_server().await;
	let client =
		Client::new(ClientConfig::new(endpoint).control_timeout(Duration::from_millis(200)));
	let handle = client
		.get("counter", vec!["a".to_string()], GetOptions::default())
		.unwrap();

	let result = timeout(Duration::from_secs(3), handle.web_socket("/", None)).await;

	server.abort();
	let result = result.expect("websocket call hung on an unresponsive metadata endpoint");
	assert!(result.is_err());
}

/// Same as above for an Engine actor lookup, which is the call behind
/// `resolve`.
#[tokio::test]
async fn resolve_errors_by_default_when_engine_never_responds() {
	let (endpoint, server) = silent_server().await;
	let client = Client::new(
		ClientConfig::new(endpoint)
			.disable_metadata_lookup(true)
			.control_timeout(Duration::from_millis(200)),
	);
	let handle = client
		.get("counter", vec!["a".to_string()], GetOptions::default())
		.unwrap();

	let result = timeout(Duration::from_secs(3), handle.resolve()).await;

	server.abort();
	let result = result.expect("resolve hung on an unresponsive engine");
	assert!(result.is_err());
}

/// The control timeout must not cut off user requests, which can legitimately
/// run for minutes. This action outlasts the control timeout and succeeds
/// because `request_timeout` is unset.
#[tokio::test]
async fn http_action_is_not_bounded_by_control_timeout() {
	let (endpoint, server) = delayed_action_server(Duration::from_millis(600)).await;
	let client = Client::new(
		ClientConfig::new(endpoint)
			.encoding(EncodingKind::Json)
			.disable_metadata_lookup(true)
			.control_timeout(Duration::from_millis(100)),
	);
	let handle = client
		.get("counter", vec!["a".to_string()], GetOptions::default())
		.unwrap();

	let result = timeout(Duration::from_secs(5), handle.action("increment", vec![])).await;

	server.abort();
	let output = result
		.expect("action hung")
		.expect("action should outlast the control timeout");
	assert_eq!(output, serde_json::json!(42));
}

#[test]
fn timeout_defaults_keep_long_requests_working() {
	let config = ClientConfig::new("http://localhost");
	assert_eq!(config.connect_timeout, Duration::from_secs(10));
	assert_eq!(config.request_timeout, None);
}

#[test]
fn control_timeout_defaults_to_thirty_seconds() {
	let config = ClientConfig::new("http://localhost");
	assert_eq!(config.control_timeout, Duration::from_secs(30));
	assert_eq!(config.request_timeout, None);
}

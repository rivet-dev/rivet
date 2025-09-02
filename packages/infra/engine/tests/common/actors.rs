use serde_json::json;

#[derive(Clone)]
pub struct CreateActorOptions {
	pub namespace: String,
	pub name: String,
	pub key: Option<String>,
	pub input: Option<String>,
	pub runner_name_selector: Option<String>,
	pub durable: bool,
	pub datacenter: Option<String>,
}

impl Default for CreateActorOptions {
	fn default() -> Self {
		Self {
			namespace: "test".to_string(),
			name: "test-actor".to_string(),
			key: Some(rand::random::<u16>().to_string()),
			input: Some(base64::Engine::encode(
				&base64::engine::general_purpose::STANDARD,
				"hello",
			)),
			runner_name_selector: Some("test-runner".to_string()),
			durable: false,
			datacenter: None,
		}
	}
}

pub async fn create_actor_with_options(options: CreateActorOptions, guard_port: u16) -> String {
	tracing::info!(?options.namespace, ?options.name, "creating actor");

	let mut body = json!({
		"name": options.name,
		"key": options.key,
		"crash_policy": if options.durable {
			"restart"
		} else {
			"destroy"
		},
	});

	if let Some(input) = options.input {
		body["input"] = json!(input);
	}
	if let Some(runner_name_selector) = options.runner_name_selector {
		body["runner_name_selector"] = json!(runner_name_selector);
	}
	let mut url = format!(
		"http://127.0.0.1:{}/actors?namespace={}",
		guard_port, options.namespace
	);

	if let Some(datacenter) = &options.datacenter {
		url.push_str(&format!("&datacenter={}", datacenter));
	}

	let client = reqwest::Client::new();
	let response = client
		.post(url)
		.json(&body)
		.send()
		.await
		.expect("Failed to send actor creation request");

	if !response.status().is_success() {
		let text = response.text().await.expect("Failed to read response text");
		panic!("Failed to create actor: {}", text);
	}

	let body: serde_json::Value = response
		.json()
		.await
		.expect("Failed to parse JSON response");
	let actor_id = body["actor"]["actor_id"]
		.as_str()
		.expect("Missing actor_id in response");

	tracing::info!(?actor_id, "actor created");

	actor_id.to_string()
}

pub async fn create_actor(namespace_name: &str, guard_port: u16) -> String {
	create_actor_with_options(
		CreateActorOptions {
			namespace: namespace_name.to_string(),
			..Default::default()
		},
		guard_port,
	)
	.await
}

/// Pings actor via Guard.
pub async fn ping_actor_via_guard(
	guard_port: u16,
	actor_id: &str,
	addr_name: &str,
) -> serde_json::Value {
	tracing::info!(
		?guard_port,
		?actor_id,
		?addr_name,
		"sending request to actor via guard"
	);

	let client = reqwest::Client::new();
	let response = client
		.get(format!("http://127.0.0.1:{}/ping", guard_port))
		.header("X-Rivet-Target", "actor")
		.header("X-Rivet-Actor", actor_id)
		.header("X-Rivet-Port", addr_name)
		.send()
		.await
		.expect("Failed to send ping request through guard");

	if !response.status().is_success() {
		let text = response.text().await.expect("Failed to read response text");
		panic!("Failed to ping actor through guard: {}", text);
	}

	let response = response
		.json()
		.await
		.expect("Failed to parse JSON response");

	tracing::info!(?response, "received response from actor");

	response
}

pub async fn destroy_actor(actor_id: &str, namespace_name: &str, guard_port: u16) {
	let client = reqwest::Client::new();
	let url = format!(
		"http://127.0.0.1:{}/actors/{}?namespace={}",
		guard_port, actor_id, namespace_name
	);

	tracing::info!(?url, "sending delete request");

	let response = client
		.delete(&url)
		.send()
		.await
		.expect("Failed to send delete request");

	let status = response.status();
	let headers = response.headers().clone();
	let text = response.text().await.expect("Failed to read response text");

	tracing::info!(?status, ?headers, ?text, "received response");

	if !status.is_success() {
		panic!("Failed to destroy actor: {}", text);
	}
}

pub async fn destroy_actor_without_namespace(actor_id: &str, guard_port: u16) -> reqwest::Response {
	let client = reqwest::Client::new();
	let url = format!("http://127.0.0.1:{}/actors/{}", guard_port, actor_id);

	tracing::info!(?url, "sending delete request without namespace");

	client
		.delete(&url)
		.send()
		.await
		.expect("Failed to send delete request")
}

pub async fn get_actor(
	actor_id: &str,
	namespace: Option<&str>,
	guard_port: u16,
) -> reqwest::Response {
	let client = reqwest::Client::new();
	let url = if let Some(ns) = namespace {
		format!(
			"http://127.0.0.1:{}/actors/{}?namespace={}",
			guard_port, actor_id, ns
		)
	} else {
		format!("http://127.0.0.1:{}/actors/{}", guard_port, actor_id)
	};

	tracing::info!(?url, "getting actor");

	client
		.get(&url)
		.send()
		.await
		.expect("Failed to send get request")
}

pub async fn get_actor_by_id(
	namespace: &str,
	name: &str,
	key: &str,
	guard_port: u16,
) -> reqwest::Response {
	let client = reqwest::Client::new();
	let url = format!("http://127.0.0.1:{}/actors/by-id", guard_port);

	tracing::info!(?url, ?namespace, ?name, ?key, "getting actor by id");

	client
		.get(&url)
		.query(&[("namespace", namespace), ("name", name), ("key", key)])
		.send()
		.await
		.expect("Failed to send get by id request")
}

pub async fn get_or_create_actor(
	namespace: &str,
	name: &str,
	key: Option<String>,
	durable: bool,
	datacenter: Option<&str>,
	input: Option<String>,
	guard_port: u16,
) -> reqwest::Response {
	let client = reqwest::Client::new();
	let url = format!(
		"http://127.0.0.1:{}/actors?namespace={}",
		guard_port, namespace
	);

	let mut body = json!({
		"name": name,
		"key": key,
		"crash_policy": if durable {
			"restart"
		} else {
			"destroy"
		},
		"runner_name_selector": "test-runner",
	});

	if let Some(input) = input {
		body["input"] = json!(input);
	}
	if let Some(datacenter) = datacenter {
		body["datacenter"] = json!(datacenter);
	}

	tracing::info!(?url, ?body, "get or create actor");

	client
		.put(&url)
		.json(&body)
		.send()
		.await
		.expect("Failed to send get or create request")
}

pub async fn get_or_create_actor_by_id(
	namespace: &str,
	name: &str,
	key: Option<String>,
	datacenter: Option<&str>,
	guard_port: u16,
) -> reqwest::Response {
	let client = reqwest::Client::new();
	let url = format!(
		"http://127.0.0.1:{}/actors/by-id?namespace={}",
		guard_port, namespace
	);

	let mut body = json!({
		"name": name,
		"key": key,
		"runner_name_selector": "test-runner",
	});

	if let Some(datacenter) = datacenter {
		body["datacenter"] = json!(datacenter);
	}

	tracing::info!(?url, ?body, "get or create actor by id");

	client
		.put(&url)
		.json(&body)
		.send()
		.await
		.expect("Failed to send get or create by id request")
}

pub async fn list_actors(
	namespace: &str,
	name: Option<&str>,
	key: Option<String>,
	actor_ids: Option<Vec<String>>,
	include_destroyed: Option<bool>,
	limit: Option<u32>,
	cursor: Option<&str>,
	guard_port: u16,
) -> reqwest::Response {
	let client = reqwest::Client::new();
	let mut url = format!(
		"http://127.0.0.1:{}/actors?namespace={}",
		guard_port, namespace
	);

	if let Some(name) = name {
		url.push_str(&format!("&name={}", name));
	}
	if let Some(key) = key {
		url.push_str(&format!("&key={}", key));
	}
	if let Some(actor_ids) = actor_ids {
		url.push_str(&format!("&actor_ids={}", actor_ids.join(",")));
	}
	if let Some(include_destroyed) = include_destroyed {
		url.push_str(&format!("&include_destroyed={}", include_destroyed));
	}
	if let Some(limit) = limit {
		url.push_str(&format!("&limit={}", limit));
	}
	if let Some(cursor) = cursor {
		url.push_str(&format!("&cursor={}", cursor));
	}

	tracing::info!(?url, "listing actors");

	client
		.get(&url)
		.send()
		.await
		.expect("Failed to send list request")
}

pub async fn list_actor_names(
	namespace: &str,
	limit: Option<u32>,
	cursor: Option<&str>,
	guard_port: u16,
) -> reqwest::Response {
	let client = reqwest::Client::new();
	let mut url = format!(
		"http://127.0.0.1:{}/actors/names?namespace={}",
		guard_port, namespace
	);

	if let Some(limit) = limit {
		url.push_str(&format!("&limit={}", limit));
	}
	if let Some(cursor) = cursor {
		url.push_str(&format!("&cursor={}", cursor));
	}

	tracing::info!(?url, "listing actor names");

	client
		.get(&url)
		.send()
		.await
		.expect("Failed to send list names request")
}

// Test helper functions
pub fn assert_success_response(response: &reqwest::Response) {
	assert!(
		response.status().is_success(),
		"Response not successful: {}",
		response.status()
	);
}

pub async fn assert_error_response(
	response: reqwest::Response,
	expected_error_code: &str,
) -> serde_json::Value {
	assert!(
		!response.status().is_success(),
		"Expected error but got success: {}",
		response.status()
	);

	let body: serde_json::Value = response
		.json()
		.await
		.expect("Failed to parse error response");

	let error_code = body["error"]["code"]
		.as_str()
		.expect("Missing error code in response");
	assert_eq!(
		error_code, expected_error_code,
		"Expected error code {} but got {}",
		expected_error_code, error_code
	);

	body
}

pub fn generate_unique_key() -> String {
	format!("key-{}", rand::random::<u32>())
}

pub async fn bulk_create_actors(
	namespace: &str,
	prefix: &str,
	count: usize,
	guard_port: u16,
) -> Vec<String> {
	let mut actor_ids = Vec::new();
	for i in 0..count {
		let actor_id = create_actor_with_options(
			CreateActorOptions {
				namespace: namespace.to_string(),
				name: format!("{}-{}", prefix, i),
				key: Some(generate_unique_key()),
				..Default::default()
			},
			guard_port,
		)
		.await;
		actor_ids.push(actor_id);
	}
	actor_ids
}

/// Tests WebSocket connection to actor via Guard using a simple ping pong.
pub async fn ping_actor_websocket_via_guard(
	guard_port: u16,
	actor_id: &str,
	addr_name: &str,
) -> serde_json::Value {
	use tokio_tungstenite::{
		connect_async,
		tungstenite::{Message, client::IntoClientRequest},
	};

	tracing::info!(
		?guard_port,
		?actor_id,
		?addr_name,
		"testing websocket connection to actor via guard"
	);

	// Build WebSocket URL and request
	let ws_url = format!("ws://127.0.0.1:{}/ws", guard_port);
	let mut request = ws_url
		.clone()
		.into_client_request()
		.expect("Failed to create WebSocket request");

	// Add headers for routing through guard to actor
	request
		.headers_mut()
		.insert("X-Rivet-Target", "actor".parse().unwrap());
	request
		.headers_mut()
		.insert("X-Rivet-Actor", actor_id.parse().unwrap());
	request
		.headers_mut()
		.insert("X-Rivet-Port", addr_name.parse().unwrap());

	// Connect to WebSocket
	let (ws_stream, response) = connect_async(request)
		.await
		.expect("Failed to connect to WebSocket");

	// Verify connection was successful
	assert_eq!(
		response.status(),
		101,
		"Expected WebSocket upgrade status 101"
	);

	tracing::info!("websocket connected successfully");

	use futures_util::{SinkExt, StreamExt};
	let (mut write, mut read) = ws_stream.split();

	// Send a ping message to verify the connection works
	let ping_message = "ping";
	tracing::info!(?ping_message, "sending ping message");
	write
		.send(Message::Text(ping_message.to_string().into()))
		.await
		.expect("Failed to send ping message");

	// Wait for response with timeout
	let response = tokio::time::timeout(tokio::time::Duration::from_secs(5), read.next())
		.await
		.expect("Timeout waiting for WebSocket response")
		.expect("WebSocket stream ended unexpectedly");

	// Verify response
	let response_text = match response {
		Ok(Message::Text(text)) => {
			let text_str = text.to_string();
			tracing::info!(?text_str, "received response from actor");
			text_str
		}
		Ok(msg) => {
			panic!("Unexpected message type: {:?}", msg);
		}
		Err(e) => {
			panic!("Failed to receive message: {}", e);
		}
	};

	// Verify the response matches expected echo pattern
	let expected_response = "Echo: ping";
	assert_eq!(
		response_text, expected_response,
		"Expected '{}' but got '{}'",
		expected_response, response_text
	);

	// Send another message to test multiple round trips
	let test_message = "hello world";
	tracing::info!(?test_message, "sending test message");
	write
		.send(Message::Text(test_message.to_string().into()))
		.await
		.expect("Failed to send test message");

	// Wait for second response
	let response2 = tokio::time::timeout(tokio::time::Duration::from_secs(5), read.next())
		.await
		.expect("Timeout waiting for second WebSocket response")
		.expect("WebSocket stream ended unexpectedly");

	// Verify second response
	let response2_text = match response2 {
		Ok(Message::Text(text)) => {
			let text_str = text.to_string();
			tracing::info!(?text_str, "received second response from actor");
			text_str
		}
		Ok(msg) => {
			panic!("Unexpected message type for second response: {:?}", msg);
		}
		Err(e) => {
			panic!("Failed to receive second message: {}", e);
		}
	};

	let expected_response2 = format!("Echo: {}", test_message);
	assert_eq!(
		response2_text, expected_response2,
		"Expected '{}' but got '{}'",
		expected_response2, response2_text
	);

	// Close the connection gracefully
	write
		.send(Message::Close(None))
		.await
		.expect("Failed to send close message");

	tracing::info!("websocket bidirectional test completed successfully");

	// Return success response
	json!({
		"status": "ok",
		"message": "WebSocket bidirectional messaging tested successfully"
	})
}

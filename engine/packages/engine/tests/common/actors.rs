#![allow(dead_code, unused_variables, unused_imports)]
use std::str::FromStr;

use serde_json::json;
use url::Url;

use super::{TEST_RUNNER_NAME, TestDatacenter, api, api_types};
use anyhow::{Result, anyhow};

/// Pings actor via Guard.
pub async fn ping_actor_via_guard(dc: &TestDatacenter, actor_id: &str) -> serde_json::Value {
	let guard_port = dc.guard_port();

	tracing::info!(?guard_port, ?actor_id, "sending request to actor via guard");

	let client = reqwest::Client::new();
	let response = client
		.get(format!("http://localhost:{}/ping", guard_port))
		.header("X-Rivet-Target", "actor")
		.header("X-Rivet-Actor", actor_id)
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

pub async fn try_get_actor(
	port: u16,
	actor_id: &str,
	namespace: &str,
) -> Result<Option<rivet_types::actors::Actor>> {
	let res = api::public::actors_list(
		port,
		api_types::actors::list::ListQuery {
			actor_ids: Some(actor_id.to_string()),
			actor_id: vec![],
			namespace: namespace.to_string(),
			name: None,
			key: None,
			include_destroyed: Some(true),
			limit: None,
			cursor: None,
		},
	)
	.await?;

	Ok(res.actors.first().map(|f| f.clone()))
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

	// Error is at root level, not under "error" key
	let error_code = body["code"]
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
	port: u16,
	namespace: &str,
	prefix: &str,
	count: usize,
) -> Vec<rivet_util::Id> {
	let mut actor_ids = Vec::new();
	for i in 0..count {
		let res = api::public::actors_create(
			port,
			api_types::actors::create::CreateQuery {
				namespace: namespace.to_string(),
			},
			api_types::actors::create::CreateRequest {
				datacenter: None,
				name: format!("{}-{}", prefix, i),
				key: Some(generate_unique_key()),
				input: None,
				runner_name_selector: TEST_RUNNER_NAME.to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");
		actor_ids.push(res.actor.actor_id.clone());
	}
	actor_ids
}

/// Tests WebSocket connection to actor via Guard using a simple ping pong.
pub async fn ping_actor_websocket_via_guard(
	dc: &TestDatacenter,
	actor_id: &str,
) -> serde_json::Value {
	use tokio_tungstenite::{
		connect_async,
		tungstenite::{Message, client::IntoClientRequest},
	};

	tracing::info!(
		guard_port=%dc.guard_port(),
		?actor_id,
		"testing websocket connection to actor via guard"
	);

	// Build WebSocket URL and request with protocols for routing
	let ws_url = format!("ws://localhost:{}/ws", dc.guard_port());
	let mut request = ws_url
		.clone()
		.into_client_request()
		.expect("Failed to create WebSocket request");

	// Add protocols for routing through guard to actor
	// URL encode the actor ID since colons are not allowed in WebSocket protocol names
	request.headers_mut().insert(
		"Sec-WebSocket-Protocol",
		format!(
			"rivet, rivet_target.actor, rivet_actor.{}",
			urlencoding::encode(&actor_id)
		)
		.parse()
		.unwrap(),
	);

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
	let response_text = match response.map_err(|e| anyhow!("{}", e)) {
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
		.expect("WebSocket stream ended unexpectedly")
		.map_err(anyhow::Error::msg);

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

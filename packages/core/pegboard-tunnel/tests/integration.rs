use std::time::Duration;

use anyhow::*;
use futures::{SinkExt, StreamExt};
use pegboard::pubsub_subjects::{TunnelHttpResponseSubject, TunnelHttpRunnerSubject};
use rivet_config::Config;
use rivet_pools::Pools;
use rivet_tunnel_protocol::{
	MessageBody, ToClientResponseStart, ToServerRequestStart, TunnelMessage, versioned,
};
use rivet_util::{Id, serde::HashableMap};
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};
use tokio_tungstenite::{
	MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message as WsMessage,
};
use universalpubsub::PubSub;

#[tokio::test]
async fn test_tunnel_bidirectional_forwarding() -> Result<()> {
	// Setup test infrastructure
	let test_port = 16425; // Use a fixed test port
	let mut root = rivet_config::config::Root::default();
	root.pegboard = Some(rivet_config::config::pegboard::Pegboard {
		port: Some(test_port),
		host: None,
		lan_host: None,
	});

	let config = Config::from_root(root);
	let pools = Pools::new(config.clone()).await?;
	let ups = pools.ups()?;

	// Start tunnel server in background
	let tunnel_config = config.clone();
	let tunnel_pools = pools.clone();
	let tunnel_handle =
		tokio::spawn(async move { pegboard_tunnel::start(tunnel_config, tunnel_pools).await });

	// Give server time to start
	sleep(Duration::from_secs(1)).await;

	// Connect WebSocket client
	let ws_url = format!("ws://127.0.0.1:{}", test_port);

	let (mut ws_stream, _) = connect_async(&ws_url).await?;

	// Use the same placeholders as the tunnel implementation
	// TODO: Update when tunnel properly extracts these from connection
	let runner_id = Id::nil();
	let port_name = "default";

	// Give tunnel time to set up NATS subscription after WebSocket connection
	sleep(Duration::from_secs(1)).await;

	// Test 1: NATS to WebSocket forwarding
	test_nats_to_websocket(&ups, &mut ws_stream, runner_id, port_name).await?;

	// Test 2: WebSocket to NATS forwarding
	test_websocket_to_nats(&ups, &mut ws_stream, runner_id).await?;

	// Clean up
	tunnel_handle.abort();

	Ok(())
}

async fn test_nats_to_websocket(
	ups: &PubSub,
	ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
	runner_id: Id,
	port_name: &str,
) -> Result<()> {
	// Create a test request message
	let request_id = rand::random::<u64>();
	let mut headers = HashableMap::new();
	headers.insert("content-type".to_string(), "application/json".to_string());

	let request_start = ToServerRequestStart {
		request_id,
		actor_id: runner_id.to_string(),
		method: "GET".to_string(),
		path: "/test".to_string(),
		headers,
		body: Some(b"test body".to_vec()),
		stream: false,
	};

	let message = TunnelMessage {
		body: MessageBody::ToServerRequestStart(request_start),
	};

	// Serialize the message
	let serialized = versioned::TunnelMessage::serialize(versioned::TunnelMessage::V1(message))?;

	// Publish to NATS topic using proper subject
	let topic = TunnelHttpRunnerSubject::new(&runner_id.to_string(), port_name).to_string();
	ups.request(&topic, &serialized).await?;

	// Wait for message on WebSocket
	let received = timeout(Duration::from_secs(10), ws_stream.next())
		.await?
		.ok_or_else(|| anyhow!("WebSocket stream ended unexpectedly"))?;

	match received? {
		WsMessage::Binary(data) => {
			// Deserialize and verify the message
			let tunnel_msg = versioned::TunnelMessage::deserialize(&data)?;
			match tunnel_msg.body {
				MessageBody::ToServerRequestStart(req) => {
					assert_eq!(req.request_id, request_id);
					assert_eq!(req.method, "GET");
					assert_eq!(req.path, "/test");
					println!("✓ NATS to WebSocket forwarding successful");
				}
				_ => bail!("Unexpected message type received"),
			}
		}
		_ => bail!("Expected binary WebSocket message"),
	}

	Ok(())
}

async fn test_websocket_to_nats(
	ups: &PubSub,
	ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
	runner_id: Id,
) -> Result<()> {
	// Create a response message to send from WebSocket
	let request_id = rand::random::<u64>();

	// Subscribe to response topic with the same request_id
	let response_topic =
		TunnelHttpResponseSubject::new(&runner_id.to_string(), "default", request_id).to_string();
	let mut subscriber = ups.subscribe(&response_topic).await?;
	let mut headers = HashableMap::new();
	headers.insert("content-type".to_string(), "text/plain".to_string());

	let response_start = ToClientResponseStart {
		request_id,
		status: 200,
		headers,
		body: Some(b"response body".to_vec()),
		stream: false,
	};

	let message = TunnelMessage {
		body: MessageBody::ToClientResponseStart(response_start),
	};

	// Serialize and send via WebSocket
	let serialized = versioned::TunnelMessage::serialize(versioned::TunnelMessage::V1(message))?;
	ws_stream.send(WsMessage::Binary(serialized.into())).await?;

	// Wait for message on NATS
	let received = timeout(Duration::from_secs(10), subscriber.next()).await??;

	match received {
		universalpubsub::pubsub::NextOutput::Message(msg) => {
			// Deserialize and verify the message
			let tunnel_msg = versioned::TunnelMessage::deserialize(&msg.payload)?;
			match tunnel_msg.body {
				MessageBody::ToClientResponseStart(resp) => {
					assert_eq!(resp.request_id, request_id);
					assert_eq!(resp.status, 200);
					println!("✓ WebSocket to NATS forwarding successful");
				}
				_ => bail!("Unexpected message type received"),
			}
		}
		_ => bail!("Expected message from NATS"),
	}

	Ok(())
}

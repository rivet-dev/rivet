use std::{sync::Arc, time::Duration};

use pegboard::pubsub_subjects::GatewayReceiverSubject;
use rivet_runner_protocol as protocol;
use scc::HashMap;
use universalpubsub::{NextOutput, PubSub, driver::memory::MemoryDriver};

use super::{handle_tunnel_message_mk1, handle_tunnel_message_mk2};

fn memory_pubsub(channel: &str) -> PubSub {
	PubSub::new(Arc::new(MemoryDriver::new(channel.to_string())))
}

fn response_abort_message_mk2(
	gateway_id: protocol::mk2::GatewayId,
	request_id: protocol::mk2::RequestId,
) -> protocol::mk2::ToServerTunnelMessage {
	protocol::mk2::ToServerTunnelMessage {
		message_id: protocol::mk2::MessageId {
			gateway_id,
			request_id,
			message_index: 0,
		},
		message_kind: protocol::mk2::ToServerTunnelMessageKind::ToServerResponseAbort,
	}
}

fn response_start_message_mk2(
	gateway_id: protocol::mk2::GatewayId,
	request_id: protocol::mk2::RequestId,
) -> protocol::mk2::ToServerTunnelMessage {
	response_start_message_mk2_with_stream(gateway_id, request_id, false)
}

fn response_start_message_mk2_with_stream(
	gateway_id: protocol::mk2::GatewayId,
	request_id: protocol::mk2::RequestId,
	stream: bool,
) -> protocol::mk2::ToServerTunnelMessage {
	protocol::mk2::ToServerTunnelMessage {
		message_id: protocol::mk2::MessageId {
			gateway_id,
			request_id,
			message_index: 0,
		},
		message_kind: protocol::mk2::ToServerTunnelMessageKind::ToServerResponseStart(
			protocol::mk2::ToServerResponseStart {
				status: 200,
				headers: Default::default(),
				body: None,
				stream,
			},
		),
	}
}

fn response_chunk_message_mk2(
	gateway_id: protocol::mk2::GatewayId,
	request_id: protocol::mk2::RequestId,
	finish: bool,
) -> protocol::mk2::ToServerTunnelMessage {
	protocol::mk2::ToServerTunnelMessage {
		message_id: protocol::mk2::MessageId {
			gateway_id,
			request_id,
			message_index: 0,
		},
		message_kind: protocol::mk2::ToServerTunnelMessageKind::ToServerResponseChunk(
			protocol::mk2::ToServerResponseChunk {
				body: b"chunk".to_vec(),
				finish,
			},
		),
	}
}

fn websocket_message_mk2(
	gateway_id: protocol::mk2::GatewayId,
	request_id: protocol::mk2::RequestId,
) -> protocol::mk2::ToServerTunnelMessage {
	protocol::mk2::ToServerTunnelMessage {
		message_id: protocol::mk2::MessageId {
			gateway_id,
			request_id,
			message_index: 0,
		},
		message_kind: protocol::mk2::ToServerTunnelMessageKind::ToServerWebSocketMessage(
			protocol::mk2::ToServerWebSocketMessage {
				data: b"ping".to_vec(),
				binary: false,
			},
		),
	}
}

fn response_abort_message_mk1(
	gateway_id: protocol::mk2::GatewayId,
	request_id: protocol::mk2::RequestId,
) -> protocol::ToServerTunnelMessage {
	protocol::ToServerTunnelMessage {
		message_id: protocol::MessageId {
			gateway_id,
			request_id,
			message_index: 0,
		},
		message_kind: protocol::ToServerTunnelMessageKind::ToServerResponseAbort,
	}
}

fn websocket_message_mk1(
	gateway_id: protocol::mk2::GatewayId,
	request_id: protocol::mk2::RequestId,
) -> protocol::ToServerTunnelMessage {
	protocol::ToServerTunnelMessage {
		message_id: protocol::MessageId {
			gateway_id,
			request_id,
			message_index: 0,
		},
		message_kind: protocol::ToServerTunnelMessageKind::ToServerWebSocketMessage(
			protocol::ToServerWebSocketMessage {
				data: b"ping".to_vec(),
				binary: false,
			},
		),
	}
}

fn response_start_message_mk1(
	gateway_id: protocol::mk2::GatewayId,
	request_id: protocol::mk2::RequestId,
) -> protocol::ToServerTunnelMessage {
	response_start_message_mk1_with_stream(gateway_id, request_id, false)
}

fn response_start_message_mk1_with_stream(
	gateway_id: protocol::mk2::GatewayId,
	request_id: protocol::mk2::RequestId,
	stream: bool,
) -> protocol::ToServerTunnelMessage {
	protocol::ToServerTunnelMessage {
		message_id: protocol::MessageId {
			gateway_id,
			request_id,
			message_index: 0,
		},
		message_kind: protocol::ToServerTunnelMessageKind::ToServerResponseStart(
			protocol::ToServerResponseStart {
				status: 200,
				headers: Default::default(),
				body: None,
				stream,
			},
		),
	}
}

fn response_chunk_message_mk1(
	gateway_id: protocol::mk2::GatewayId,
	request_id: protocol::mk2::RequestId,
	finish: bool,
) -> protocol::ToServerTunnelMessage {
	protocol::ToServerTunnelMessage {
		message_id: protocol::MessageId {
			gateway_id,
			request_id,
			message_index: 0,
		},
		message_kind: protocol::ToServerTunnelMessageKind::ToServerResponseChunk(
			protocol::ToServerResponseChunk {
				body: b"chunk".to_vec(),
				finish,
			},
		),
	}
}

#[tokio::test]
async fn rejects_unissued_mk2_tunnel_message_pairs() {
	let pubsub = memory_pubsub("pegboard-runner-ws-to-tunnel-test-reject-mk2");
	let gateway_id = [1, 2, 3, 4];
	let request_id = [5, 6, 7, 8];
	let mut sub = pubsub
		.subscribe(&GatewayReceiverSubject::new(gateway_id).to_string())
		.await
		.unwrap();
	let authorized_tunnel_routes = HashMap::new();

	let err = handle_tunnel_message_mk2(
		&pubsub,
		1024,
		&authorized_tunnel_routes,
		response_abort_message_mk2(gateway_id, request_id),
	)
	.await
	.unwrap_err();
	assert!(err.to_string().contains("unauthorized tunnel message"));

	let recv = tokio::time::timeout(Duration::from_millis(50), sub.next()).await;
	assert!(recv.is_err());
}

#[tokio::test]
async fn republishes_issued_mk2_tunnel_message_pairs() {
	let pubsub = memory_pubsub("pegboard-runner-ws-to-tunnel-test-allow-mk2");
	let gateway_id = [9, 10, 11, 12];
	let request_id = [13, 14, 15, 16];
	let mut sub = pubsub
		.subscribe(&GatewayReceiverSubject::new(gateway_id).to_string())
		.await
		.unwrap();
	let authorized_tunnel_routes = HashMap::new();
	let _ = authorized_tunnel_routes
		.insert_async((gateway_id, request_id), ())
		.await;

	handle_tunnel_message_mk2(
		&pubsub,
		1024,
		&authorized_tunnel_routes,
		websocket_message_mk2(gateway_id, request_id),
	)
	.await
	.unwrap();

	let msg = tokio::time::timeout(Duration::from_secs(1), sub.next())
		.await
		.unwrap()
		.unwrap();
	assert!(matches!(msg, NextOutput::Message(_)));
	assert!(
		authorized_tunnel_routes
			.contains_async(&(gateway_id, request_id))
			.await
	);
}

#[tokio::test]
async fn rejects_unissued_mk1_tunnel_message_pairs() {
	let pubsub = memory_pubsub("pegboard-runner-ws-to-tunnel-test-reject-mk1");
	let gateway_id = [17, 18, 19, 20];
	let request_id = [21, 22, 23, 24];
	let mut sub = pubsub
		.subscribe(&GatewayReceiverSubject::new(gateway_id).to_string())
		.await
		.unwrap();
	let authorized_tunnel_routes = HashMap::new();

	let err = handle_tunnel_message_mk1(
		&pubsub,
		1024,
		&authorized_tunnel_routes,
		response_abort_message_mk1(gateway_id, request_id),
	)
	.await
	.unwrap_err();
	assert!(err.to_string().contains("unauthorized tunnel message"));

	let recv = tokio::time::timeout(Duration::from_millis(50), sub.next()).await;
	assert!(recv.is_err());
}

#[tokio::test]
async fn republishes_issued_mk1_tunnel_message_pairs() {
	let pubsub = memory_pubsub("pegboard-runner-ws-to-tunnel-test-allow-mk1");
	let gateway_id = [25, 26, 27, 28];
	let request_id = [29, 30, 31, 32];
	let mut sub = pubsub
		.subscribe(&GatewayReceiverSubject::new(gateway_id).to_string())
		.await
		.unwrap();
	let authorized_tunnel_routes = HashMap::new();
	let _ = authorized_tunnel_routes
		.insert_async((gateway_id, request_id), ())
		.await;

	handle_tunnel_message_mk1(
		&pubsub,
		1024,
		&authorized_tunnel_routes,
		websocket_message_mk1(gateway_id, request_id),
	)
	.await
	.unwrap();

	let msg = tokio::time::timeout(Duration::from_secs(1), sub.next())
		.await
		.unwrap()
		.unwrap();
	assert!(matches!(msg, NextOutput::Message(_)));
	assert!(
		authorized_tunnel_routes
			.contains_async(&(gateway_id, request_id))
			.await
	);
}

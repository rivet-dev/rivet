// TODO: Use TestCtx
// use std::{sync::Arc, time::Duration};

// use pegboard::pubsub_subjects::GatewayReceiverSubject;
// use rivet_envoy_protocol as protocol;
// use scc::HashMap;
// use universalpubsub::{NextOutput, PubSub, driver::memory::MemoryDriver};

// use super::handle_tunnel_message;

// fn memory_pubsub(channel: &str) -> PubSub {
// 	PubSub::new(Arc::new(MemoryDriver::new(channel.to_string())))
// }

// fn response_abort_message(
// 	gateway_id: protocol::GatewayId,
// 	request_id: protocol::RequestId,
// ) -> protocol::ToRivetTunnelMessage {
// 	protocol::ToRivetTunnelMessage {
// 		message_id: protocol::MessageId {
// 			gateway_id,
// 			request_id,
// 			message_index: 0,
// 		},
// 		message_kind: protocol::ToRivetTunnelMessageKind::ToRivetResponseAbort,
// 	}
// }

// #[tokio::test]
// async fn rejects_unissued_tunnel_message_pairs() {
// 	let pubsub = memory_pubsub("pegboard-envoy-ws-to-tunnel-test-reject");
// 	let gateway_id = [1, 2, 3, 4];
// 	let request_id = [5, 6, 7, 8];
// 	let mut sub = pubsub
// 		.subscribe(&GatewayReceiverSubject::new(gateway_id).to_string())
// 		.await
// 		.unwrap();
// 	let authorized_tunnel_routes = HashMap::new();

// 	let err = handle_tunnel_message(
// 		&pubsub,
// 		1024,
// 		&authorized_tunnel_routes,
// 		response_abort_message(gateway_id, request_id),
// 	)
// 	.await
// 	.unwrap_err();
// 	assert!(err.to_string().contains("unauthorized tunnel message"));

// 	let recv = tokio::time::timeout(Duration::from_millis(50), sub.next()).await;
// 	assert!(recv.is_err());
// }

// #[tokio::test]
// async fn republishes_issued_tunnel_message_pairs() {
// 	let pubsub = memory_pubsub("pegboard-envoy-ws-to-tunnel-test-allow");
// 	let gateway_id = [9, 10, 11, 12];
// 	let request_id = [13, 14, 15, 16];
// 	let mut sub = pubsub
// 		.subscribe(&GatewayReceiverSubject::new(gateway_id).to_string())
// 		.await
// 		.unwrap();
// 	let authorized_tunnel_routes = HashMap::new();
// 	let _ = authorized_tunnel_routes
// 		.insert_async((gateway_id, request_id), ())
// 		.await;

// 	handle_tunnel_message(
// 		&pubsub,
// 		1024,
// 		&authorized_tunnel_routes,
// 		response_abort_message(gateway_id, request_id),
// 	)
// 	.await
// 	.unwrap();

// 	let msg = tokio::time::timeout(Duration::from_secs(1), sub.next())
// 		.await
// 		.unwrap()
// 		.unwrap();
// 	assert!(matches!(msg, NextOutput::Message(_)));
// }

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

use sqlite_storage::error::SqliteStorageError;

use super::{
	actor_lifecycle::{ActiveActor, ActiveActorState},
	cached_active_sqlite_actor, cached_serverless_sqlite_generation,
	validate_sqlite_get_page_range_request,
};

#[tokio::test]
async fn cached_active_sqlite_actor_accepts_running_actor_generation() {
	let active_actors = scc::HashMap::new();
	active_actors
		.insert_async(
			"actor-a".to_string(),
			ActiveActor {
				actor_generation: 1,
				sqlite_generation: Some(7),
				state: ActiveActorState::Running,
			},
		)
		.await
		.expect("insert active actor");

	assert!(cached_active_sqlite_actor(&active_actors, "actor-a", 7).await);
	assert!(!cached_active_sqlite_actor(&active_actors, "actor-a", 8).await);
	assert!(!cached_active_sqlite_actor(&active_actors, "actor-b", 7).await);
}

#[tokio::test]
async fn cached_active_sqlite_actor_rejects_starting_actor() {
	let active_actors = scc::HashMap::new();
	active_actors
		.insert_async(
			"actor-a".to_string(),
			ActiveActor {
				actor_generation: 1,
				sqlite_generation: Some(7),
				state: ActiveActorState::Starting,
			},
		)
		.await
		.expect("insert active actor");

	assert!(!cached_active_sqlite_actor(&active_actors, "actor-a", 7).await);
}

#[tokio::test]
async fn cached_serverless_sqlite_generation_accepts_matching_generation() {
	let serverless_sqlite_actors = scc::HashMap::new();
	serverless_sqlite_actors
		.insert_async("actor-a".to_string(), 7)
		.await
		.expect("insert serverless actor");

	assert!(
		cached_serverless_sqlite_generation(&serverless_sqlite_actors, "actor-a", 7)
			.await
			.expect("matching cached generation succeeds")
	);
	assert!(
		!cached_serverless_sqlite_generation(&serverless_sqlite_actors, "actor-b", 7)
			.await
			.expect("missing cached generation falls back")
	);
}

#[tokio::test]
async fn cached_serverless_sqlite_generation_reports_fence_mismatch() {
	let serverless_sqlite_actors = scc::HashMap::new();
	serverless_sqlite_actors
		.insert_async("actor-a".to_string(), 7)
		.await
		.expect("insert serverless actor");

	let err = cached_serverless_sqlite_generation(&serverless_sqlite_actors, "actor-a", 8)
		.await
		.expect_err("stale generation should be fenced");

	assert!(matches!(
		err.downcast_ref::<SqliteStorageError>(),
		Some(SqliteStorageError::FenceMismatch { .. })
	));
	assert!(
		err.to_string()
			.contains("did not match cached generation 7")
	);
}

#[test]
fn validate_sqlite_get_page_range_request_rejects_empty_bounds() {
	let valid = rivet_envoy_protocol::SqliteGetPageRangeRequest {
		actor_id: "actor-a".to_string(),
		generation: 7,
		start_pgno: 1,
		max_pages: 1,
		max_bytes: 4096,
	};

	validate_sqlite_get_page_range_request(&valid).expect("valid range request");

	let mut invalid = valid.clone();
	invalid.start_pgno = 0;
	assert!(validate_sqlite_get_page_range_request(&invalid).is_err());

	let mut invalid = valid.clone();
	invalid.max_pages = 0;
	assert!(validate_sqlite_get_page_range_request(&invalid).is_err());

	let mut invalid = valid;
	invalid.max_bytes = 0;
	assert!(validate_sqlite_get_page_range_request(&invalid).is_err());
}

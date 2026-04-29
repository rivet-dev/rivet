use std::sync::Arc;

use anyhow::{Context, Result};
use gas::prelude::util::serde::HashableMap;
use pegboard::{
	actor_lifecycle::{
		self, ActorSuspension, RESTORE_HTTP_RETRY_AFTER_SECONDS, RESTORE_WS_CLOSE_CODE,
		RESTORE_WS_CLOSE_REASON,
	},
	pubsub_subjects::GatewayReceiverSubject,
};
use pegboard_envoy::restore_lifecycle;
use rivet_envoy_protocol::{self as protocol, versioned};
use scc::HashMap;
use tempfile::Builder;
use universalpubsub::{NextOutput, PubSub, Subscriber, driver::memory::MemoryDriver};
use uuid::Uuid;
use vbare::OwnedVersionedData;

type Routes = HashMap<restore_lifecycle::RouteKey, restore_lifecycle::RouteState>;

const ACTOR_ID: &str = "restore-lifecycle-actor";

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new()
		.prefix("pegboard-envoy-restore-lifecycle-")
		.tempdir()?
		.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

fn test_ups(name: &str) -> PubSub {
	PubSub::new(Arc::new(MemoryDriver::new(name.to_string())))
}

fn gateway_id() -> protocol::GatewayId {
	[1, 2, 3, 4]
}

fn request_id(value: u8) -> protocol::RequestId {
	[value, 0, 0, 0]
}

fn message_id(value: u8) -> protocol::MessageId {
	protocol::MessageId {
		gateway_id: gateway_id(),
		request_id: request_id(value),
		message_index: 0,
	}
}

fn ws_open(value: u8, actor_id: &str) -> protocol::ToEnvoyTunnelMessage {
	protocol::ToEnvoyTunnelMessage {
		message_id: message_id(value),
		message_kind: protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(
			protocol::ToEnvoyWebSocketOpen {
				actor_id: actor_id.to_string(),
				path: "/".to_string(),
				headers: HashableMap::new(),
			},
		),
	}
}

fn http_start(value: u8, actor_id: &str) -> protocol::ToEnvoyTunnelMessage {
	protocol::ToEnvoyTunnelMessage {
		message_id: message_id(value),
		message_kind: protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(
			protocol::ToEnvoyRequestStart {
				actor_id: actor_id.to_string(),
				method: "GET".to_string(),
				path: "/".to_string(),
				headers: HashableMap::new(),
				body: None,
				stream: false,
			},
		),
	}
}

async fn suspend(db: &universaldb::Database, ups: &PubSub) -> Result<ActorSuspension> {
	actor_lifecycle::suspend_actor(
		db,
		ups,
		ACTOR_ID.to_string(),
		actor_lifecycle::RESTORE_SUSPENSION_REASON,
		Uuid::new_v4(),
	)
	.await
}

async fn gateway_sub(ups: &PubSub) -> Result<Subscriber> {
	ups
		.subscribe(&GatewayReceiverSubject::new(gateway_id()).to_string())
		.await
		.map_err(Into::into)
}

async fn recv_gateway(sub: &mut Subscriber) -> Result<protocol::ToRivetTunnelMessage> {
	let msg = tokio::time::timeout(std::time::Duration::from_secs(1), sub.next())
		.await
		.context("timed out waiting for gateway message")??;
	let NextOutput::Message(msg) = msg else {
		anyhow::bail!("gateway subscription ended");
	};
	let protocol::ToGateway::ToRivetTunnelMessage(msg) =
		versioned::ToGateway::deserialize_with_embedded_version(&msg.payload)?
	else {
		anyhow::bail!("expected tunnel message");
	};
	Ok(msg)
}

#[tokio::test]
async fn suspend_closes_existing_ws_with_1012() -> Result<()> {
	let ups = test_ups("restore-close-existing");
	let routes = Routes::new();
	let suspension = ActorSuspension {
		actor_id: ACTOR_ID.to_string(),
		reason: actor_lifecycle::RESTORE_SUSPENSION_REASON.to_string(),
		op_id: Uuid::new_v4(),
		suspended_at_ms: 1,
	};
	routes
		.insert_async(
			(gateway_id(), request_id(1)),
			restore_lifecycle::RouteState {
				actor_id: ACTOR_ID.to_string(),
				kind: restore_lifecycle::RouteKind::WebSocket,
			},
		)
		.await
		.expect("route should insert");

	let mut sub = gateway_sub(&ups).await?;
	restore_lifecycle::close_websocket_routes_for_suspension(&ups, &routes, &suspension).await?;
	let msg = recv_gateway(&mut sub).await?;

	let protocol::ToRivetTunnelMessageKind::ToRivetWebSocketClose(close) = msg.message_kind else {
		anyhow::bail!("expected ws close");
	};
	assert_eq!(close.code, Some(RESTORE_WS_CLOSE_CODE));
	assert_eq!(close.reason.as_deref(), Some(RESTORE_WS_CLOSE_REASON));
	Ok(())
}

#[tokio::test]
async fn suspend_rejects_new_ws_with_1012() -> Result<()> {
	let db = test_db().await?;
	let ups = test_ups("restore-reject-ws");
	let routes = Routes::new();
	suspend(&db, &ups).await?;
	let mut sub = gateway_sub(&ups).await?;

	let rejected = restore_lifecycle::maybe_reject_suspended_tunnel_message(
		&db,
		&ups,
		&routes,
		&ws_open(2, ACTOR_ID),
	)
	.await?;
	let msg = recv_gateway(&mut sub).await?;

	assert!(rejected);
	let protocol::ToRivetTunnelMessageKind::ToRivetWebSocketClose(close) = msg.message_kind else {
		anyhow::bail!("expected ws close");
	};
	assert_eq!(close.code, Some(RESTORE_WS_CLOSE_CODE));
	assert_eq!(close.reason.as_deref(), Some(RESTORE_WS_CLOSE_REASON));
	Ok(())
}

#[tokio::test]
async fn suspend_returns_503_for_http() -> Result<()> {
	let db = test_db().await?;
	let ups = test_ups("restore-reject-http");
	let routes = Routes::new();
	suspend(&db, &ups).await?;
	let mut sub = gateway_sub(&ups).await?;

	let rejected = restore_lifecycle::maybe_reject_suspended_tunnel_message(
		&db,
		&ups,
		&routes,
		&http_start(3, ACTOR_ID),
	)
	.await?;
	let msg = recv_gateway(&mut sub).await?;

	assert!(rejected);
	let protocol::ToRivetTunnelMessageKind::ToRivetResponseStart(res) = msg.message_kind else {
		anyhow::bail!("expected http response");
	};
	assert_eq!(res.status, 503);
	assert_eq!(
		res.headers.get("retry-after").map(String::as_str),
		Some(RESTORE_HTTP_RETRY_AFTER_SECONDS)
	);
	Ok(())
}

#[tokio::test]
async fn resume_after_restore_completed() -> Result<()> {
	let db = test_db().await?;
	let ups = test_ups("restore-resume");
	let routes = Routes::new();
	suspend(&db, &ups).await?;
	actor_lifecycle::resume_actor(&db, &ups, ACTOR_ID.to_string()).await?;

	let rejected = restore_lifecycle::maybe_reject_suspended_tunnel_message(
		&db,
		&ups,
		&routes,
		&ws_open(4, ACTOR_ID),
	)
	.await?;
	assert!(!rejected);
	assert!(actor_lifecycle::read_suspension(&db, ACTOR_ID).await?.is_none());

	let rejected = restore_lifecycle::maybe_reject_suspended_tunnel_message(
		&db,
		&ups,
		&routes,
		&http_start(5, ACTOR_ID),
	)
	.await?;
	assert!(!rejected);
	Ok(())
}

#[tokio::test]
async fn failed_restore_leaves_suspended() -> Result<()> {
	let db = test_db().await?;
	let ups = test_ups("restore-failed-stays-suspended");
	let routes = Routes::new();
	suspend(&db, &ups).await?;
	let mut sub = gateway_sub(&ups).await?;

	let rejected = restore_lifecycle::maybe_reject_suspended_tunnel_message(
		&db,
		&ups,
		&routes,
		&ws_open(6, ACTOR_ID),
	)
	.await?;
	let msg = recv_gateway(&mut sub).await?;

	assert!(rejected);
	assert!(actor_lifecycle::read_suspension(&db, ACTOR_ID).await?.is_some());
	let protocol::ToRivetTunnelMessageKind::ToRivetWebSocketClose(close) = msg.message_kind else {
		anyhow::bail!("expected ws close");
	};
	assert_eq!(close.code, Some(RESTORE_WS_CLOSE_CODE));
	Ok(())
}

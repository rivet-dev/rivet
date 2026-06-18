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

use std::{
	sync::Arc,
	time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use depot::conveyer::Db;
use depot_client::types::{BindParam, ColumnValue};
use gas::prelude::Id;
use rivet_pools::NodeId;
use universaldb::driver::RocksDbDatabaseDriver;

use super::{
	clear_remote_sqlite_executors, remote_sqlite_executor_cell, remote_sqlite_executor_from_parts,
	remove_remote_sqlite_executor_generation, remove_remote_sqlite_executors_for_actor,
	validate_remote_sqlite_params,
};
use crate::conn::RemoteSqliteExecutors;

#[tokio::test]
async fn remote_sqlite_executor_cache_is_lazy_and_actor_generation_scoped() {
	let executors = RemoteSqliteExecutors::new();

	assert_eq!(executors.len(), 0);

	let first = remote_sqlite_executor_cell(&executors, "actor-a", 7).await;
	assert!(first.get().is_none());
	assert_eq!(executors.len(), 1);

	let second = remote_sqlite_executor_cell(&executors, "actor-a", 7).await;
	assert!(Arc::ptr_eq(&first, &second));
	assert_eq!(executors.len(), 1);

	let next_generation = remote_sqlite_executor_cell(&executors, "actor-a", 8).await;
	assert!(!Arc::ptr_eq(&first, &next_generation));
	assert_eq!(executors.len(), 2);
}

#[tokio::test]
async fn remote_sqlite_executor_cleanup_removes_actor_scoped_entries() {
	let executors = RemoteSqliteExecutors::new();
	let _ = remote_sqlite_executor_cell(&executors, "actor-a", 7).await;
	let _ = remote_sqlite_executor_cell(&executors, "actor-a", 8).await;
	let _ = remote_sqlite_executor_cell(&executors, "actor-b", 7).await;

	remove_remote_sqlite_executor_generation(&executors, "actor-a", 7).await;
	assert!(!has_remote_sqlite_executor(&executors, "actor-a", 7).await);
	assert!(has_remote_sqlite_executor(&executors, "actor-a", 8).await);
	assert!(has_remote_sqlite_executor(&executors, "actor-b", 7).await);

	remove_remote_sqlite_executors_for_actor(&executors, "actor-a");
	assert!(!has_remote_sqlite_executor(&executors, "actor-a", 8).await);
	assert!(has_remote_sqlite_executor(&executors, "actor-b", 7).await);

	clear_remote_sqlite_executors(&executors);
	assert_eq!(executors.len(), 0);
}

#[tokio::test]
async fn remote_sqlite_executor_reopens_fresh_cell_with_persisted_contents() -> Result<()> {
	let actor_id = unique_actor_id("remote-sqlite-lazy");
	let actor_db = test_actor_db(&actor_id).await?;
	let executors = RemoteSqliteExecutors::new();

	assert_eq!(executors.len(), 0);

	let handle = remote_sqlite_executor_from_parts(
		&executors,
		Arc::clone(&actor_db),
		&actor_id,
		1,
	)
	.await?;
	assert_eq!(executors.len(), 1);
	handle
		.execute(
			"CREATE TABLE items(id INTEGER PRIMARY KEY, label TEXT);".to_string(),
			None,
		)
		.await?;
	handle
		.execute(
			"INSERT INTO items(label) VALUES (?);".to_string(),
			Some(vec![BindParam::Text("alpha".to_string())]),
		)
		.await?;
	handle.close().await?;
	remove_remote_sqlite_executor_generation(&executors, &actor_id, 1).await;

	let fresh_handle = remote_sqlite_executor_from_parts(
		&executors,
		Arc::clone(&actor_db),
		&actor_id,
		2,
	)
	.await?;
	let result = fresh_handle
		.execute(
			"SELECT label FROM items WHERE id = ?;".to_string(),
			Some(vec![BindParam::Integer(1)]),
		)
		.await?;
	assert_eq!(
		result.rows,
		vec![vec![ColumnValue::Text("alpha".to_string())]]
	);

	fresh_handle.close().await?;
	remove_remote_sqlite_executor_generation(&executors, &actor_id, 2).await;
	Ok(())
}

#[test]
fn validate_remote_sqlite_params_bounds_total_bind_bytes() {
	let valid = vec![
		rivet_envoy_protocol::SqliteBindParam::SqliteValueText(
			rivet_envoy_protocol::SqliteValueText {
				value: "alpha".to_string(),
			},
		),
		rivet_envoy_protocol::SqliteBindParam::SqliteValueBlob(
			rivet_envoy_protocol::SqliteValueBlob {
				value: vec![0, 1, 2],
			},
		),
	];
	validate_remote_sqlite_params(Some(&valid)).expect("small bind params should pass");

	let too_large = vec![rivet_envoy_protocol::SqliteBindParam::SqliteValueBlob(
		rivet_envoy_protocol::SqliteValueBlob {
			value: vec![0; super::MAX_REMOTE_SQL_BIND_BYTES + 1],
		},
	)];
	let err = validate_remote_sqlite_params(Some(&too_large))
		.expect_err("oversized bind params should fail");
	assert!(err.to_string().contains("bind params had"));
}

async fn has_remote_sqlite_executor(
	executors: &RemoteSqliteExecutors,
	actor_id: &str,
	generation: u64,
) -> bool {
	let key = (actor_id.to_string(), generation);
	executors.read_async(&key, |_, _| ()).await.is_some()
}

async fn test_actor_db(actor_id: &str) -> Result<Arc<Db>> {
	let db_dir = tempfile::tempdir()?.keep();
	let driver = RocksDbDatabaseDriver::new(db_dir).await?;
	let db = Arc::new(universaldb::Database::new(Arc::new(driver)));
	Ok(Arc::new(Db::new(
		db,
		Id::new_v1(1),
		actor_id.to_string(),
		NodeId::new(),
	)))
}

fn unique_actor_id(prefix: &str) -> String {
	let nanos = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.expect("system time should be after epoch")
		.as_nanos();
	format!("{prefix}-{nanos}")
}

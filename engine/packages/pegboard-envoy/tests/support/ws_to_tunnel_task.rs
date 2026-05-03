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
	sync::{
		Arc,
		atomic::{AtomicBool, Ordering},
	},
	time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use depot_client::types::{BindParam, ColumnValue};
use sqlite_storage::{engine::SqliteEngine, error::SqliteStorageError, open::OpenConfig};
use tokio::{
	sync::{Notify, Semaphore},
	time::{Duration, Instant, timeout},
};
use universaldb::{Subspace, driver::RocksDbDatabaseDriver};

use super::{
	actor_lifecycle::{
		ActiveActor, ActiveActorState, clear_remote_sqlite_executors,
		remove_remote_sqlite_executor_generation, remove_remote_sqlite_executors_for_actor,
	},
	cached_active_sqlite_actor, cached_serverless_sqlite_generation, remote_sqlite_executor_cell,
	remote_sqlite_executor_from_parts, spawn_tracked_remote_sqlite_task,
	validate_remote_sqlite_params, validate_sqlite_get_page_range_request,
};
use crate::conn::{
	RemoteSqliteExecutors, RemoteSqliteInflight, remote_sqlite_inflight_count,
	remove_remote_sqlite_inflight_generation_if_idle, wait_remote_sqlite_inflight_generation,
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
async fn tracked_remote_sqlite_tasks_are_bounded_and_visible_to_stop_waiters() {
	let worker_permits = Arc::new(Semaphore::new(1));
	let in_flight = RemoteSqliteInflight::new();
	let first_started = Arc::new(Notify::new());
	let first_release = Arc::new(Notify::new());
	let second_started = Arc::new(Notify::new());
	let second_release = Arc::new(Notify::new());
	let second_ran = Arc::new(AtomicBool::new(false));

	spawn_tracked_remote_sqlite_task(
		worker_permits.clone(),
		&in_flight,
		"actor-a".to_string(),
		7,
		"test sqlite execute",
		{
			let first_started = first_started.clone();
			let first_release = first_release.clone();
			async move {
				first_started.notify_waiters();
				first_release.notified().await;
			}
		},
	)
	.await;
	first_started.notified().await;

	spawn_tracked_remote_sqlite_task(
		worker_permits,
		&in_flight,
		"actor-a".to_string(),
		7,
		"test sqlite execute",
		{
			let second_started = second_started.clone();
			let second_release = second_release.clone();
			let second_ran = second_ran.clone();
			async move {
				second_ran.store(true, Ordering::SeqCst);
				second_started.notify_waiters();
				second_release.notified().await;
			}
		},
	)
	.await;

	assert_eq!(
		remote_sqlite_inflight_count(&in_flight, "actor-a", 7).await,
		2
	);
	assert!(!second_ran.load(Ordering::SeqCst));
	assert!(
		!wait_remote_sqlite_inflight_generation(
			&in_flight,
			"actor-a",
			7,
			Instant::now() + Duration::from_millis(1),
		)
		.await
	);

	first_release.notify_waiters();
	timeout(Duration::from_secs(1), second_started.notified())
		.await
		.expect("second task should start after the first releases its worker permit");
	second_release.notify_waiters();
	assert!(
		wait_remote_sqlite_inflight_generation(
			&in_flight,
			"actor-a",
			7,
			Instant::now() + Duration::from_secs(1),
		)
		.await
	);

	remove_remote_sqlite_inflight_generation_if_idle(&in_flight, "actor-a", 7).await;
	assert_eq!(
		remote_sqlite_inflight_count(&in_flight, "actor-a", 7).await,
		0
	);
}

#[tokio::test]
async fn remote_sqlite_stop_wait_does_not_finish_before_running_task() {
	let worker_permits = Arc::new(Semaphore::new(1));
	let in_flight = RemoteSqliteInflight::new();
	let task_started = Arc::new(Notify::new());
	let task_release = Arc::new(Notify::new());

	spawn_tracked_remote_sqlite_task(
		worker_permits,
		&in_flight,
		"actor-a".to_string(),
		7,
		"test sqlite execute",
		{
			let task_started = task_started.clone();
			let task_release = task_release.clone();
			async move {
				task_started.notify_waiters();
				task_release.notified().await;
			}
		},
	)
	.await;
	task_started.notified().await;

	assert!(
		timeout(
			Duration::from_millis(20),
			wait_remote_sqlite_inflight_generation(
				&in_flight,
				"actor-a",
				7,
				Instant::now() + Duration::from_secs(1),
			),
		)
		.await
		.is_err()
	);
	task_release.notify_waiters();
	assert!(
		wait_remote_sqlite_inflight_generation(
			&in_flight,
			"actor-a",
			7,
			Instant::now() + Duration::from_secs(1),
		)
		.await
	);
}

#[tokio::test]
async fn remote_sqlite_executor_reopens_fresh_cell_with_persisted_contents() -> Result<()> {
	let actor_id = unique_actor_id("remote-sqlite-lazy");
	let db_dir = tempfile::tempdir()?;
	let driver = RocksDbDatabaseDriver::new(db_dir.path().to_path_buf()).await?;
	let db = universaldb::Database::new(Arc::new(driver));
	let (engine, _compaction_rx) =
		SqliteEngine::new(db, Subspace::new(&("remote-sqlite-lazy", &actor_id)));
	let engine = Arc::new(engine);
	let executors = RemoteSqliteExecutors::new();
	let opened = engine.open(&actor_id, OpenConfig::new(1)).await?;

	assert_eq!(executors.len(), 0);

	let handle = remote_sqlite_executor_from_parts(
		&executors,
		Arc::clone(&engine),
		&actor_id,
		opened.generation,
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
	remove_remote_sqlite_executor_generation(&executors, &actor_id, opened.generation).await;
	engine.close(&actor_id, opened.generation).await?;

	let reopened = engine.open(&actor_id, OpenConfig::new(2)).await?;
	let fresh_handle = remote_sqlite_executor_from_parts(
		&executors,
		Arc::clone(&engine),
		&actor_id,
		reopened.generation,
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
	remove_remote_sqlite_executor_generation(&executors, &actor_id, reopened.generation).await;
	engine.close(&actor_id, reopened.generation).await?;
	Ok(())
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

fn unique_actor_id(prefix: &str) -> String {
	let nanos = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.expect("system time should be after epoch")
		.as_nanos();
	format!("{prefix}-{nanos}")
}

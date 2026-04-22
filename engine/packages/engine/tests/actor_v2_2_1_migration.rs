use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result, ensure};
use gas::prelude::*;
use pegboard::actor_kv::Recipient;
use rivet_envoy_protocol as protocol;
use rusqlite::Connection;
use serde::Deserialize;
use sqlite_storage::{
	engine::SqliteEngine,
	types::{SQLITE_VFS_V2_SCHEMA_VERSION, SqliteOrigin},
};
use test_snapshot::SnapshotTestCtx;

const SNAPSHOT_NAME: &str = "actor-v2-2-1-baseline";
const ACTOR_NAME: &str = "actor-v2-2-1-baseline";
const USER_KV_VALUE: &[u8] = b"snapshot-value";
const QUEUE_MESSAGE_BODY: &[u8] = b"queued-from-v2.2.1";

#[tokio::test(flavor = "multi_thread")]
async fn actor_v2_2_1_baseline_migrates_to_current_layout() -> Result<()> {
	let mut test_ctx = SnapshotTestCtx::from_snapshot_with_coordinator(SNAPSHOT_NAME).await?;
	let ctx = test_ctx.get_ctx(test_ctx.leader_id);

	let namespace = ctx
		.op(namespace::ops::resolve_for_name_local::Input {
			name: "default".to_string(),
		})
		.await?
		.context("default namespace should exist")?;
	let actor = ctx
		.op(pegboard::ops::actor::list_for_ns::Input {
			namespace_id: namespace.namespace_id,
			name: ACTOR_NAME.to_string(),
			key: None,
			include_destroyed: true,
			created_before: None,
			limit: 1,
			fetch_error: false,
		})
		.await?
		.actors
		.into_iter()
		.next()
		.context("snapshot actor should exist")?;

	let db = Arc::new((*ctx.udb()?).clone());
	let standalone_ctx = ctx.standalone()?;
	let (sqlite_engine, _compaction_rx) = SqliteEngine::new(
		Arc::clone(&db),
		pegboard::actor_sqlite_v2::sqlite_subspace(),
	);
	let mut start = protocol::CommandStartActor {
		config: protocol::ActorConfig {
			name: actor.name.clone(),
			key: actor.key.clone(),
			create_ts: actor.create_ts,
			input: None,
		},
		hibernating_requests: Vec::new(),
		preloaded_kv: None,
		sqlite_schema_version: SQLITE_VFS_V2_SCHEMA_VERSION,
		sqlite_startup_data: None,
	};

	pegboard_envoy::sqlite_runtime::populate_start_command(
		&standalone_ctx,
		&sqlite_engine,
		protocol::PROTOCOL_VERSION,
		namespace.namespace_id,
		actor.actor_id,
		&mut start,
	)
	.await?;

	assert_eq!(start.sqlite_schema_version, SQLITE_VFS_V2_SCHEMA_VERSION);
	assert!(start.sqlite_startup_data.is_some());
	assert_eq!(
		sqlite_engine
			.load_head(&actor.actor_id.to_string())
			.await?
			.origin,
		SqliteOrigin::MigratedFromV1
	);
	assert_eq!(
		query_sqlite_notes(&load_v2_sqlite_bytes(&sqlite_engine, actor.actor_id).await?)?,
		vec!["sqlite-from-v2.2.1"]
	);

	let recipient = Recipient {
		actor_id: actor.actor_id,
		namespace_id: namespace.namespace_id,
		name: actor.name.clone(),
	};
	let (keys, values, _) = pegboard::actor_kv::get(
		&db,
		&recipient,
		vec![vec![1], make_user_kv_key(b"snapshot-key"), vec![5, 1, 1]],
	)
	.await?;
	let by_key: HashMap<Vec<u8>, Vec<u8>> = keys.into_iter().zip(values).collect();
	assert_eq!(
		by_key
			.get(&make_user_kv_key(b"snapshot-key"))
			.map(Vec::as_slice),
		Some(USER_KV_VALUE)
	);

	let persisted = decode_persisted_actor(
		by_key
			.get(&vec![1])
			.context("persisted actor state should exist")?,
	)?;
	assert!(persisted.input.is_none());
	assert!(persisted.has_initialized);
	assert!(!persisted.state.is_empty());
	assert_eq!(persisted.scheduled_events.len(), 1);
	assert_eq!(persisted.scheduled_events[0].event_id, "baseline-alarm");
	assert_eq!(persisted.scheduled_events[0].action, "scheduled");
	assert!(persisted.scheduled_events[0].timestamp_ms > 0);
	assert!(!persisted.scheduled_events[0].args.is_empty());

	let queue_messages = pegboard::actor_kv::list(
		&db,
		&recipient,
		protocol::KvListQuery::KvListPrefixQuery(protocol::KvListPrefixQuery {
			key: vec![5, 1, 2],
		}),
		false,
		Some(10),
	)
	.await?;
	ensure!(queue_messages.0.len() == 1, "expected one queue message");
	let queue_message = decode_queue_message(&queue_messages.1[0])?;
	assert_eq!(queue_message.name, "baseline-message");
	assert_eq!(queue_message.body, QUEUE_MESSAGE_BODY);
	assert!(queue_message.created_at > 0);
	assert_eq!(queue_message.failure_count, None);
	assert_eq!(queue_message.available_at, None);
	assert_eq!(queue_message.in_flight, None);
	assert_eq!(queue_message.in_flight_at, None);
	pegboard::actor_kv::delete(&db, &recipient, vec![queue_messages.0[0].clone()]).await?;
	let drained_queue_messages = pegboard::actor_kv::list(
		&db,
		&recipient,
		protocol::KvListQuery::KvListPrefixQuery(protocol::KvListPrefixQuery {
			key: vec![5, 1, 2],
		}),
		false,
		Some(10),
	)
	.await?;
	ensure!(
		drained_queue_messages.0.is_empty(),
		"queue message should drain"
	);

	test_ctx.shutdown().await?;
	Ok(())
}

#[derive(Deserialize)]
struct PersistedScheduleEvent {
	event_id: String,
	timestamp_ms: i64,
	action: String,
	args: Vec<u8>,
}

#[derive(Deserialize)]
struct PersistedActor {
	input: Option<Vec<u8>>,
	has_initialized: bool,
	state: Vec<u8>,
	scheduled_events: Vec<PersistedScheduleEvent>,
}

#[derive(Deserialize)]
struct PersistedQueueMessage {
	name: String,
	body: Vec<u8>,
	created_at: i64,
	failure_count: Option<u32>,
	available_at: Option<i64>,
	in_flight: Option<bool>,
	in_flight_at: Option<i64>,
}

fn decode_persisted_actor(bytes: &[u8]) -> Result<PersistedActor> {
	decode_embedded_version(bytes, 4, "persisted actor")
}

fn decode_queue_message(bytes: &[u8]) -> Result<PersistedQueueMessage> {
	decode_embedded_version(bytes, 4, "queue message")
}

fn decode_embedded_version<T>(bytes: &[u8], expected: u16, label: &str) -> Result<T>
where
	T: for<'de> Deserialize<'de>,
{
	ensure!(bytes.len() >= 2, "{label} payload too short");
	let version = u16::from_le_bytes([bytes[0], bytes[1]]);
	ensure!(
		version == expected,
		"{label} version was {version}, expected {expected}"
	);
	Ok(serde_bare::from_slice(&bytes[2..])?)
}

async fn load_v2_sqlite_bytes(engine: &SqliteEngine, actor_id: Id) -> Result<Vec<u8>> {
	let actor_id = actor_id.to_string();
	let meta = engine.load_meta(&actor_id).await?;
	let pages = engine
		.get_pages(
			&actor_id,
			meta.generation,
			(1..=meta.db_size_pages).collect(),
		)
		.await?;
	let mut bytes = Vec::with_capacity(meta.db_size_pages as usize * meta.page_size as usize);
	for page in pages {
		bytes.extend_from_slice(
			&page
				.bytes
				.unwrap_or_else(|| vec![0; meta.page_size as usize]),
		);
	}
	Ok(bytes)
}

fn query_sqlite_notes(bytes: &[u8]) -> Result<Vec<String>> {
	let tmp = tempfile::tempdir()?;
	let path = tmp.path().join("query.db");
	std::fs::write(&path, bytes)?;
	let conn = Connection::open(path)?;
	let mut stmt = conn.prepare("SELECT note FROM items ORDER BY id")?;
	Ok(stmt
		.query_map([], |row| row.get::<_, String>(0))?
		.collect::<std::result::Result<Vec<_>, _>>()?)
}

fn make_user_kv_key(key: &[u8]) -> Vec<u8> {
	let mut out = Vec::with_capacity(1 + key.len());
	out.push(4);
	out.extend_from_slice(key);
	out
}

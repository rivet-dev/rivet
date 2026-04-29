use anyhow::{Context, Result};
use async_trait::async_trait;
use gas::prelude::*;
use rivet_types::actors::CrashPolicy;
use rivet_types::namespaces::Namespace;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use tempfile::tempdir;

use crate::test_cluster::TestCluster;

use super::Scenario;

const ACTOR_NAME: &str = "actor-v2-2-1-baseline";
const RUNNER_NAME: &str = "default";
const USER_KV_KEY: &[u8] = b"snapshot-key";
const USER_KV_VALUE: &[u8] = b"snapshot-value";
const QUEUE_MESSAGE_ID: u64 = 1;
const QUEUE_MESSAGE_NAME: &str = "baseline-message";
const QUEUE_MESSAGE_BODY: &[u8] = b"queued-from-v2.2.1";
const SQLITE_V1_PREFIX: u8 = 0x08;
const SQLITE_V1_SCHEMA_VERSION: u8 = 0x01;
const SQLITE_V1_META_PREFIX: u8 = 0x00;
const SQLITE_V1_CHUNK_PREFIX: u8 = 0x01;
const SQLITE_V1_META_VERSION: u16 = 1;
const SQLITE_V1_CHUNK_SIZE: usize = 4096;
const FILE_TAG_MAIN: u8 = 0x00;
const ACTOR_PERSIST_VERSION: u16 = 4;
const QUEUE_PAYLOAD_VERSION: u16 = 4;

/// Scenario that seeds an actor using the v2.2.1 actor KV layouts.
pub struct ActorV221Baseline;

#[async_trait(?Send)]
impl Scenario for ActorV221Baseline {
	fn name(&self) -> &'static str {
		"actor-v2-2-1-baseline"
	}

	fn replica_count(&self) -> usize {
		2
	}

	async fn populate(&self, cluster: &TestCluster) -> Result<()> {
		let ctx = cluster.get_ctx(cluster.leader_id());

		let namespace = get_or_create_default_namespace(ctx).await?;

		let actor_id = Id::new_v1(ctx.config().dc_label());
		ctx.op(pegboard::ops::actor::create::Input {
			actor_id,
			namespace_id: namespace.namespace_id,
			name: ACTOR_NAME.to_string(),
			key: None,
			runner_name_selector: RUNNER_NAME.to_string(),
			input: None,
			crash_policy: CrashPolicy::Sleep,
			forward_request: false,
			datacenter_name: None,
		})
		.await?;

		let recipient = pegboard::actor_kv::Recipient {
			actor_id,
			namespace_id: namespace.namespace_id,
			name: ACTOR_NAME.to_string(),
		};

		let fixture = build_sqlite_fixture()?;
		let persisted = encode_with_embedded_version(
			&PersistedActor {
				input: None,
				has_initialized: true,
				state: encode_cbor(&serde_json::json!({
					"source": "v2.2.1",
					"counter": 42,
				}))?,
				scheduled_events: vec![PersistedScheduleEvent {
					event_id: "baseline-alarm".to_string(),
					timestamp_ms: util::timestamp::now() + 60_000,
					action: "scheduled".to_string(),
					args: encode_cbor(&serde_json::json!({ "ok": true }))?,
				}],
			},
			ACTOR_PERSIST_VERSION,
		)?;
		let queue_metadata = encode_with_embedded_version(
			&QueueMetadata {
				next_id: QUEUE_MESSAGE_ID + 1,
				size: 1,
			},
			QUEUE_PAYLOAD_VERSION,
		)?;
		let queue_message = encode_with_embedded_version(
			&PersistedQueueMessage {
				name: QUEUE_MESSAGE_NAME.to_string(),
				body: QUEUE_MESSAGE_BODY.to_vec(),
				created_at: util::timestamp::now(),
				failure_count: None,
				available_at: None,
				in_flight: None,
				in_flight_at: None,
			},
			QUEUE_PAYLOAD_VERSION,
		)?;

		let mut keys = vec![
			vec![1],
			make_user_kv_key(USER_KV_KEY),
			vec![5, 1, 1],
			make_queue_message_key(QUEUE_MESSAGE_ID),
		];
		let mut values = vec![
			persisted,
			USER_KV_VALUE.to_vec(),
			queue_metadata,
			queue_message,
		];
		append_sqlite_v1_file(&mut keys, &mut values, FILE_TAG_MAIN, &fixture);

		pegboard::actor_kv::put(&*ctx.udb()?, &recipient, keys, values).await?;

		tracing::info!(%actor_id, "seeded v2.2.1 baseline actor snapshot");

		Ok(())
	}
}

async fn get_or_create_default_namespace(ctx: &gas::prelude::TestCtx) -> Result<Namespace> {
	if let Some(namespace) = ctx
		.op(namespace::ops::resolve_for_name_local::Input {
			name: "default".to_string(),
		})
		.await?
	{
		return Ok(namespace);
	}

	let namespace_id = Id::new_v1(ctx.config().dc_label());
	let mut create_sub = ctx
		.subscribe::<namespace::workflows::namespace::CreateComplete>((
			"namespace_id",
			namespace_id,
		))
		.await?;
	let mut fail_sub = ctx
		.subscribe::<namespace::workflows::namespace::Failed>(("namespace_id", namespace_id))
		.await?;

	ctx.workflow(namespace::workflows::namespace::Input {
		namespace_id,
		name: "default".to_string(),
		display_name: "Default".to_string(),
	})
	.tag("namespace_id", namespace_id)
	.dispatch()
	.await?;

	tokio::select! {
		res = create_sub.next() => { res?; },
		res = fail_sub.next() => {
			let msg = res?;
			return Err(msg.into_body().error.build().into());
		}
	}

	ctx.op(namespace::ops::get_local::Input {
		namespace_ids: vec![namespace_id],
	})
	.await?
	.into_iter()
	.next()
	.context("created default namespace should exist")
}

#[derive(Serialize, Deserialize)]
struct PersistedScheduleEvent {
	event_id: String,
	timestamp_ms: i64,
	action: String,
	args: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
struct PersistedActor {
	input: Option<Vec<u8>>,
	has_initialized: bool,
	state: Vec<u8>,
	scheduled_events: Vec<PersistedScheduleEvent>,
}

#[derive(Serialize, Deserialize)]
struct QueueMetadata {
	next_id: u64,
	size: u32,
}

#[derive(Serialize, Deserialize)]
struct PersistedQueueMessage {
	name: String,
	body: Vec<u8>,
	created_at: i64,
	failure_count: Option<u32>,
	available_at: Option<i64>,
	in_flight: Option<bool>,
	in_flight_at: Option<i64>,
}

fn build_sqlite_fixture() -> Result<Vec<u8>> {
	let tmp = tempdir()?;
	let path = tmp.path().join("baseline.db");
	let conn = Connection::open(&path)?;
	conn.pragma_update(None, "page_size", 4096)?;
	conn.pragma_update(None, "journal_mode", "DELETE")?;
	conn.pragma_update(None, "synchronous", "NORMAL")?;
	conn.pragma_update(None, "temp_store", "MEMORY")?;
	conn.pragma_update(None, "auto_vacuum", "NONE")?;
	conn.pragma_update(None, "locking_mode", "EXCLUSIVE")?;
	conn.execute_batch("CREATE TABLE items (id INTEGER PRIMARY KEY, note TEXT NOT NULL);")?;
	conn.execute(
		"INSERT INTO items(note) VALUES (?1)",
		params!["sqlite-from-v2.2.1"],
	)?;
	drop(conn);
	Ok(std::fs::read(path)?)
}

fn append_sqlite_v1_file(
	keys: &mut Vec<Vec<u8>>,
	values: &mut Vec<Vec<u8>>,
	file_tag: u8,
	bytes: &[u8],
) {
	keys.push(v1_meta_key(file_tag).to_vec());
	values.push(encode_v1_meta(bytes.len() as u64).to_vec());

	for (chunk_idx, chunk) in bytes.chunks(SQLITE_V1_CHUNK_SIZE).enumerate() {
		keys.push(v1_chunk_key(file_tag, chunk_idx as u32).to_vec());
		values.push(chunk.to_vec());
	}
}

fn encode_cbor(value: &serde_json::Value) -> Result<Vec<u8>> {
	let mut bytes = Vec::new();
	ciborium::into_writer(value, &mut bytes)?;
	Ok(bytes)
}

fn encode_with_embedded_version<T>(value: &T, version: u16) -> Result<Vec<u8>>
where
	T: Serialize,
{
	let payload = serde_bare::to_vec(value)?;
	let mut encoded = Vec::with_capacity(2 + payload.len());
	encoded.extend_from_slice(&version.to_le_bytes());
	encoded.extend_from_slice(&payload);
	Ok(encoded)
}

fn encode_v1_meta(size: u64) -> [u8; 10] {
	let mut bytes = [0_u8; 10];
	bytes[..2].copy_from_slice(&SQLITE_V1_META_VERSION.to_le_bytes());
	bytes[2..].copy_from_slice(&size.to_le_bytes());
	bytes
}

fn v1_meta_key(file_tag: u8) -> [u8; 4] {
	[
		SQLITE_V1_PREFIX,
		SQLITE_V1_SCHEMA_VERSION,
		SQLITE_V1_META_PREFIX,
		file_tag,
	]
}

fn v1_chunk_key(file_tag: u8, chunk_idx: u32) -> [u8; 8] {
	let chunk_idx = chunk_idx.to_be_bytes();
	[
		SQLITE_V1_PREFIX,
		SQLITE_V1_SCHEMA_VERSION,
		SQLITE_V1_CHUNK_PREFIX,
		file_tag,
		chunk_idx[0],
		chunk_idx[1],
		chunk_idx[2],
		chunk_idx[3],
	]
}

fn make_user_kv_key(key: &[u8]) -> Vec<u8> {
	let mut out = Vec::with_capacity(1 + key.len());
	out.push(4);
	out.extend_from_slice(key);
	out
}

fn make_queue_message_key(id: u64) -> Vec<u8> {
	let mut out = Vec::with_capacity(11);
	out.extend_from_slice(&[5, 1, 2]);
	out.extend_from_slice(&id.to_be_bytes());
	out
}

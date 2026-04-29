use std::sync::Arc;

use anyhow::Result;
use rivet_envoy_protocol as protocol;
use rivet_pools::NodeId;
use scc::HashMap;
use sqlite_storage::{
	keys::{
		delta_chunk_key, meta_compact_key, meta_compactor_lease_key, meta_head_key, meta_quota_key,
		pidx_delta_key, shard_key,
	},
	pump::ActorDb,
};
use tempfile::Builder;
use universaldb::utils::IsolationLevel::Snapshot;
use universalpubsub::{PubSub, driver::memory::MemoryDriver};

mod conn {
	use std::sync::Arc;

	use scc::HashMap;
	use sqlite_storage::pump::ActorDb;

	pub struct Conn {
		pub actor_dbs: HashMap<String, Arc<ActorDb>>,
	}
}

#[allow(dead_code)]
#[path = "../src/actor_lifecycle.rs"]
mod actor_lifecycle;

const TEST_ACTOR: &str = "actor-lifecycle-test";

async fn test_db() -> Result<universaldb::Database> {
	let path = Builder::new()
		.prefix("pegboard-envoy-actor-lifecycle-")
		.tempdir()?
		.keep();
	let driver = universaldb::driver::RocksDbDatabaseDriver::new(path).await?;

	Ok(universaldb::Database::new(Arc::new(driver)))
}

fn test_ups() -> PubSub {
	PubSub::new(Arc::new(MemoryDriver::new(
		"pegboard-envoy-actor-lifecycle-test".to_string(),
	)))
}

fn checkpoint(actor_id: &str) -> protocol::ActorCheckpoint {
	protocol::ActorCheckpoint {
		actor_id: actor_id.to_string(),
		generation: 1,
		index: 2,
	}
}

async fn seed(db: &universaldb::Database, keys: &[Vec<u8>]) -> Result<()> {
	let writes = keys
		.iter()
		.cloned()
		.map(|key| (key, b"present".to_vec()))
		.collect::<Vec<_>>();
	db.run(move |tx| {
		let writes = writes.clone();
		async move {
			for (key, value) in writes {
				tx.informal().set(&key, &value);
			}
			Ok(())
		}
	})
	.await
}

async fn value_exists(db: &universaldb::Database, key: Vec<u8>) -> Result<bool> {
	db.run(move |tx| {
		let key = key.clone();
		async move { Ok(tx.informal().get(&key, Snapshot).await?.is_some()) }
	})
	.await
}

fn sqlite_keys(actor_id: &str) -> Vec<Vec<u8>> {
	vec![
		meta_head_key(actor_id),
		meta_compact_key(actor_id),
		meta_quota_key(actor_id),
		meta_compactor_lease_key(actor_id),
		pidx_delta_key(actor_id, 1),
		delta_chunk_key(actor_id, 1, 0),
		shard_key(actor_id, 0),
	]
}

#[tokio::test]
async fn stop_actor_evicts_cached_actor_db() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let actor_db = Arc::new(ActorDb::new(
		db,
		test_ups(),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	));
	let conn = conn::Conn {
		actor_dbs: HashMap::new(),
	};

	assert!(conn
		.actor_dbs
		.insert_async(TEST_ACTOR.to_string(), actor_db)
		.await
		.is_ok());

	actor_lifecycle::stop_actor(&conn, &checkpoint(TEST_ACTOR)).await?;

	assert!(!conn.actor_dbs.contains_async(TEST_ACTOR).await);
	Ok(())
}

#[tokio::test]
async fn stop_actor_does_not_touch_udb() -> Result<()> {
	let db = Arc::new(test_db().await?);
	let actor_db = Arc::new(ActorDb::new(
		Arc::clone(&db),
		test_ups(),
		TEST_ACTOR.to_string(),
		NodeId::new(),
	));
	let conn = conn::Conn {
		actor_dbs: HashMap::new(),
	};
	assert!(conn
		.actor_dbs
		.insert_async(TEST_ACTOR.to_string(), actor_db)
		.await
		.is_ok());

	let keys = sqlite_keys(TEST_ACTOR);
	seed(&db, &keys).await?;

	actor_lifecycle::stop_actor(&conn, &checkpoint(TEST_ACTOR)).await?;

	for key in keys {
		assert!(value_exists(&db, key).await?);
	}

	Ok(())
}

//! The pegboard index lists actors by name and envoys, both ordered by creation time.
//!
//! Actor entries of one namespace and actor name are spread over
//! `pegboard.actor_index_buckets_per_name` hashed bucket volumes so inserts do not share one
//! volume; a listing merges that name's buckets into one total order. Envoy entries live in one
//! volume per namespace and pool, ordered by creation time.
//!
//! The active envoy index keeps, per envoy and per actor name it serves, one entry keyed by the
//! name and the envoy's latest heartbeat, also in one volume per namespace and pool. The entry's
//! value is the envoy's route, so allocating an actor is one listing of its name with no record
//! read. A listing scans only heartbeats inside [`crate::ENVOY_HEARTBEAT_TIMEOUT_MS`], so envoys
//! that stopped heartbeating drop out without a sweep. A per-envoy state key in the same volume
//! fences the entries by connection epoch and remembers where they sit, so an older connection
//! of the envoy can neither overwrite nor delete a newer connection's route.
//!
//! The pool catalog keeps one entry per pool of a namespace, in one volume per namespace, so the
//! configured pools can be enumerated even before any envoy connects.
//!
//! Creation-ordered index entries are keys with empty values. Inserting the same entry twice
//! writes the same key, so inserts are idempotent and need no command receipt.
use crate::commands::EnvoyRead;
use crate::keys::{
	kv::ActiveEnvoyIndexKey, kv::ActiveEnvoyIndexNamePrefix, kv::ActiveEnvoyStateKey,
	kv::ActiveEnvoyStatePrefix, kv::ActorIndexKey, kv::ActorIndexPrefix,
	kv::DestroyedActorIndexKey, kv::DestroyedActorIndexPrefix, kv::EnvoyIndexKey,
	kv::EnvoyIndexPrefix, kv::EnvoyPoolKey, kv::EnvoyPoolPrefix, partition::ActiveEnvoyIndexVolume,
	partition::ActorIndexVolume, partition::EnvoyIndexVolume, partition::EnvoyPoolIndexVolume,
	partition::EnvoyVolume, partition::actor_index_bucket,
};
use crate::wire;
use anyhow::{Context, Result, ensure};
use depot_protocol::command::{Command, CommandTag};
use depot_worker::command::{CommandRegistry, KvHandle};
use futures_util::StreamExt;
use rivet_engine_context::EngineContext;
use rivet_env::Clock;
use rivet_keys::keys::{
	KeyAddress,
	codec::{FormalKey, SubspaceRange},
	tags::depot_command::{
		PEGBOARD_INDEX_DELETE_ACTIVE_ENVOY, PEGBOARD_INDEX_DELETE_ACTOR,
		PEGBOARD_INDEX_INSERT_ACTOR, PEGBOARD_INDEX_INSERT_ENVOY,
		PEGBOARD_INDEX_LIST_ACTIVE_ENVOY_STATES, PEGBOARD_INDEX_LIST_ACTIVE_ENVOYS,
		PEGBOARD_INDEX_LIST_ACTORS, PEGBOARD_INDEX_LIST_ENVOY_POOLS, PEGBOARD_INDEX_LIST_ENVOYS,
		PEGBOARD_INDEX_READ_DESTROYED_ACTOR, PEGBOARD_INDEX_UPSERT_ACTIVE_ENVOY,
	},
};
use rivet_util::Id;
use std::cmp::Ordering;
use std::collections::{HashMap, VecDeque};
use std::ops::{Bound, Range};
use std::sync::Arc;

/// Maximum number of entries returned by one list page.
const PAGE_SIZE: usize = 100;

/// Entries the merge first requests from each bucket. Uniform hashing puts about `limit / N`
/// entries of a page in each bucket, so a small first page rarely needs a refill.
const MERGE_FIRST_PAGE: usize = 8;

pub struct IndexInsertActor;

impl Command for IndexInsertActor {
	const TAG: CommandTag = CommandTag(PEGBOARD_INDEX_INSERT_ACTOR);
	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::IndexInsertActorRequest;
	type Response = rivet_data::versioned::IndexInsertActorResponse;
}

pub struct IndexDeleteActor;

impl Command for IndexDeleteActor {
	const TAG: CommandTag = CommandTag(PEGBOARD_INDEX_DELETE_ACTOR);
	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::IndexDeleteActorRequest;
	type Response = rivet_data::versioned::IndexDeleteActorResponse;
}

/// Lists one actor bucket. A listing must not create every bucket volume, so a missing bucket
/// is reported as `VolumeNotFound` and [`list_actors`] treats it as empty.
pub struct IndexListActors;

impl Command for IndexListActors {
	const TAG: CommandTag = CommandTag(PEGBOARD_INDEX_LIST_ACTORS);
	const CREATE_IF_MISSING: bool = false;
	type Request = rivet_data::versioned::IndexListActorsRequest;
	type Response = rivet_data::versioned::IndexListActorsResponse;
}

/// Reads when one destroyed incarnation of an actor left the index. A bucket volume that was
/// never created holds no destroyed entries.
pub struct IndexReadDestroyedActor;

impl Command for IndexReadDestroyedActor {
	const TAG: CommandTag = CommandTag(PEGBOARD_INDEX_READ_DESTROYED_ACTOR);
	const READ_ONLY: bool = true;
	const CREATE_IF_MISSING: bool = false;
	type Request = rivet_data::versioned::IndexReadDestroyedActorRequest;
	type Response = rivet_data::versioned::IndexReadDestroyedActorResponse;
}

pub struct IndexInsertEnvoy;

impl Command for IndexInsertEnvoy {
	const TAG: CommandTag = CommandTag(PEGBOARD_INDEX_INSERT_ENVOY);
	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::IndexInsertEnvoyRequest;
	type Response = rivet_data::versioned::IndexInsertEnvoyResponse;
}

/// Lists one pool's envoy keys. A pool with no index volume has no entries.
pub struct IndexListEnvoys;

impl Command for IndexListEnvoys {
	const TAG: CommandTag = CommandTag(PEGBOARD_INDEX_LIST_ENVOYS);
	const CREATE_IF_MISSING: bool = false;
	type Request = rivet_data::versioned::IndexListEnvoysRequest;
	type Response = rivet_data::versioned::IndexListEnvoysResponse;
}

/// Moves an envoy's active entries to its new heartbeat and route under the names it serves,
/// unless a newer connection of the envoy already wrote its own. The reply says so.
pub struct IndexUpsertActiveEnvoy;

impl Command for IndexUpsertActiveEnvoy {
	const TAG: CommandTag = CommandTag(PEGBOARD_INDEX_UPSERT_ACTIVE_ENVOY);
	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::IndexUpsertActiveEnvoyRequest;
	type Response = rivet_data::versioned::IndexUpsertActiveEnvoyResponse;
}

/// Removes an envoy's active entries and state, unless a newer connection of the envoy owns
/// them. Deleting an envoy that is already gone is a no-op, so retries are safe.
pub struct IndexDeleteActiveEnvoy;

impl Command for IndexDeleteActiveEnvoy {
	const TAG: CommandTag = CommandTag(PEGBOARD_INDEX_DELETE_ACTIVE_ENVOY);
	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::IndexDeleteActiveEnvoyRequest;
	type Response = rivet_data::versioned::IndexDeleteActiveEnvoyResponse;
}

/// Lists a namespace's pools. A namespace with no catalog volume has no pools.
pub struct IndexListEnvoyPools;

impl Command for IndexListEnvoyPools {
	const TAG: CommandTag = CommandTag(PEGBOARD_INDEX_LIST_ENVOY_POOLS);
	const CREATE_IF_MISSING: bool = false;
	type Request = rivet_data::versioned::IndexListEnvoyPoolsRequest;
	type Response = rivet_data::versioned::IndexListEnvoyPoolsResponse;
}

/// Lists one pool's envoys serving one actor name whose latest heartbeat is inside the
/// eligibility window, with their routes. A pool with no active index volume has no entries.
pub struct IndexListActiveEnvoys;

impl Command for IndexListActiveEnvoys {
	const TAG: CommandTag = CommandTag(PEGBOARD_INDEX_LIST_ACTIVE_ENVOYS);
	const CREATE_IF_MISSING: bool = false;
	type Request = rivet_data::versioned::IndexListActiveEnvoysRequest;
	type Response = rivet_data::versioned::IndexListActiveEnvoysResponse;
}

/// Lists the state of every envoy of one pool whose latest heartbeat is inside the eligibility
/// window, whatever names it serves. A pool with no active index volume has no entries.
pub struct IndexListActiveEnvoyStates;

impl Command for IndexListActiveEnvoyStates {
	const TAG: CommandTag = CommandTag(PEGBOARD_INDEX_LIST_ACTIVE_ENVOY_STATES);
	const CREATE_IF_MISSING: bool = false;
	type Request = rivet_data::versioned::IndexListActiveEnvoyStatesRequest;
	type Response = rivet_data::versioned::IndexListActiveEnvoyStatesResponse;
}

pub fn register(registry: &mut CommandRegistry, ctx: &EngineContext) -> Result<()> {
	let buckets = ctx.config().pegboard().actor_index_buckets_per_name();
	crate::catalog::register(registry, ctx)?;
	let concurrency = ctx.config().pegboard().command_concurrency.index();
	registry.register_with_concurrency::<IndexInsertActor>(
		concurrency,
		move |db: KvHandle, request| insert_actor_handler(buckets, db, request),
	)?;
	let clock = ctx.env().clock();
	registry.register_with_concurrency::<IndexDeleteActor>(
		concurrency,
		move |db: KvHandle, request| delete_actor_handler(buckets, clock.clone(), db, request),
	)?;
	registry.register_snapshot_read::<IndexReadDestroyedActor>(
		concurrency,
		None,
		move |db: KvHandle, request| read_destroyed_actor_handler(buckets, db, request),
	)?;
	registry.register_with_concurrency::<IndexListActors>(
		concurrency,
		move |db: KvHandle, request| list_actors_handler(buckets, db, request),
	)?;
	registry
		.register_with_concurrency::<IndexInsertEnvoy>(concurrency, |db: KvHandle, request| {
			insert_envoy_handler(db, request)
		})?;
	registry
		.register_with_concurrency::<IndexListEnvoys>(concurrency, |db: KvHandle, request| {
			list_envoys_handler(db, request)
		})?;
	registry.register_with_concurrency::<IndexUpsertActiveEnvoy>(
		concurrency,
		|db: KvHandle, request| upsert_active_envoy_handler(db, request),
	)?;
	registry.register_with_concurrency::<IndexDeleteActiveEnvoy>(
		concurrency,
		|db: KvHandle, request| delete_active_envoy_handler(db, request),
	)?;
	registry.register_with_concurrency::<IndexListEnvoyPools>(
		concurrency,
		|db: KvHandle, request| list_envoy_pools_handler(db, request),
	)?;
	let clock = ctx.env().clock();
	registry.register_with_concurrency::<IndexListActiveEnvoys>(
		concurrency,
		move |db: KvHandle, request| list_active_envoys_handler(clock.clone(), db, request),
	)?;
	let clock = ctx.env().clock();
	registry.register_with_concurrency::<IndexListActiveEnvoyStates>(
		concurrency,
		move |db: KvHandle, request| list_active_envoy_states_handler(clock.clone(), db, request),
	)?;

	Ok(())
}

/// Fails unless `db` is the bucket volume of its namespace and name that holds `actor_id`'s entry.
fn ensure_actor_bucket(db: &KvHandle, actor_id: Id, buckets: u64) -> Result<()> {
	let volume = ActorIndexVolume::from_volume_key(db.volume_key())?;
	ensure!(
		volume.bucket == actor_index_bucket(actor_id, buckets),
		"wrong actor index bucket volume"
	);
	Ok(())
}

async fn insert_actor_handler(
	buckets: u64,
	db: KvHandle,
	request: wire::IndexInsertActorRequest,
) -> Result<()> {
	let actor_id = Id::from_slice(&request.actor_id).context("invalid index actor id")?;
	ensure_actor_bucket(&db, actor_id, buckets)?;
	let key = ActorIndexKey::new(request.create_unix_ms, actor_id);
	let destroyed_key = DestroyedActorIndexKey::new(request.create_unix_ms, actor_id);

	// An insert that arrives after the incarnation was destroyed must not list it as live again.
	let mut txn = db.begin().await?;
	if txn.get(&destroyed_key.pack()).await?.is_some() {
		return Ok(());
	}
	txn.put(&key.pack(), &[])?;
	db.commit(txn).await?;

	Ok(())
}

/// Moves the actor's creation entry to the destroyed entries, stamped with the current time, so
/// listings that ask for destroyed actors still find it. Deleting an entry that was already
/// moved keeps the first stamp, so retries are safe.
async fn delete_actor_handler(
	buckets: u64,
	clock: Arc<dyn Clock>,
	db: KvHandle,
	request: wire::IndexDeleteActorRequest,
) -> Result<()> {
	let actor_id = Id::from_slice(&request.actor_id).context("invalid index actor id")?;
	ensure_actor_bucket(&db, actor_id, buckets)?;
	let key = ActorIndexKey::new(request.create_unix_ms, actor_id);
	let destroyed_key = DestroyedActorIndexKey::new(request.create_unix_ms, actor_id);
	let now_unix_ms = clock.observe()?.unix_ms;

	let mut txn = db.begin().await?;
	txn.delete(&key.pack())?;
	if txn.get(&destroyed_key.pack()).await?.is_none() {
		let entry = wire::DestroyedActorIndexEntry {
			destroy_unix_ms: now_unix_ms,
		};
		txn.put(&destroyed_key.pack(), &destroyed_key.serialize(entry)?)?;
	}
	db.commit(txn).await?;

	Ok(())
}

async fn read_destroyed_actor_handler(
	buckets: u64,
	db: KvHandle,
	request: wire::IndexReadDestroyedActorRequest,
) -> Result<wire::IndexReadDestroyedActorResponse> {
	let actor_id = Id::from_slice(&request.actor_id).context("invalid index actor id")?;
	ensure_actor_bucket(&db, actor_id, buckets)?;
	let key = DestroyedActorIndexKey::new(request.create_unix_ms, actor_id);

	let destroy_unix_ms = match db.snapshot().await?.get(&key.pack()).await? {
		Some(value) => Some(key.deserialize(value.into())?.destroy_unix_ms),
		None => None,
	};
	Ok(wire::IndexReadDestroyedActorResponse { destroy_unix_ms })
}

/// Lists one bucket's entries, newest first, strictly after `request.after`, with its
/// destroyed entries merged in when the request asks for them. Reads at most
/// `min(limit, 100) + 1` entries of each kind.
async fn list_actors_handler(
	buckets: u64,
	db: KvHandle,
	request: wire::IndexListActorsRequest,
) -> Result<wire::IndexListActorsResponse> {
	let volume = ActorIndexVolume::from_volume_key(db.volume_key())?;
	ensure!(volume.bucket < buckets, "actor index bucket out of range");
	ensure!(
		request.limit > 0,
		"actor index listing limit must be positive"
	);
	let limit = (request.limit as usize).min(PAGE_SIZE);
	let after = match &request.after {
		Some(after) => Some((
			after.create_unix_ms,
			Id::from_slice(&after.actor_id).context("invalid index cursor actor id")?,
		)),
		None => None,
	};

	let mut snapshot = db.snapshot().await?;

	let live_range = ActorIndexPrefix::new().range();
	let live_upper = match after {
		Some((created_at, actor_id)) => ActorIndexKey::new(created_at, actor_id).pack().to_vec(),
		None => live_range.end.clone(),
	};
	let live_rows = snapshot
		.scan(
			(
				Bound::Included(live_range.start.as_slice()),
				Bound::Excluded(live_upper.as_slice()),
			),
			Some(limit + 1),
			true,
		)
		.await?;
	let mut entries = Vec::with_capacity(live_rows.len());
	for (key, _) in live_rows {
		let key = ActorIndexKey::unpack(&key).context("invalid index actor key")?;
		entries.push((cursor_of(key.created_at, key.actor_id), None));
	}

	if request.include_destroyed {
		let destroyed_range = DestroyedActorIndexPrefix::new().range();
		let destroyed_upper = match after {
			Some((created_at, actor_id)) => DestroyedActorIndexKey::new(created_at, actor_id)
				.pack()
				.to_vec(),
			None => destroyed_range.end.clone(),
		};
		let destroyed_rows = snapshot
			.scan(
				(
					Bound::Included(destroyed_range.start.as_slice()),
					Bound::Excluded(destroyed_upper.as_slice()),
				),
				Some(limit + 1),
				true,
			)
			.await?;
		for (raw_key, value) in destroyed_rows {
			let key = DestroyedActorIndexKey::unpack(&raw_key)
				.context("invalid destroyed actor index key")?;
			let entry = key.deserialize(value.into())?;
			entries.push((
				cursor_of(key.created_at, key.actor_id),
				Some(entry.destroy_unix_ms),
			));
		}

		// Both scans are newest first; merge them into one order.
		entries.sort_by(|(a, _), (b, _)| compare_entries(b, a));
	}

	let has_more = entries.len() > limit;
	entries.truncate(limit);
	let mut actors = Vec::with_capacity(entries.len());
	let mut destroyed = Vec::new();
	for (cursor, destroy_unix_ms) in entries {
		if let Some(destroy_unix_ms) = destroy_unix_ms {
			destroyed.push(wire::IndexDestroyedActor {
				create_unix_ms: cursor.create_unix_ms,
				actor_id: cursor.actor_id.clone(),
				destroy_unix_ms,
			});
		}
		actors.push(cursor);
	}

	Ok(wire::IndexListActorsResponse {
		actors,
		has_more,
		destroyed,
	})
}

fn cursor_of(create_unix_ms: i64, actor_id: Id) -> wire::IndexActorCursor {
	wire::IndexActorCursor {
		create_unix_ms,
		actor_id: actor_id.as_bytes().into(),
	}
}

async fn insert_envoy_handler(db: KvHandle, request: wire::IndexInsertEnvoyRequest) -> Result<()> {
	let volume = EnvoyIndexVolume::from_volume_key(db.volume_key())?;
	let key = EnvoyIndexKey::new(request.create_unix_ms, request.envoy_key.to_string());

	let mut txn = db.begin().await?;
	txn.put(&key.pack(), &[])?;
	db.commit(txn).await?;
	crate::metrics::ENVOY_INDEX_UPSERTS
		.with_label_values(&[&volume.namespace, "connect"])
		.inc();

	Ok(())
}

async fn list_envoys_handler(
	db: KvHandle,
	request: wire::IndexListEnvoysRequest,
) -> Result<wire::IndexListEnvoysResponse> {
	EnvoyIndexVolume::from_volume_key(db.volume_key())?;
	let envoy_range = EnvoyIndexPrefix::new().range();
	let (entries, has_more) = scan_page(&db, envoy_range, request.cursor.as_deref()).await?;
	let cursor = if has_more {
		entries.last().map(|(key, _)| key.clone())
	} else {
		None
	};

	let mut envoy_keys = Vec::with_capacity(entries.len());
	for (raw_key, _) in entries {
		let key = EnvoyIndexKey::unpack(&raw_key).context("invalid index envoy key")?;
		envoy_keys.push(key.envoy_key.into());
	}

	Ok(wire::IndexListEnvoysResponse {
		envoy_keys,
		cursor: cursor.map(Into::into),
	})
}

/// Lists configured pools for internal discovery of their envoy indexes.
async fn list_envoy_pools_handler(
	db: KvHandle,
	_request: (),
) -> Result<wire::IndexListEnvoyPoolsResponse> {
	EnvoyPoolIndexVolume::from_volume_key(db.volume_key())?;
	let range = EnvoyPoolPrefix::new().range();
	let rows = db
		.snapshot()
		.await?
		.scan(
			(
				Bound::Included(range.start.as_slice()),
				Bound::Excluded(range.end.as_slice()),
			),
			None,
			false,
		)
		.await?;
	let pools = rows
		.into_iter()
		.map(|(key, _)| {
			EnvoyPoolKey::unpack(&key)
				.map(|key| key.pool_name.into())
				.context("invalid envoy pool key")
		})
		.collect::<Result<_>>()?;
	Ok(wire::IndexListEnvoyPoolsResponse { pools })
}

/// The stored state of an envoy in the active index, if it has one.
async fn read_active_envoy_state(
	txn: &mut depot_worker::command::Transaction,
	state_key: &ActiveEnvoyStateKey,
) -> Result<Option<wire::ActiveEnvoyState>> {
	let Some(stored) = txn.get(&state_key.pack()).await? else {
		return Ok(None);
	};
	Ok(Some(state_key.deserialize(stored.into())?))
}

/// Deletes the index entries the stored state says the envoy sits at.
fn delete_active_envoy_entries(
	txn: &mut depot_worker::command::Transaction,
	envoy_key: &str,
	stored: &wire::ActiveEnvoyState,
) -> Result<()> {
	for name in &stored.names {
		let key = ActiveEnvoyIndexKey::new(
			name.to_string(),
			stored.heartbeat_unix_ms,
			envoy_key.to_owned(),
		);
		txn.delete(&key.pack())?;
	}
	Ok(())
}

/// Replaces the envoy's index entries and state with the request's, in one transaction, unless
/// the stored state belongs to a newer connection (`superseded`) or to a newer heartbeat of the
/// same connection, which is a reordered or retried write and changes nothing.
async fn upsert_active_envoy_handler(
	db: KvHandle,
	request: wire::IndexUpsertActiveEnvoyRequest,
) -> Result<wire::IndexUpsertActiveEnvoyResponse> {
	let volume = ActiveEnvoyIndexVolume::from_volume_key(db.volume_key())?;
	let envoy_key = request.envoy_key.to_string();
	let state_key = ActiveEnvoyStateKey::new(envoy_key.clone());

	let mut txn = db.begin().await?;
	if let Some(stored) = read_active_envoy_state(&mut txn, &state_key).await? {
		if stored.connection_epoch > request.connection_epoch {
			return Ok(wire::IndexUpsertActiveEnvoyResponse { superseded: true });
		}
		if stored.connection_epoch == request.connection_epoch
			&& stored.heartbeat_unix_ms > request.heartbeat_unix_ms
		{
			return Ok(wire::IndexUpsertActiveEnvoyResponse { superseded: false });
		}
		delete_active_envoy_entries(&mut txn, &envoy_key, &stored)?;
	}

	let entry = wire::ActiveEnvoyIndexEntry {
		worker_id: request.worker_id.clone(),
		connection_id: request.connection_id.clone(),
	};
	for name in &request.names {
		let key = ActiveEnvoyIndexKey::new(
			name.to_string(),
			request.heartbeat_unix_ms,
			envoy_key.clone(),
		);
		txn.put(&key.pack(), &key.serialize(entry.clone())?)?;
	}
	let state = wire::ActiveEnvoyState {
		envoy_key: request.envoy_key,
		connection_epoch: request.connection_epoch,
		heartbeat_unix_ms: request.heartbeat_unix_ms,
		names: request.names,
	};
	txn.put(&state_key.pack(), &state_key.serialize(state)?)?;
	db.commit(txn).await?;
	crate::metrics::ENVOY_INDEX_UPSERTS
		.with_label_values(&[&volume.namespace, "heartbeat"])
		.inc();

	Ok(wire::IndexUpsertActiveEnvoyResponse { superseded: false })
}

/// Removes the envoy's index entries and state, unless a newer connection owns them. No state
/// means nothing to remove.
async fn delete_active_envoy_handler(
	db: KvHandle,
	request: wire::IndexDeleteActiveEnvoyRequest,
) -> Result<()> {
	let volume = ActiveEnvoyIndexVolume::from_volume_key(db.volume_key())?;
	let envoy_key = request.envoy_key.to_string();
	let state_key = ActiveEnvoyStateKey::new(envoy_key.clone());

	let mut txn = db.begin().await?;
	let Some(stored) = read_active_envoy_state(&mut txn, &state_key).await? else {
		return Ok(());
	};
	if stored.connection_epoch > request.connection_epoch {
		return Ok(());
	}
	delete_active_envoy_entries(&mut txn, &envoy_key, &stored)?;
	txn.delete(&state_key.pack())?;
	db.commit(txn).await?;
	crate::metrics::ENVOY_INDEX_UPSERTS
		.with_label_values(&[&volume.namespace, "disconnect"])
		.inc();

	Ok(())
}

/// The oldest heartbeat still inside [`crate::ENVOY_HEARTBEAT_TIMEOUT_MS`] at `now_unix_ms`.
fn oldest_eligible_heartbeat(now_unix_ms: i64) -> i64 {
	now_unix_ms.saturating_sub(crate::ENVOY_HEARTBEAT_TIMEOUT_MS) + 1
}

/// Lists one page of the active envoys serving `request.name`, in ascending heartbeat order, as
/// the allocations an actor of that name could be given. The scan starts at the oldest
/// heartbeat still inside [`crate::ENVOY_HEARTBEAT_TIMEOUT_MS`] now, or after the cursor when
/// the cursor is newer, so expired entries are never returned.
async fn list_active_envoys_handler(
	clock: Arc<dyn Clock>,
	db: KvHandle,
	request: wire::IndexListActiveEnvoysRequest,
) -> Result<wire::IndexListActiveEnvoysResponse> {
	ActiveEnvoyIndexVolume::from_volume_key(db.volume_key())?;
	let now_unix_ms = clock.observe()?.unix_ms;
	let name = request.name.to_string();
	let range = ActiveEnvoyIndexNamePrefix::new(name.clone()).range();

	// An envoy is eligible while its heartbeat age is below the timeout. The empty envoy key sorts
	// before every other key at the same heartbeat.
	let window_start =
		ActiveEnvoyIndexKey::new(name, oldest_eligible_heartbeat(now_unix_ms), String::new())
			.pack()
			.to_vec();

	let lower = match request.cursor.as_deref() {
		Some(cursor) => {
			ensure!(
				range.start.as_slice() <= cursor && cursor < range.end.as_slice(),
				"cursor outside index range"
			);
			// The cursor must encode an entire entry, not just a heartbeat prefix.
			ActiveEnvoyIndexKey::unpack(cursor).context("invalid index cursor")?;
			if cursor < window_start.as_slice() {
				Bound::Included(window_start.as_slice())
			} else {
				Bound::Excluded(cursor)
			}
		}
		None => Bound::Included(window_start.as_slice()),
	};

	let mut snapshot = db.snapshot().await?;
	let mut entries = snapshot
		.scan(
			(lower, Bound::Excluded(range.end.as_slice())),
			Some(PAGE_SIZE + 1),
			false,
		)
		.await?;
	let has_more = entries.len() > PAGE_SIZE;
	entries.truncate(PAGE_SIZE);

	let cursor = if has_more {
		entries.last().map(|(key, _)| key.clone())
	} else {
		None
	};

	let mut envoys = Vec::with_capacity(entries.len());
	for (raw_key, value) in entries {
		let key =
			ActiveEnvoyIndexKey::unpack(&raw_key).context("invalid active envoy index key")?;
		let entry = key
			.deserialize(value.into())
			.context("invalid active envoy index entry")?;
		envoys.push(wire::ActorAllocation {
			envoy_key: key.envoy_key.into(),
			worker_id: entry.worker_id,
			connection_id: entry.connection_id,
		});
	}

	Ok(wire::IndexListActiveEnvoysResponse {
		envoys,
		cursor: cursor.map(Into::into),
	})
}

/// Lists one page of envoy states in envoy key order, skipping envoys whose heartbeat is outside
/// the eligibility window now. The cursor is the last state scanned, skipped or not, so a page
/// of expired states still advances.
async fn list_active_envoy_states_handler(
	clock: Arc<dyn Clock>,
	db: KvHandle,
	request: wire::IndexListActiveEnvoyStatesRequest,
) -> Result<wire::IndexListActiveEnvoyStatesResponse> {
	let volume = ActiveEnvoyIndexVolume::from_volume_key(db.volume_key())?;
	let now_unix_ms = clock.observe()?.unix_ms;
	let range = ActiveEnvoyStatePrefix::new().range();
	let oldest_eligible = oldest_eligible_heartbeat(now_unix_ms);

	let lower = match request.cursor.as_deref() {
		Some(cursor) => {
			ensure!(
				range.start.as_slice() <= cursor && cursor < range.end.as_slice(),
				"cursor outside index range"
			);
			ActiveEnvoyStateKey::unpack(cursor).context("invalid index cursor")?;
			Bound::Excluded(cursor)
		}
		None => Bound::Included(range.start.as_slice()),
	};

	let mut snapshot = db.snapshot().await?;
	let mut entries = snapshot
		.scan(
			(lower, Bound::Excluded(range.end.as_slice())),
			Some(PAGE_SIZE + 1),
			false,
		)
		.await?;
	let has_more = entries.len() > PAGE_SIZE;
	entries.truncate(PAGE_SIZE);

	let cursor = if has_more {
		entries.last().map(|(key, _)| key.clone())
	} else {
		None
	};

	let mut envoys = Vec::with_capacity(entries.len());
	for (raw_key, value) in entries {
		let key =
			ActiveEnvoyStateKey::unpack(&raw_key).context("invalid active envoy state key")?;
		let state = key
			.deserialize(value.into())
			.context("invalid active envoy state")?;
		ensure!(
			state.envoy_key.to_string() == key.envoy_key,
			"active envoy state names another envoy than its key"
		);
		if state.heartbeat_unix_ms < oldest_eligible {
			crate::metrics::ENVOY_INDEX_STALE_ROWS
				.with_label_values(&[&volume.namespace, "heartbeat_expired", "skipped"])
				.inc();
			continue;
		}
		envoys.push(state);
	}

	Ok(wire::IndexListActiveEnvoyStatesResponse {
		envoys,
		cursor: cursor.map(Into::into),
	})
}

/// Scans one page of envoy index entries in ascending key order and reports whether more remain.
async fn scan_page(
	db: &KvHandle,
	range: Range<Vec<u8>>,
	cursor: Option<&[u8]>,
) -> Result<(Vec<(Vec<u8>, Vec<u8>)>, bool)> {
	if let Some(cursor) = cursor {
		ensure!(
			range.start.as_slice() <= cursor && cursor < range.end.as_slice(),
			"cursor outside index range"
		);
		// The cursor must encode an entire entry, not just a timestamp prefix.
		EnvoyIndexKey::unpack(cursor).context("invalid index cursor")?;
	}
	let mut snapshot = db.snapshot().await?;
	let mut entries = snapshot
		.scan(
			(
				cursor.map_or(Bound::Included(range.start.as_slice()), Bound::Excluded),
				Bound::Excluded(range.end.as_slice()),
			),
			Some(PAGE_SIZE + 1),
			false,
		)
		.await?;

	let has_more = entries.len() > PAGE_SIZE;
	entries.truncate(PAGE_SIZE);

	Ok((entries, has_more))
}

/// Reads the data of every indexed envoy of one namespace and pool, in creation order. Each
/// envoy's data is read from its own envoy volume; an indexed envoy with no envoy volume
/// or no stored data is skipped. A pool with no index volume has no envoys.
pub async fn list_envoys(
	ctx: &EngineContext,
	namespace: &str,
	pool_name: &str,
) -> Result<Vec<wire::EnvoyData>> {
	let index_volume =
		EnvoyIndexVolume::new(namespace.to_owned(), pool_name.to_owned()).volume_key();

	let mut envoy_keys = Vec::new();
	let mut cursor = None;
	loop {
		let page = ctx
			.depot()
			.command::<IndexListEnvoys>(&index_volume, wire::IndexListEnvoysRequest { cursor })
			.await;
		let page = match page {
			Ok(page) => page,
			Err(error) if is_volume_not_found(&error) => break,
			Err(error) => return Err(error),
		};
		envoy_keys.extend(page.envoy_keys);

		match page.cursor {
			Some(next) => cursor = Some(next),
			None => break,
		}
	}

	let mut envoys = Vec::with_capacity(envoy_keys.len());
	for envoy_key in envoy_keys {
		let volume = EnvoyVolume::new(
			namespace.to_owned(),
			pool_name.to_owned(),
			envoy_key.to_string(),
		)
		.volume_key();
		let response = match ctx.depot().command::<EnvoyRead>(&volume, ()).await {
			Ok(response) => response,
			Err(error) if is_volume_not_found(&error) => {
				crate::metrics::ENVOY_INDEX_STALE_ROWS
					.with_label_values(&[namespace, "missing_record", "skipped"])
					.inc();
				continue;
			}
			Err(error) => return Err(error),
		};
		if let Some(data) = response.data {
			envoys.push(data);
		} else {
			crate::metrics::ENVOY_INDEX_STALE_ROWS
				.with_label_values(&[namespace, "missing_record", "skipped"])
				.inc();
		}
	}

	Ok(envoys)
}

/// Lists every envoy of one namespace and pool serving `name` whose latest heartbeat is inside
/// the eligibility window, as the allocations an actor of that name could be given, in
/// heartbeat order. A pool with no active index volume has no envoys.
pub async fn list_active_envoys(
	ctx: &EngineContext,
	namespace: &str,
	pool_name: &str,
	name: &str,
) -> Result<Vec<wire::ActorAllocation>> {
	let index_volume =
		ActiveEnvoyIndexVolume::new(namespace.to_owned(), pool_name.to_owned()).volume_key();

	let mut envoys = Vec::new();
	let mut cursor = None;
	loop {
		let page = ctx
			.depot()
			.command::<IndexListActiveEnvoys>(
				&index_volume,
				wire::IndexListActiveEnvoysRequest {
					name: name.to_owned().into(),
					cursor,
				},
			)
			.await;
		let page = match page {
			Ok(page) => page,
			Err(error) if is_volume_not_found(&error) => break,
			Err(error) => return Err(error),
		};
		envoys.extend(page.envoys);

		match page.cursor {
			Some(next) => cursor = Some(next),
			None => break,
		}
	}

	Ok(envoys)
}

/// Lists the state of every envoy of one namespace and pool whose latest heartbeat is inside the
/// eligibility window, in envoy key order. A pool with no active index volume has no envoys.
pub async fn list_active_envoy_states(
	ctx: &EngineContext,
	namespace: &str,
	pool_name: &str,
) -> Result<Vec<wire::ActiveEnvoyState>> {
	let index_volume =
		ActiveEnvoyIndexVolume::new(namespace.to_owned(), pool_name.to_owned()).volume_key();

	let mut envoys = Vec::new();
	let mut cursor = None;
	loop {
		let page = ctx
			.depot()
			.command::<IndexListActiveEnvoyStates>(
				&index_volume,
				wire::IndexListActiveEnvoyStatesRequest { cursor },
			)
			.await;
		let page = match page {
			Ok(page) => page,
			Err(error) if is_volume_not_found(&error) => break,
			Err(error) => return Err(error),
		};
		envoys.extend(page.envoys);

		match page.cursor {
			Some(next) => cursor = Some(next),
			None => break,
		}
	}

	Ok(envoys)
}

/// One page of [`list_active_envoy_states_page`].
pub struct ActiveEnvoyStatesPage {
	/// Live envoy states in envoy key order.
	pub envoys: Vec<wire::ActiveEnvoyState>,
	/// The envoy key the next page starts after, present while more states may follow.
	pub after: Option<String>,
}

/// Reads one index page of the live envoy states of one namespace and pool, in envoy key order,
/// strictly after the envoy key `after`. A page scans at most 100 states, so it may come back
/// short or empty while `after` is still set. A pool with no active index volume has no envoys.
pub async fn list_active_envoy_states_page(
	ctx: &EngineContext,
	namespace: &str,
	pool_name: &str,
	after: Option<&str>,
) -> Result<ActiveEnvoyStatesPage> {
	let index_volume =
		ActiveEnvoyIndexVolume::new(namespace.to_owned(), pool_name.to_owned()).volume_key();
	let cursor = after.map(|envoy_key| ActiveEnvoyStateKey::new(envoy_key.to_owned()).pack());

	let page = ctx
		.depot()
		.command::<IndexListActiveEnvoyStates>(
			&index_volume,
			wire::IndexListActiveEnvoyStatesRequest {
				cursor: cursor.map(|cursor| cursor.to_vec().into()),
			},
		)
		.await;
	let page = match page {
		Ok(page) => page,
		Err(error) if is_volume_not_found(&error) => {
			return Ok(ActiveEnvoyStatesPage {
				envoys: Vec::new(),
				after: None,
			});
		}
		Err(error) => return Err(error),
	};

	let after = page
		.cursor
		.map(|cursor| {
			ActiveEnvoyStateKey::unpack(&cursor)
				.map(|key| key.envoy_key)
				.context("invalid active envoy state cursor")
		})
		.transpose()?;
	Ok(ActiveEnvoyStatesPage {
		envoys: page.envoys,
		after,
	})
}

/// Reads the stored record of one envoy, or `None` when it has no envoy volume or no record.
pub async fn read_envoy(
	ctx: &EngineContext,
	namespace: &str,
	pool_name: &str,
	envoy_key: &str,
) -> Result<Option<wire::EnvoyData>> {
	let volume = EnvoyVolume::new(
		namespace.to_owned(),
		pool_name.to_owned(),
		envoy_key.to_owned(),
	)
	.volume_key();
	match ctx.depot().command::<EnvoyRead>(&volume, ()).await {
		Ok(response) => Ok(response.data),
		Err(error) if is_volume_not_found(&error) => Ok(None),
		Err(error) => Err(error),
	}
}

/// One page of a merged actor listing.
pub struct ActorPage {
	/// Entries in descending `(create_unix_ms, actor_id)` order.
	pub actors: Vec<wire::IndexActorCursor>,
	/// The position to resume from, present only when more entries may follow.
	pub cursor: Option<wire::IndexActorCursor>,
	/// When each destroyed entry of `actors` left the live index. Empty unless the listing
	/// included destroyed actors.
	pub destroyed: HashMap<wire::IndexActorCursor, i64>,
}

/// Lists up to `limit` (at most 100) actors of `namespace` named `name` across that name's
/// buckets, newest first, strictly after `after`. Destroyed incarnations are listed too when
/// `include_destroyed` is set.
///
/// Each bucket is read at a different moment, so an entry that lands behind the listing's position
/// appears only in a fresh listing.
pub async fn list_actors(
	ctx: &EngineContext,
	namespace: &str,
	name: &str,
	after: Option<wire::IndexActorCursor>,
	limit: usize,
	include_destroyed: bool,
) -> Result<ActorPage> {
	ensure!(limit > 0, "actor listing limit must be positive");
	let limit = limit.min(PAGE_SIZE);
	let buckets = ctx.config().pegboard().actor_index_buckets_per_name();
	let concurrency =
		(buckets as usize).min(ctx.config().pegboard().actor_index_list_concurrency());
	crate::metrics::ACTOR_INDEX_LIST_BUCKETS.observe(buckets as f64);

	#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
	let started = std::time::Instant::now();

	let mut merge = ListMerge::new(buckets as usize, after, limit);

	// Read a small first page from every bucket.
	let first_page = limit.min(MERGE_FIRST_PAGE);
	let pages = futures_util::stream::iter(0..buckets)
		.map(|bucket| {
			let request = merge.page_request(bucket as usize);
			async move {
				let page = list_bucket(
					ctx,
					namespace,
					name,
					bucket,
					request,
					first_page,
					include_destroyed,
				)
				.await;
				(bucket, page)
			}
		})
		.buffer_unordered(concurrency)
		.collect::<Vec<_>>()
		.await;
	let mut destroyed = HashMap::new();
	for (bucket, page) in pages {
		let page = page?;
		collect_destroyed(&mut destroyed, page.destroyed);
		merge.push_page(bucket as usize, page.actors, page.has_more)?;
	}

	// Emit entries, refilling a bucket only when the merge cannot order its next entry.
	let mut actors = Vec::with_capacity(limit);
	loop {
		match merge.next() {
			MergeStep::Entry(entry) => actors.push(entry),
			MergeStep::NeedPage(bucket) => {
				crate::metrics::ACTOR_INDEX_LIST_REFILLS.inc();
				let request = merge.page_request(bucket);
				let remaining = limit.saturating_sub(actors.len()).max(1);
				let page = list_bucket(
					ctx,
					namespace,
					name,
					bucket as u64,
					request,
					remaining,
					include_destroyed,
				)
				.await?;
				collect_destroyed(&mut destroyed, page.destroyed);
				merge.push_page(bucket, page.actors, page.has_more)?;
			}
			MergeStep::Done => break,
		}
	}

	crate::metrics::ACTOR_INDEX_LIST_DURATION.observe(started.elapsed().as_secs_f64());

	destroyed.retain(|entry, _| actors.contains(entry));
	Ok(ActorPage {
		cursor: merge.resume_cursor(),
		actors,
		destroyed,
	})
}

fn collect_destroyed(
	destroyed: &mut HashMap<wire::IndexActorCursor, i64>,
	entries: Vec<wire::IndexDestroyedActor>,
) {
	for entry in entries {
		let cursor = wire::IndexActorCursor {
			create_unix_ms: entry.create_unix_ms,
			actor_id: entry.actor_id,
		};
		destroyed.insert(cursor, entry.destroy_unix_ms);
	}
}

/// Reads when the incarnation of `actor_id` created at `create_unix_ms` was destroyed, or
/// `None` when the index holds no destroyed entry for it.
pub async fn read_destroyed_actor(
	ctx: &EngineContext,
	namespace: &str,
	name: &str,
	actor_id: Id,
	create_unix_ms: i64,
) -> Result<Option<i64>> {
	let buckets = ctx.config().pegboard().actor_index_buckets_per_name();
	let volume =
		ActorIndexVolume::for_actor(namespace.to_owned(), name.to_owned(), actor_id, buckets)
			.volume_key();
	let result = ctx
		.depot()
		.command::<IndexReadDestroyedActor>(
			&volume,
			wire::IndexReadDestroyedActorRequest {
				create_unix_ms,
				actor_id: actor_id.as_bytes().into(),
			},
		)
		.await;
	match result {
		Ok(response) => Ok(response.destroy_unix_ms),
		Err(error) if is_volume_not_found(&error) => Ok(None),
		Err(error) => Err(error),
	}
}

/// Reads one bucket page. A bucket volume that was never created is empty.
async fn list_bucket(
	ctx: &EngineContext,
	namespace: &str,
	name: &str,
	bucket: u64,
	after: Option<wire::IndexActorCursor>,
	limit: usize,
	include_destroyed: bool,
) -> Result<wire::IndexListActorsResponse> {
	let result = ctx
		.depot()
		.command::<IndexListActors>(
			&ActorIndexVolume::new(namespace.to_owned(), name.to_owned(), bucket).volume_key(),
			wire::IndexListActorsRequest {
				after,
				limit: limit as u32,
				include_destroyed,
			},
		)
		.await;

	match result {
		Ok(page) => Ok(page),
		Err(error) if is_volume_not_found(&error) => Ok(wire::IndexListActorsResponse {
			actors: Vec::new(),
			has_more: false,
			destroyed: Vec::new(),
		}),
		Err(error) => Err(error),
	}
}

/// Lists the configured pools of `namespace`. A namespace with no catalog
/// volume has no pools.
pub async fn list_envoy_pools(ctx: &EngineContext, namespace: &str) -> Result<Vec<String>> {
	let result = ctx
		.depot()
		.command::<IndexListEnvoyPools>(
			&EnvoyPoolIndexVolume::new(namespace.to_owned()).volume_key(),
			(),
		)
		.await;
	match result {
		Ok(response) => Ok(response.pools.into_iter().map(Into::into).collect()),
		Err(error) if is_volume_not_found(&error) => Ok(Vec::new()),
		Err(error) => Err(error),
	}
}

/// Whether `error` is Depot's `volume_not_found`, which a non-creating command returns for a
/// volume that does not exist.
pub fn is_volume_not_found(error: &anyhow::Error) -> bool {
	error
		.downcast_ref::<rivet_error::RivetError>()
		.is_some_and(|error| error.group() == "depot" && error.code() == "volume_not_found")
}

/// Orders listing entries the way the index keys sort, so the merge and the bucket scans agree.
fn compare_entries(a: &wire::IndexActorCursor, b: &wire::IndexActorCursor) -> Ordering {
	a.create_unix_ms
		.cmp(&b.create_unix_ms)
		.then_with(|| a.actor_id.cmp(&b.actor_id))
}

/// The next step of a [`ListMerge`].
#[derive(Debug, PartialEq, Eq)]
pub enum MergeStep {
	/// The next entry in descending order.
	Entry(wire::IndexActorCursor),
	/// The merge cannot order its next entry until this bucket's next page arrives.
	NeedPage(usize),
	/// The page is full or every bucket is exhausted.
	Done,
}

#[derive(Default)]
struct BucketMerge {
	buffer: VecDeque<wire::IndexActorCursor>,
	/// Whether the bucket reported entries past its buffer, or was never read.
	has_more: bool,
	/// The last entry received from this bucket, where its next page starts.
	last_received: Option<wire::IndexActorCursor>,
}

/// A k-way merge of bucket pages into one descending order, with refills.
///
/// Every bucket is read from the same position in one total order, and an entry is emitted only
/// once every bucket's next entry is known (buffered or exhausted), so no entry is skipped or
/// repeated across pages.
pub struct ListMerge {
	buckets: Vec<BucketMerge>,
	after: Option<wire::IndexActorCursor>,
	limit: usize,
	emitted: usize,
	last_emitted: Option<wire::IndexActorCursor>,
}

impl ListMerge {
	pub fn new(buckets: usize, after: Option<wire::IndexActorCursor>, limit: usize) -> Self {
		Self {
			buckets: (0..buckets)
				.map(|_| BucketMerge {
					has_more: true,
					..Default::default()
				})
				.collect(),
			after,
			limit,
			emitted: 0,
			last_emitted: None,
		}
	}

	/// The position the next page of `bucket` starts after.
	pub fn page_request(&self, bucket: usize) -> Option<wire::IndexActorCursor> {
		self.buckets[bucket]
			.last_received
			.clone()
			.or_else(|| self.after.clone())
	}

	/// Adds one page of `bucket`, which must continue from [`Self::page_request`].
	pub fn push_page(
		&mut self,
		bucket: usize,
		entries: Vec<wire::IndexActorCursor>,
		has_more: bool,
	) -> Result<()> {
		ensure!(
			!entries.is_empty() || !has_more,
			"actor index bucket page reported more entries but returned none"
		);
		let state = self
			.buckets
			.get_mut(bucket)
			.context("actor index bucket out of range")?;
		if let Some(last) = entries.last() {
			state.last_received = Some(last.clone());
		}
		state.buffer.extend(entries);
		state.has_more = has_more;
		Ok(())
	}

	pub fn next(&mut self) -> MergeStep {
		if self.emitted >= self.limit {
			return MergeStep::Done;
		}

		// Every bucket's next entry must be known before the largest one can be emitted.
		for (bucket, state) in self.buckets.iter().enumerate() {
			if state.buffer.is_empty() && state.has_more {
				return MergeStep::NeedPage(bucket);
			}
		}

		let best = self
			.buckets
			.iter()
			.enumerate()
			.filter_map(|(bucket, state)| state.buffer.front().map(|head| (bucket, head)))
			.max_by(|(_, a), (_, b)| compare_entries(a, b))
			.map(|(bucket, _)| bucket);
		let Some(entry) = best.and_then(|bucket| self.buckets[bucket].buffer.pop_front()) else {
			return MergeStep::Done;
		};

		self.emitted += 1;
		self.last_emitted = Some(entry.clone());
		MergeStep::Entry(entry)
	}

	/// The cursor for the next page: the last emitted entry, when any bucket may still have
	/// entries.
	pub fn resume_cursor(&self) -> Option<wire::IndexActorCursor> {
		let more = self
			.buckets
			.iter()
			.any(|state| !state.buffer.is_empty() || state.has_more);
		if more {
			self.last_emitted.clone()
		} else {
			None
		}
	}
}







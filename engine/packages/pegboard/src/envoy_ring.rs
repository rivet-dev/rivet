//! The hash-ring active envoy index in each pool's volume, and actor allocation over it by power
//! of two choices.
//!
//! Every connected envoy has one [`PoolEnvoyStateKey`] row holding its route, version, the names
//! it serves, its latest heartbeat and the number of actors its connection holds. For each name
//! it also occupies [`RING_VIRTUAL_NODES`] positions on that name's ring, under its version, and
//! one entry under its version for the pool-wide newest live version. Ring and version entries
//! are written when a connection joins and deleted when it leaves; a heartbeat only overwrites
//! the state row, so its key never moves and heartbeats leave no deleted history behind.
//!
//! Each engine worker sends one [`PoolHeartbeats`] command per pool and flush, carrying every
//! envoy it holds in that pool, so the volume sees one commit per worker per flush rather than
//! one per envoy per ping. The heartbeat is also the node's pool admission ([`crate::pool_admin`]):
//! it lands only while the pool's record exists, and a stopping node keeps sending it, marked so
//! that allocation skips it.
//!
//! An allocation sends one [`PoolPickEnvoy`] command with a few random ring positions. Each
//! position walks forward to the first envoy whose heartbeat is inside
//! [`crate::ENVOY_HEARTBEAT_TIMEOUT_MS`], at the newest version that has one. Each name and
//! version carries a stamp of its newest heartbeat, so the pick passes over versions with no
//! live envoy without reading their entries. The caller keeps the candidate holding the fewest
//! actors. Envoy positions are hashes of the envoy key, so the envoy after a random position is
//! close to uniform, and comparing two of them by load keeps the most loaded envoy close to the
//! mean even with counts a heartbeat old.
//!
//! Before every live storage worker reads `pegboard.pool`, envoys stay in the legacy
//! heartbeat-ordered index of [`crate::index`]. An envoy is in exactly one of the two at a time:
//! each write removes the envoy's rows in the other one, fenced by connection epoch, so a worker
//! that has not yet seen the fleet upgrade and one that has never leave the same envoy indexed
//! twice.
//!
//! A process indexes its envoys in the legacy index until it has observed the whole fleet, which
//! also happens briefly after every restart. So a pick that finds no live ring envoy at a version
//! reads that version's legacy entries in the same snapshot, and an envoy indexed there is still
//! found. Legacy entries carry no actor count, so they are candidates only when the ring has none
//! at that version.
use crate::index::{
	active_envoy_window_start, bound_slice, defer_queued_heartbeat_for_reconciliation,
	delete_active_envoy_entries, ensure_active_envoy_versions, oldest_eligible_heartbeat,
	prepare_active_envoy_index, prepare_active_envoy_index_with_budget, read_active_envoy_state,
	shrink_index_page,
};
use crate::keys::kv::{
	ActiveEnvoyIndexKey, ActiveEnvoyIndexVersionPrefix, ActiveEnvoyStateKey, ActiveEnvoyVersionKey,
	ActiveEnvoyVersionNumberPrefix, PoolEnvoyRingKey, PoolEnvoyRingVersionPrefix,
	PoolEnvoyStateKey, PoolEnvoyStatePrefix, PoolEnvoyVersionKey, PoolEnvoyVersionNumberPrefix,
	PoolNameVersionKey, PoolNameVersionPrefix, PoolRecordKey, PoolVersionKey, PoolVersionPrefix,
	at_pool_write_version,
};
use crate::keys::partition::ActiveEnvoyIndexVolume;
use crate::pool_wire;
use anyhow::{Context, Result, ensure};
use bytes::Bytes;
use depot_protocol::command::{Command, CommandTag};
use depot_worker::command::{CommandRegistry, KvHandle, RangeLockGuard, Transaction};
use rivet_data::families::VersionFamily;
use rivet_engine_context::EngineContext;
use rivet_env::Clock;
use rivet_keys::keys::{
	KeyAddress,
	codec::{DescendingU64, FormalKey, SubspaceRange, ValueCodec},
	tags::depot_command::{PEGBOARD_POOL_HEARTBEATS, PEGBOARD_POOL_PICK_ENVOY},
};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound;
use std::sync::Arc;

/// Ring positions each envoy takes per name it serves. With V positions an envoy's share of the
/// ring is a sum of V arcs, so the luckiest envoy of N is picked about `1 + ln(N) / V` times as
/// often as the mean. A connection records the V it wrote, so changing this only affects envoys
/// that connect afterwards.
pub const RING_VIRTUAL_NODES: u32 = 8;

/// Ring positions an allocation samples. Two is the power of two choices; more cost reads and
/// help little.
pub const PICK_SAMPLES: usize = 2;

/// Ring entries one pick walks past, across all its positions and versions, before it gives up.
/// Entries of envoys that stopped heartbeating are skipped and count against it.
const PICK_MAX_SCAN: usize = 32;

/// Ring entries a walk reads per scan. The first entry after a pivot is usually live, so most
/// walks read one page.
const WALK_PAGE: usize = 4;

/// Entries of one version the pool-wide newest live version reads before moving to the next.
const VERSION_MAX_SCAN: usize = 32;

/// How old a version's stamp may grow before a heartbeat at that version rewrites it. A pick
/// allows the same slack, so a stamp lagging a live envoy's heartbeat never hides its version,
/// and a live version costs one stamp write per name this often rather than one per heartbeat.
pub(crate) const VERSION_STAMP_REFRESH_MS: i64 = crate::ENVOY_HEARTBEAT_TIMEOUT_MS / 3;

/// Version stamps a pick reads per scan while it passes over versions with no live envoy.
const STAMP_SCAN_PAGE: usize = 32;

/// Version stamps one refresh writes per transaction, halved when a transaction is too large.
const STAMP_WRITE_PAGE: usize = 128;

/// Heartbeats one [`PoolHeartbeats`] command carries. A worker holding more envoys of a pool
/// sends several commands in a row, so one transaction never grows with the worker's envoys.
pub const HEARTBEATS_PER_COMMAND: usize = 128;

/// Writes the latest state of every envoy one worker holds in a pool, in one transaction, and
/// replies per envoy.
pub struct PoolHeartbeats;

impl Command for PoolHeartbeats {
	const TAG: CommandTag = CommandTag(PEGBOARD_POOL_HEARTBEATS);
	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::PoolHeartbeatsRequest;
	type Response = rivet_data::versioned::PoolHeartbeatsResponse;
}

/// Resolves random ring positions to live envoys serving a name. A pool with no volume has no
/// envoys. It writes only to remove envoys whose heartbeat is past the lost threshold.
pub struct PoolPickEnvoy;

impl Command for PoolPickEnvoy {
	const TAG: CommandTag = CommandTag(PEGBOARD_POOL_PICK_ENVOY);
	const CREATE_IF_MISSING: bool = false;
	type Request = rivet_data::versioned::PoolPickEnvoyRequest;
	type Response = rivet_data::versioned::PoolPickEnvoyResponse;
}

pub fn register(registry: &mut CommandRegistry, ctx: &EngineContext) -> Result<()> {
	let concurrency = ctx.config().pegboard().command_concurrency.index();
	let lost_threshold_ms = lost_threshold_ms(ctx);
	let clock = ctx.env().clock();
	registry.register_with_concurrency::<PoolHeartbeats>(
		concurrency,
		move |db: KvHandle, request| {
			heartbeats_handler(clock.clone(), lost_threshold_ms, db, request)
		},
	)?;
	let clock = ctx.env().clock();
	registry
		.register_with_concurrency::<PoolPickEnvoy>(concurrency, move |db: KvHandle, request| {
			pick_handler(clock.clone(), lost_threshold_ms, db, request)
		})?;
	Ok(())
}

/// Whether every live storage worker reads `pegboard.pool`, so envoys may be written to the ring
/// and allocations picked from it. `false` while an older binary runs, and while this process
/// has not yet observed the whole fleet. Re-read on every use: the fleet only moves forward, so
/// once true it stays true.
pub fn ring_open(ctx: &EngineContext) -> bool {
	ctx.topology()
		.version_for(VersionFamily::PEGBOARD_POOL)
		.is_ok()
}

/// How long after its last heartbeat an envoy's rows may be removed by a reader that walks past
/// them. Longer than the eligibility window, so a connection whose heartbeat writes are only
/// delayed by a slow flush is skipped by allocation but not removed.
fn lost_threshold_ms(ctx: &EngineContext) -> i64 {
	i64::try_from(ctx.config().pegboard().envoy_lost_threshold())
		.unwrap_or(i64::MAX)
		.max(crate::ENVOY_HEARTBEAT_TIMEOUT_MS)
}

/// The ring position of an envoy's virtual node `index`.
pub fn ring_position(envoy_key: &str, index: u32) -> [u8; 16] {
	xxhash_rust::xxh3::xxh3_128_with_seed(envoy_key.as_bytes(), u64::from(index)).to_be_bytes()
}

/// Serializes the writes of one pool volume that decide admission: heartbeats and the pool admin
/// operations. Each holds this lock from its read of the pool record through its commit, so a
/// delete's check for admitted nodes and its tombstone cannot interleave with a heartbeat that
/// admits one.
pub(crate) async fn lock_pool(db: &KvHandle) -> Result<RangeLockGuard> {
	db.lock_partition(PoolRecordKey::new().range()).await
}

/// The newest heartbeat per name and version, and per version across the pool, that a set of
/// heartbeats reports.
#[derive(Default)]
pub(crate) struct VersionStamps {
	by_name: BTreeMap<(String, u32), i64>,
	by_version: BTreeMap<u32, i64>,
}

impl VersionStamps {
	/// Records `heartbeat_unix_ms` for `version` and every name the heartbeat serves there,
	/// keeping the newest of each.
	pub(crate) fn add(&mut self, names: &[vbare::Str], version: u32, heartbeat_unix_ms: i64) {
		for name in names {
			let newest = self
				.by_name
				.entry((name.to_string(), version))
				.or_insert(heartbeat_unix_ms);
			*newest = (*newest).max(heartbeat_unix_ms);
		}
		let newest = self.by_version.entry(version).or_insert(heartbeat_unix_ms);
		*newest = (*newest).max(heartbeat_unix_ms);
	}

	pub(crate) fn is_empty(&self) -> bool {
		self.by_name.is_empty() && self.by_version.is_empty()
	}

	/// The stamps whose stored heartbeat is missing or more than [`VERSION_STAMP_REFRESH_MS`]
	/// older than the one recorded here.
	pub(crate) async fn stale(&self, snapshot: &mut Transaction) -> Result<VersionStamps> {
		let mut stale = VersionStamps::default();
		for ((name, version), heartbeat_unix_ms) in &self.by_name {
			let key = PoolNameVersionKey::new(name.clone(), DescendingU64(u64::from(*version)));
			if stamp_is_stale(snapshot, &key, *heartbeat_unix_ms).await? {
				stale
					.by_name
					.insert((name.clone(), *version), *heartbeat_unix_ms);
			}
		}
		for (version, heartbeat_unix_ms) in &self.by_version {
			let key = PoolVersionKey::new(DescendingU64(u64::from(*version)));
			if stamp_is_stale(snapshot, &key, *heartbeat_unix_ms).await? {
				stale.by_version.insert(*version, *heartbeat_unix_ms);
			}
		}
		Ok(stale)
	}

	/// Each stamp as the key and encoded value it is written at.
	fn encode(&self, db: &KvHandle) -> Result<Vec<(Bytes, Bytes)>> {
		let mut puts = Vec::with_capacity(self.by_name.len() + self.by_version.len());
		for ((name, version), heartbeat_unix_ms) in &self.by_name {
			let key = PoolNameVersionKey::new(name.clone(), DescendingU64(u64::from(*version)));
			let value = key.serialize(at_pool_write_version(db, stamp(*heartbeat_unix_ms))?)?;
			puts.push((key.pack(), value));
		}
		for (version, heartbeat_unix_ms) in &self.by_version {
			let key = PoolVersionKey::new(DescendingU64(u64::from(*version)));
			let value = key.serialize(at_pool_write_version(db, stamp(*heartbeat_unix_ms))?)?;
			puts.push((key.pack(), value));
		}
		Ok(puts)
	}
}

fn stamp(heartbeat_unix_ms: i64) -> pool_wire::PoolVersionStamp {
	pool_wire::PoolVersionStamp {
		newest_heartbeat_unix_ms: heartbeat_unix_ms,
	}
}

/// Whether the stamp at `key`, per name or pool-wide, is missing or more than
/// [`VERSION_STAMP_REFRESH_MS`] older than `heartbeat_unix_ms`.
async fn stamp_is_stale<K>(
	snapshot: &mut Transaction,
	key: &K,
	heartbeat_unix_ms: i64,
) -> Result<bool>
where
	K: FormalKey<Packed = Bytes>,
	K::Codec: ValueCodec<Value = pool_wire::PoolVersionStamp>,
{
	let Some(raw) = snapshot.get(&key.pack()).await? else {
		return Ok(true);
	};
	let stored = key
		.deserialize(raw.into())
		.context("invalid pool version stamp")?
		.newest_heartbeat_unix_ms;
	Ok(stored < heartbeat_unix_ms.saturating_sub(VERSION_STAMP_REFRESH_MS))
}

/// Rewrites the stale stamps of `stamps`. The caller holds [`lock_pool`], so stamp writers never
/// interleave and a stamp only moves forward. Runs before the entries it stamps are written, so
/// a live envoy's version always carries stamps no more than [`VERSION_STAMP_REFRESH_MS`] older
/// than its heartbeat.
pub(crate) async fn refresh_version_stamps(db: &KvHandle, stamps: &VersionStamps) -> Result<()> {
	let mut snapshot = db.snapshot().await?;
	let stale = stamps.stale(&mut snapshot).await?;
	drop(snapshot);

	let puts = stale.encode(db)?;
	let mut offset = 0usize;
	let mut page_rows = STAMP_WRITE_PAGE;
	while offset < puts.len() {
		let end = offset.saturating_add(page_rows).min(puts.len());
		let result: Result<()> = async {
			let mut txn = db.begin().await?;
			for (key, value) in &puts[offset..end] {
				txn.put(key, value)?;
			}
			db.commit(txn).await?;
			Ok(())
		}
		.await;
		if let Err(error) = result {
			if shrink_index_page(&error, &mut page_rows) {
				continue;
			}
			return Err(error);
		}
		offset = end;
	}
	Ok(())
}

/// Reads the pool's record: `None` when the pool's configuration is not in this volume yet,
/// `Some(None)` once the pool was deleted.
pub(crate) async fn read_record(
	txn: &mut Transaction,
) -> Result<Option<Option<pool_wire::PoolConfig>>> {
	let key = PoolRecordKey::new();
	let Some(raw) = txn.get(&key.pack()).await? else {
		return Ok(None);
	};
	let record = key.deserialize(raw.into()).context("invalid pool record")?;
	Ok(Some(record.config))
}

/// Writes the pool's record.
pub(crate) fn put_record(
	db: &KvHandle,
	txn: &mut Transaction,
	config: Option<pool_wire::PoolConfig>,
) -> Result<()> {
	let key = PoolRecordKey::new();
	let encoded = key.serialize(at_pool_write_version(db, pool_wire::PoolRecord { config })?)?;
	txn.put(&key.pack(), &encoded)?;
	Ok(())
}

/// Reads an envoy's ring state, if it has one.
pub(crate) async fn read_state(
	txn: &mut Transaction,
	envoy_key: &str,
) -> Result<Option<pool_wire::PoolEnvoyState>> {
	let key = PoolEnvoyStateKey::new(envoy_key.to_owned());
	let Some(raw) = txn.get(&key.pack()).await? else {
		return Ok(None);
	};
	let state = key
		.deserialize(raw.into())
		.context("invalid pool envoy state")?;
	ensure!(
		&*state.envoy_key == envoy_key,
		"pool envoy state names another envoy than its key"
	);
	Ok(Some(state))
}

/// Deletes an envoy's state row and every ring and version entry its state says it wrote.
pub(crate) fn delete_rows(txn: &mut Transaction, state: &pool_wire::PoolEnvoyState) -> Result<()> {
	delete_entries(txn, state)?;
	txn.delete(&PoolEnvoyStateKey::new(state.envoy_key.to_string()).pack())?;
	Ok(())
}

/// Deletes the ring and version entries `state` says its connection wrote.
fn delete_entries(txn: &mut Transaction, state: &pool_wire::PoolEnvoyState) -> Result<()> {
	let version = DescendingU64(u64::from(state.version));
	for name in &state.names {
		for index in 0..state.virtual_nodes {
			let key = PoolEnvoyRingKey::new(
				name.to_string(),
				version,
				ring_position(&state.envoy_key, index),
				state.envoy_key.to_string(),
			);
			txn.delete(&key.pack())?;
		}
	}
	txn.delete(&PoolEnvoyVersionKey::new(version, state.envoy_key.to_string()).pack())?;
	Ok(())
}

/// Writes the ring and version entries of `state`.
fn put_entries(txn: &mut Transaction, state: &pool_wire::PoolEnvoyState) -> Result<()> {
	let version = DescendingU64(u64::from(state.version));
	for name in &state.names {
		for index in 0..state.virtual_nodes {
			let key = PoolEnvoyRingKey::new(
				name.to_string(),
				version,
				ring_position(&state.envoy_key, index),
				state.envoy_key.to_string(),
			);
			txn.put(&key.pack(), &[])?;
		}
	}
	txn.put(
		&PoolEnvoyVersionKey::new(version, state.envoy_key.to_string()).pack(),
		&[],
	)?;
	Ok(())
}

fn put_state(db: &KvHandle, txn: &mut Transaction, state: pool_wire::PoolEnvoyState) -> Result<()> {
	let key = PoolEnvoyStateKey::new(state.envoy_key.to_string());
	let encoded = key.serialize(at_pool_write_version(db, state)?)?;
	txn.put(&key.pack(), &encoded)?;
	Ok(())
}

/// What [`leave_ring`] found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RingLeave {
	/// The envoy had no ring rows.
	Absent,
	/// The envoy's ring rows were deleted.
	Removed,
	/// A newer connection than the caller's wrote the ring rows; they stay.
	Superseded,
}

/// Removes the ring rows of the envoy in `txn` unless a newer connection than
/// `connection_epoch` wrote them. Used by the legacy index's writes, so an envoy written there
/// leaves the ring.
pub(crate) async fn leave_ring(
	txn: &mut Transaction,
	envoy_key: &str,
	connection_epoch: u64,
) -> Result<RingLeave> {
	let Some(stored) = read_state(txn, envoy_key).await? else {
		return Ok(RingLeave::Absent);
	};
	if stored.connection_epoch > connection_epoch {
		return Ok(RingLeave::Superseded);
	}
	delete_rows(txn, &stored)?;
	Ok(RingLeave::Removed)
}

/// What one heartbeat did to its envoy's rows.
enum Applied {
	/// The state was written at this heartbeat.
	Written,
	/// The stored state is a newer heartbeat of the same connection; nothing changed.
	Reordered,
	/// A newer connection owns the envoy.
	Superseded,
	/// The heartbeat is already outside the eligibility window; nothing changed.
	Stale,
}

/// Applies one heartbeat: replaces the envoy's legacy rows, moves its ring and version entries
/// when its connection, version or names changed, and overwrites its state.
async fn apply_heartbeat(
	db: &KvHandle,
	txn: &mut Transaction,
	heartbeat: &pool_wire::PoolHeartbeat,
	oldest_eligible: i64,
) -> Result<Applied> {
	if heartbeat.heartbeat_unix_ms < oldest_eligible {
		return Ok(Applied::Stale);
	}
	let stored = read_state(txn, &heartbeat.envoy_key).await?;
	if let Some(stored) = &stored {
		if stored.connection_epoch > heartbeat.connection_epoch {
			return Ok(Applied::Superseded);
		}
		if stored.connection_epoch == heartbeat.connection_epoch
			&& stored.heartbeat_unix_ms > heartbeat.heartbeat_unix_ms
		{
			return Ok(Applied::Reordered);
		}
	}

	// A connection written by the legacy index leaves it here, unless that connection is newer.
	let legacy_key = ActiveEnvoyStateKey::new(heartbeat.envoy_key.to_string());
	if let Some(legacy) = read_active_envoy_state(txn, &legacy_key).await? {
		if legacy.connection_epoch > heartbeat.connection_epoch {
			return Ok(Applied::Superseded);
		}
		delete_active_envoy_entries(txn, &heartbeat.envoy_key, &legacy)?;
		txn.delete(&legacy_key.pack())?;
	}

	let state = pool_wire::PoolEnvoyState {
		envoy_key: heartbeat.envoy_key.clone(),
		connection_epoch: heartbeat.connection_epoch,
		worker_id: heartbeat.worker_id.clone(),
		connection_id: heartbeat.connection_id.clone(),
		version: heartbeat.version,
		names: heartbeat.names.clone(),
		virtual_nodes: stored
			.as_ref()
			.filter(|stored| stored.connection_epoch == heartbeat.connection_epoch)
			.map_or(RING_VIRTUAL_NODES, |stored| stored.virtual_nodes),
		heartbeat_unix_ms: heartbeat.heartbeat_unix_ms,
		actor_count: heartbeat.actor_count,
		stopping: heartbeat.stopping,
	};
	let moved = match &stored {
		Some(stored) => {
			stored.connection_epoch != state.connection_epoch
				|| stored.version != state.version
				|| stored.names != state.names
		}
		None => true,
	};
	if moved {
		if let Some(stored) = &stored {
			delete_entries(txn, stored)?;
		}
		put_entries(txn, &state)?;
	}
	put_state(db, txn, state)?;
	Ok(Applied::Written)
}

/// Live envoys per version, found while working out the pool's newest live version.
#[derive(Default)]
struct Liveness {
	/// Distinct live envoy keys per version.
	by_version: BTreeMap<u32, BTreeSet<String>>,
}

impl Liveness {
	fn add(&mut self, version: u32, envoy_key: &str) {
		self.by_version
			.entry(version)
			.or_default()
			.insert(envoy_key.to_owned());
	}

	/// Distinct envoys live at `version` or newer.
	fn live_at_or_above(&self, version: u32) -> usize {
		let mut keys = BTreeSet::new();
		for (_, envoys) in self.by_version.range(version..) {
			keys.extend(envoys.iter());
		}
		keys.len()
	}

	/// The newest version with a live envoy other than `envoy_key`.
	fn newest_other(&self, envoy_key: &str) -> Option<u32> {
		self.by_version
			.iter()
			.rev()
			.find(|(_, envoys)| envoys.iter().any(|key| key != envoy_key))
			.map(|(version, _)| *version)
	}
}

/// Walks the pool's version stamps newest first and records live envoys from both indexes,
/// until two distinct envoys are live at the versions walked so far, which answers
/// [`Liveness::newest_other`] for every envoy. Ring envoys in `skip` were just written and are
/// already recorded by the caller. Ring envoys past `lost_before` at a version the walk reads
/// are returned for removal; a version whose stamp is stale is never read, so envoys that
/// crashed there stay until a later envoy of that version heartbeats.
async fn walk_live_versions(
	txn: &mut Transaction,
	liveness: &mut Liveness,
	skip: &BTreeSet<String>,
	oldest_eligible: i64,
	lost_before: i64,
) -> Result<Vec<pool_wire::PoolEnvoyState>> {
	let mut lost = Vec::new();
	// A version whose stamp no live envoy could have left is passed over without reading its
	// entries, so the walk costs one stamp scan per page of dead versions.
	let skip_stamped_before = oldest_eligible.saturating_sub(VERSION_STAMP_REFRESH_MS);
	let catalog = PoolVersionPrefix::new().range();
	let mut lower = Bound::Included(catalog.start.clone());
	let mut stamps = Vec::new().into_iter();
	loop {
		let Some((raw, value)) = stamps.next() else {
			let page = txn
				.scan(
					(bound_slice(&lower), Bound::Excluded(catalog.end.as_slice())),
					Some(STAMP_SCAN_PAGE),
					false,
				)
				.await?;
			let Some((last, _)) = page.last() else {
				return Ok(lost);
			};
			lower = Bound::Excluded(last.clone());
			stamps = page.into_iter();
			continue;
		};
		let candidate = PoolVersionKey::unpack(&raw)?;
		let stamp = candidate
			.deserialize(value.into())
			.context("invalid pool version stamp")?;
		if stamp.newest_heartbeat_unix_ms < skip_stamped_before {
			continue;
		}
		let version = u32::try_from(candidate.version.0).unwrap_or(u32::MAX);

		// Ring envoys under the version, each judged by its state.
		let range = PoolEnvoyVersionNumberPrefix::new(candidate.version).range();
		let entries = txn
			.scan(
				(
					Bound::Included(range.start.as_slice()),
					Bound::Excluded(range.end.as_slice()),
				),
				Some(VERSION_MAX_SCAN),
				false,
			)
			.await?;
		let mut found = 0;
		for (raw, _) in entries {
			let entry = PoolEnvoyVersionKey::unpack(&raw)?;
			if skip.contains(&entry.envoy_key) {
				continue;
			}
			let Some(state) = read_state(txn, &entry.envoy_key).await? else {
				continue;
			};
			if state.version != version {
				continue;
			}
			if state.heartbeat_unix_ms >= oldest_eligible && !state.stopping {
				liveness.add(version, &entry.envoy_key);
				found += 1;
				if found == 2 {
					break;
				}
			} else if state.heartbeat_unix_ms < lost_before {
				lost.push(state);
			}
		}

		// Legacy envoys under the version, whose keys sit inside the live window.
		let window = ActiveEnvoyVersionNumberPrefix::new(candidate.version).range();
		let start =
			ActiveEnvoyVersionKey::new(candidate.version, oldest_eligible, String::new()).pack();
		let legacy = txn
			.scan(
				(
					Bound::Included(start.as_ref()),
					Bound::Excluded(window.end.as_slice()),
				),
				Some(2),
				false,
			)
			.await?;
		for (raw, _) in legacy {
			liveness.add(version, &ActiveEnvoyVersionKey::unpack(&raw)?.envoy_key);
		}

		if liveness.live_at_or_above(version) >= 2 {
			return Ok(lost);
		}
	}
}

pub(crate) async fn heartbeats_handler(
	clock: Arc<dyn Clock>,
	lost_threshold_ms: i64,
	db: KvHandle,
	request: pool_wire::PoolHeartbeatsRequest,
) -> Result<pool_wire::PoolHeartbeatsResponse> {
	let volume = ActiveEnvoyIndexVolume::from_volume_key(db.volume_key())?;
	ensure!(
		request.heartbeats.len() <= HEARTBEATS_PER_COMMAND,
		"too many heartbeats in one command"
	);
	let _lock = lock_pool(&db).await?;
	defer_queued_heartbeat_for_reconciliation(&db).await?;

	// A heartbeat is the envoy's admission, so it lands only in a pool that exists here. A pool
	// whose configuration is still in the namespace catalog is moved by the caller first.
	let read_started = clock.boot_ms()?;
	let mut txn = db.begin().await?;
	let record = read_record(&mut txn).await?;
	drop(txn);
	let Some(config) = record else {
		return Ok(pool_wire::PoolHeartbeatsResponse {
			replies: Vec::new(),
			unmigrated: true,
		});
	};
	if config.is_none() {
		return Ok(pool_wire::PoolHeartbeatsResponse {
			replies: request
				.heartbeats
				.iter()
				.map(|heartbeat| pool_wire::PoolHeartbeatReply {
					superseded: false,
					admission_lost: true,
					newest_live_version: heartbeat.version,
				})
				.collect(),
			unmigrated: false,
		});
	}

	// The legacy rows a heartbeat replaces must be in their reconciled layout first.
	let record_read_ms = clock.boot_ms()?.saturating_sub(read_started);
	let reconciliation_budget_ms = db.admitted_budget_ms().saturating_sub(record_read_ms);
	prepare_active_envoy_index_with_budget(
		&db,
		db.write_version(VersionFamily::PEGBOARD_ACTOR)?,
		reconciliation_budget_ms,
	)
	.await?;

	// Version catalogs are monotonic hints a pick walks, published before any entry under them.
	let mut names_by_version = BTreeMap::<u32, BTreeSet<String>>::new();
	for heartbeat in &request.heartbeats {
		names_by_version
			.entry(heartbeat.version)
			.or_default()
			.extend(heartbeat.names.iter().map(ToString::to_string));
	}
	for (version, names) in names_by_version {
		let names: Vec<vbare::Str> = names.into_iter().map(Into::into).collect();
		ensure_active_envoy_versions(&db, &names, DescendingU64(u64::from(version))).await?;
	}
	let mut stamps = VersionStamps::default();
	for heartbeat in &request.heartbeats {
		stamps.add(
			&heartbeat.names,
			heartbeat.version,
			heartbeat.heartbeat_unix_ms,
		);
	}
	refresh_version_stamps(&db, &stamps).await?;

	let now_unix_ms = clock.observe()?.unix_ms;
	let oldest_eligible = oldest_eligible_heartbeat(now_unix_ms);
	let lost_before = now_unix_ms.saturating_sub(lost_threshold_ms);
	let mut txn = db.begin().await?;
	let mut applied = Vec::with_capacity(request.heartbeats.len());
	for heartbeat in &request.heartbeats {
		applied.push(apply_heartbeat(&db, &mut txn, heartbeat, oldest_eligible).await?);
	}

	// The pool's live versions, with this batch's written heartbeats counted as live. Envoys
	// past the lost threshold that the walk passes are removed in the same transaction.
	// Envoys this batch wrote are judged by their new heartbeat; the walk skips their rows,
	// which this transaction replaced. A superseded or reordered heartbeat changed nothing, so
	// the walk judges its envoy by the stored state.
	let mut liveness = Liveness::default();
	let mut written = BTreeSet::new();
	for (heartbeat, applied) in request.heartbeats.iter().zip(&applied) {
		if !matches!(applied, Applied::Written) {
			continue;
		}
		written.insert(heartbeat.envoy_key.to_string());
		if !heartbeat.stopping {
			liveness.add(heartbeat.version, &heartbeat.envoy_key);
		}
	}
	let lost = walk_live_versions(
		&mut txn,
		&mut liveness,
		&written,
		oldest_eligible,
		lost_before,
	)
	.await?;
	for state in &lost {
		delete_rows(&mut txn, state)?;
	}

	db.commit(txn).await?;
	crate::metrics::ENVOY_INDEX_UPSERTS
		.with_label_values(&[&volume.namespace, "heartbeat"])
		.inc_by(request.heartbeats.len() as u64);
	if !lost.is_empty() {
		crate::metrics::ENVOY_INDEX_STALE_ROWS
			.with_label_values(&[&volume.namespace, "heartbeat_lost", "deleted"])
			.inc_by(lost.len() as u64);
	}

	let replies = request
		.heartbeats
		.iter()
		.zip(applied)
		.map(|(heartbeat, applied)| pool_wire::PoolHeartbeatReply {
			superseded: matches!(applied, Applied::Superseded),
			admission_lost: false,
			newest_live_version: liveness
				.newest_other(&heartbeat.envoy_key)
				.map_or(heartbeat.version, |newest| newest.max(heartbeat.version)),
		})
		.collect();
	Ok(pool_wire::PoolHeartbeatsResponse {
		replies,
		unmigrated: false,
	})
}

/// What a pick learned about one envoy it walked past.
enum Judged {
	Live(pool_wire::PoolCandidate),
	/// Out of the eligibility window, under another version than the entry, or already judged.
	/// An envoy past the lost threshold is also queued for removal after the pick.
	Skipped,
}

/// Reads and judges the envoy of a ring entry at `version`, once per envoy per pick.
async fn judge(
	snapshot: &mut Transaction,
	judged: &mut BTreeMap<String, bool>,
	lost: &mut Vec<pool_wire::PoolEnvoyState>,
	envoy_key: &str,
	version: u32,
	oldest_eligible: i64,
	lost_before: i64,
) -> Result<Judged> {
	if judged.contains_key(envoy_key) {
		return Ok(Judged::Skipped);
	}
	let state = read_state(snapshot, envoy_key).await?;
	let Some(state) = state.filter(|state| state.version == version) else {
		judged.insert(envoy_key.to_owned(), false);
		return Ok(Judged::Skipped);
	};
	if state.stopping {
		judged.insert(envoy_key.to_owned(), false);
		return Ok(Judged::Skipped);
	}
	if state.heartbeat_unix_ms >= oldest_eligible {
		judged.insert(envoy_key.to_owned(), true);
		return Ok(Judged::Live(pool_wire::PoolCandidate {
			envoy_key: state.envoy_key,
			worker_id: state.worker_id,
			connection_id: state.connection_id,
			actor_count: state.actor_count,
		}));
	}
	judged.insert(envoy_key.to_owned(), false);
	if state.heartbeat_unix_ms < lost_before {
		lost.push(state);
	}
	Ok(Judged::Skipped)
}

/// Walks the ring of `name` at `version` forward from `pivot`, wrapping once, to the first live
/// envoy not yet a candidate. Every entry walked counts against `budget`.
#[allow(clippy::too_many_arguments)]
async fn walk_from(
	snapshot: &mut Transaction,
	judged: &mut BTreeMap<String, bool>,
	lost: &mut Vec<pool_wire::PoolEnvoyState>,
	budget: &mut usize,
	name: &str,
	version: DescendingU64,
	pivot: [u8; 16],
	oldest_eligible: i64,
	lost_before: i64,
) -> Result<Option<pool_wire::PoolCandidate>> {
	let range = PoolEnvoyRingVersionPrefix::new(name.to_owned(), version).range();
	let pivot_key = PoolEnvoyRingKey::new(name.to_owned(), version, pivot, String::new())
		.pack()
		.to_vec();
	let version_number = u32::try_from(version.0).unwrap_or(u32::MAX);

	// The arc from the pivot to the end of the ring, then from its start back to the pivot.
	let halves = [
		(pivot_key.clone(), range.end.clone()),
		(range.start.clone(), pivot_key),
	];
	for (start, end) in halves {
		let mut lower = Bound::Included(start);
		loop {
			if *budget == 0 {
				return Ok(None);
			}
			let page = (*budget).min(WALK_PAGE);
			let entries = snapshot
				.scan(
					(bound_slice(&lower), Bound::Excluded(end.as_slice())),
					Some(page),
					false,
				)
				.await?;
			let Some((last, _)) = entries.last() else {
				break;
			};
			lower = Bound::Excluded(last.clone());
			for (raw, _) in &entries {
				*budget -= 1;
				let entry = PoolEnvoyRingKey::unpack(raw)?;
				let judgement = judge(
					snapshot,
					judged,
					lost,
					&entry.envoy_key,
					version_number,
					oldest_eligible,
					lost_before,
				)
				.await?;
				if let Judged::Live(candidate) = judgement {
					return Ok(Some(candidate));
				}
			}
			if entries.len() < page {
				break;
			}
		}
	}
	Ok(None)
}

/// Up to one live legacy envoy of `name` at `version` per pivot, each pivot choosing among the
/// first [`PICK_MAX_SCAN`] entries of the version's eligibility window. Legacy entries have no
/// actor count, so every candidate reports zero.
async fn legacy_candidates(
	snapshot: &mut Transaction,
	name: &str,
	version: DescendingU64,
	pivots: &[[u8; 16]],
	oldest_eligible: i64,
) -> Result<Vec<pool_wire::PoolCandidate>> {
	let range = ActiveEnvoyIndexVersionPrefix::new(name.to_owned(), version).range();
	let start = active_envoy_window_start(name, version, oldest_eligible);
	let entries = snapshot
		.scan(
			(
				Bound::Included(start.as_slice()),
				Bound::Excluded(range.end.as_slice()),
			),
			Some(PICK_MAX_SCAN),
			false,
		)
		.await?;
	if entries.is_empty() {
		return Ok(Vec::new());
	}

	let mut chosen = BTreeSet::new();
	let mut candidates = Vec::new();
	for pivot in pivots {
		let index = u128::from_be_bytes(*pivot) % entries.len() as u128;
		let (raw, value) = &entries[index as usize];
		let key = ActiveEnvoyIndexKey::unpack(raw).context("invalid active envoy index key")?;
		if !chosen.insert(key.envoy_key.clone()) {
			continue;
		}
		let entry = key
			.deserialize(value.clone().into())
			.context("invalid active envoy index entry")?;
		candidates.push(pool_wire::PoolCandidate {
			envoy_key: key.envoy_key.into(),
			worker_id: entry.worker_id,
			connection_id: entry.connection_id,
			actor_count: 0,
		});
	}
	Ok(candidates)
}

pub(crate) async fn pick_handler(
	clock: Arc<dyn Clock>,
	lost_threshold_ms: i64,
	db: KvHandle,
	request: pool_wire::PoolPickEnvoyRequest,
) -> Result<pool_wire::PoolPickEnvoyResponse> {
	let volume = ActiveEnvoyIndexVolume::from_volume_key(db.volume_key())?;
	let mut pivots = Vec::with_capacity(request.pivots.len());
	for pivot in &request.pivots {
		let pivot: [u8; 16] = pivot[..]
			.try_into()
			.context("a ring position must be 16 bytes")?;
		pivots.push(pivot);
	}
	ensure!(
		!pivots.is_empty() && pivots.len() <= PICK_SAMPLES * 4,
		"a pick takes between one and {} ring positions",
		PICK_SAMPLES * 4
	);

	// The legacy entries a pick may fall back to must be in their reconciled layout. After the
	// opening's first reconciliation this only takes the opening's lock.
	prepare_active_envoy_index(&db, db.write_version(VersionFamily::PEGBOARD_ACTOR)?).await?;
	let now_unix_ms = clock.observe()?.unix_ms;
	let oldest_eligible = oldest_eligible_heartbeat(now_unix_ms);
	let lost_before = now_unix_ms.saturating_sub(lost_threshold_ms);
	let mut snapshot = db.snapshot().await?;
	let mut judged = BTreeMap::new();
	let mut lost = Vec::new();
	let mut budget = PICK_MAX_SCAN;
	let mut candidates = Vec::new();

	// Versions newest first; the first with a live envoy, on the ring or else in the legacy index,
	// is the one served.
	// A version whose stamp no live envoy could have left is passed over without reading its
	// entries, so a pool with no live envoy costs one catalog scan per page of versions.
	let skip_stamped_before = oldest_eligible.saturating_sub(VERSION_STAMP_REFRESH_MS);
	let catalog = PoolNameVersionPrefix::new(request.name.to_string()).range();
	let mut lower = Bound::Included(catalog.start.clone());
	let mut versions = Vec::new().into_iter();
	while budget > 0 {
		let Some((raw, value)) = versions.next() else {
			let page = snapshot
				.scan(
					(bound_slice(&lower), Bound::Excluded(catalog.end.as_slice())),
					Some(STAMP_SCAN_PAGE),
					false,
				)
				.await?;
			let Some((last, _)) = page.last() else {
				break;
			};
			lower = Bound::Excluded(last.clone());
			versions = page.into_iter();
			continue;
		};
		let key = PoolNameVersionKey::unpack(&raw)?;
		let stamp = key
			.deserialize(value.into())
			.context("invalid pool version stamp")?;
		if stamp.newest_heartbeat_unix_ms < skip_stamped_before {
			continue;
		}
		let version = key.version;
		for pivot in &pivots {
			let candidate = walk_from(
				&mut snapshot,
				&mut judged,
				&mut lost,
				&mut budget,
				&request.name,
				version,
				*pivot,
				oldest_eligible,
				lost_before,
			)
			.await?;
			if let Some(candidate) = candidate {
				candidates.push(candidate);
			}
		}
		if candidates.is_empty() {
			candidates = legacy_candidates(
				&mut snapshot,
				&request.name,
				version,
				&pivots,
				oldest_eligible,
			)
			.await?;
			if !candidates.is_empty() {
				crate::metrics::ENVOY_PICKS_FROM_LEGACY_INDEX
					.with_label_values(&[&volume.namespace])
					.inc();
			}
		}
		if !candidates.is_empty() {
			break;
		}
	}
	drop(snapshot);

	// Envoys past the lost threshold leave the ring, unless they heartbeated since the read.
	if !lost.is_empty() {
		let mut txn = db.begin().await?;
		let mut removed = 0u64;
		for state in &lost {
			let current = read_state(&mut txn, &state.envoy_key).await?;
			if current.as_ref() == Some(state) {
				delete_rows(&mut txn, state)?;
				removed += 1;
			}
		}
		db.commit(txn).await?;
		crate::metrics::ENVOY_INDEX_STALE_ROWS
			.with_label_values(&[&volume.namespace, "heartbeat_lost", "deleted"])
			.inc_by(removed);
	}

	Ok(pool_wire::PoolPickEnvoyResponse { candidates })
}

/// Whether some envoy on the ring of `name` at `version` is live. Bounded like a pick; an
/// envoy hidden behind more lost entries than the bound reads as absent until a pick removes
/// them.
pub(crate) async fn ring_live_at(
	snapshot: &mut Transaction,
	name: &str,
	version: DescendingU64,
	oldest_eligible: i64,
) -> Result<bool> {
	let range = PoolEnvoyRingVersionPrefix::new(name.to_owned(), version).range();
	let entries = snapshot
		.scan(
			(
				Bound::Included(range.start.as_slice()),
				Bound::Excluded(range.end.as_slice()),
			),
			Some(PICK_MAX_SCAN),
			false,
		)
		.await?;
	let version = u32::try_from(version.0).unwrap_or(u32::MAX);
	let mut read = BTreeSet::new();
	for (raw, _) in entries {
		let entry = PoolEnvoyRingKey::unpack(&raw)?;
		if !read.insert(entry.envoy_key.clone()) {
			continue;
		}
		if read_state(snapshot, &entry.envoy_key)
			.await?
			.is_some_and(|state| {
				state.version == version
					&& state.heartbeat_unix_ms >= oldest_eligible
					&& !state.stopping
			}) {
			return Ok(true);
		}
	}
	Ok(false)
}

/// The live ring envoys at `version` serving `name`, one page in envoy key order after the
/// version entry `after`, and the entry the next page starts after while more may follow.
pub(crate) async fn list_ring_page(
	snapshot: &mut Transaction,
	name: &str,
	version: DescendingU64,
	after: Option<&[u8]>,
	limit: usize,
	oldest_eligible: i64,
) -> Result<(Vec<pool_wire::PoolEnvoyState>, Option<Vec<u8>>)> {
	let range = PoolEnvoyVersionNumberPrefix::new(version).range();
	let lower = after.map_or(Bound::Included(range.start.as_slice()), Bound::Excluded);
	let mut entries = snapshot
		.scan(
			(lower, Bound::Excluded(range.end.as_slice())),
			Some(limit + 1),
			false,
		)
		.await?;
	let has_more = entries.len() > limit;
	entries.truncate(limit);
	let cursor = if has_more {
		entries.last().map(|(key, _)| key.clone())
	} else {
		None
	};
	let version = u32::try_from(version.0).unwrap_or(u32::MAX);
	let mut envoys = Vec::new();
	for (raw, _) in entries {
		let entry = PoolEnvoyVersionKey::unpack(&raw)?;
		let Some(state) = read_state(snapshot, &entry.envoy_key).await? else {
			continue;
		};
		if state.version == version
			&& state.heartbeat_unix_ms >= oldest_eligible
			&& !state.stopping
			&& state.names.iter().any(|served| served == name)
		{
			envoys.push(state);
		}
	}
	Ok((envoys, cursor))
}

/// Up to `limit` ring envoy states in envoy key order strictly after `after`.
pub(crate) async fn list_states_after(
	snapshot: &mut Transaction,
	after: Option<&str>,
	limit: usize,
) -> Result<Vec<pool_wire::PoolEnvoyState>> {
	let range = PoolEnvoyStatePrefix::new().range();
	let after = after.map(|envoy_key| PoolEnvoyStateKey::new(envoy_key.to_owned()).pack());
	let lower = after
		.as_ref()
		.map_or(Bound::Included(range.start.as_slice()), |key| {
			Bound::Excluded(key.as_ref())
		});
	let entries = snapshot
		.scan(
			(lower, Bound::Excluded(range.end.as_slice())),
			Some(limit),
			false,
		)
		.await?;
	let mut states = Vec::with_capacity(entries.len());
	for (raw, value) in entries {
		let key = PoolEnvoyStateKey::unpack(&raw)?;
		let state = key
			.deserialize(value.into())
			.context("invalid pool envoy state")?;
		ensure!(
			&*state.envoy_key == key.envoy_key.as_str(),
			"pool envoy state names another envoy than its key"
		);
		states.push(state);
	}
	Ok(states)
}

/// Writes heartbeats for envoys of one pool, in commands of at most
/// [`HEARTBEATS_PER_COMMAND`] sent one after another, and returns one reply per heartbeat in
/// order. A pool whose configuration is still only in its namespace's catalog adopts it first.
pub async fn send_heartbeats(
	ctx: &EngineContext,
	namespace: &str,
	pool_name: &str,
	heartbeats: Vec<pool_wire::PoolHeartbeat>,
) -> Result<Vec<pool_wire::PoolHeartbeatReply>> {
	let volume =
		ActiveEnvoyIndexVolume::new(namespace.to_owned(), pool_name.to_owned()).volume_key();
	let mut replies = Vec::with_capacity(heartbeats.len());
	let mut heartbeats = heartbeats.into_iter().peekable();
	while heartbeats.peek().is_some() {
		let chunk: Vec<_> = heartbeats.by_ref().take(HEARTBEATS_PER_COMMAND).collect();
		let sent = chunk.len();
		let request = pool_wire::PoolHeartbeatsRequest { heartbeats: chunk };
		let mut response = ctx
			.depot()
			.command::<PoolHeartbeats>(&volume, request.clone())
			.await?;
		if response.unmigrated {
			crate::pool_admin::legacy_adopt_catalog_pool(ctx, namespace, pool_name).await?;
			response = ctx
				.depot()
				.command::<PoolHeartbeats>(&volume, request)
				.await?;
			ensure!(
				!response.unmigrated,
				"pool volume has no record after adopting the catalog's"
			);
		}
		ensure!(
			response.replies.len() == sent,
			"pool heartbeats replied {} times for {sent} heartbeats",
			response.replies.len()
		);
		replies.extend(response.replies);
	}
	Ok(replies)
}

/// Extra load, drawn uniformly below this, added to each candidate's actor count before the
/// comparison. Allocators reading the same counts a heartbeat old would otherwise all prefer
/// the same envoy of a pair; a little noise spreads a burst between them.
const COUNT_JITTER: u32 = 4;

/// Picks an envoy for an actor of `name` in the pool by power of two choices: the candidate with
/// the fewest actors, after jitter, of [`PICK_SAMPLES`] random ring positions, ties broken at
/// random. `None` when no live ring envoy serves the name.
pub async fn pick_envoy(
	ctx: &EngineContext,
	namespace: &str,
	pool_name: &str,
	name: &str,
) -> Result<Option<crate::wire::ActorAllocation>> {
	let mut rng = ctx.env().rng("pegboard.envoy_pick");
	let pivots = (0..PICK_SAMPLES)
		.map(|_| rng.gen_range(0..u128::MAX).to_be_bytes().to_vec().into())
		.collect();
	let volume =
		ActiveEnvoyIndexVolume::new(namespace.to_owned(), pool_name.to_owned()).volume_key();
	let response = ctx
		.depot()
		.command::<PoolPickEnvoy>(
			&volume,
			pool_wire::PoolPickEnvoyRequest {
				name: name.to_owned().into(),
				pivots,
			},
		)
		.await;
	let response = match response {
		Ok(response) => response,
		Err(error) if crate::index::is_volume_not_found(&error) => return Ok(None),
		Err(error) => return Err(error),
	};
	let chosen = choose_least_loaded(response.candidates, |bound| rng.gen_range(0..bound));
	Ok(chosen.map(|candidate| crate::wire::ActorAllocation {
		envoy_key: candidate.envoy_key,
		worker_id: candidate.worker_id,
		connection_id: candidate.connection_id,
	}))
}

/// The candidate with the lowest actor count after adding `below(COUNT_JITTER)` to each, ties
/// broken by `below(ties)`. `below(n)` must return a value in `0..n`.
pub fn choose_least_loaded(
	candidates: Vec<pool_wire::PoolCandidate>,
	mut below: impl FnMut(u32) -> u32,
) -> Option<pool_wire::PoolCandidate> {
	let mut scored: Vec<_> = candidates
		.into_iter()
		.map(|candidate| {
			let score = u64::from(candidate.actor_count) + u64::from(below(COUNT_JITTER));
			(score, candidate)
		})
		.collect();
	let lowest = scored.iter().map(|(score, _)| *score).min()?;
	scored.retain(|(score, _)| *score == lowest);
	let ties = u32::try_from(scored.len()).unwrap_or(u32::MAX);
	let index = usize::try_from(below(ties))
		.unwrap_or(0)
		.min(scored.len() - 1);
	Some(scored.swap_remove(index).1)
}

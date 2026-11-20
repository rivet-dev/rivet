//! A node pool's configuration and admission in the pool's own volume, the same volume as its
//! active envoy index (`[ACTIVE_ENVOY_INDEX, namespace, pool_name]`).
//!
//! Once every storage worker reads `pegboard.pool`, the pool's [`PoolRecordKey`] row is the
//! authority for its configuration, and an envoy's ring state row is its admission: the batched
//! heartbeat that keeps an envoy allocatable also keeps it admitted, so a connected node costs one
//! write per flush in its pool's volume and nothing in its namespace's catalog. Deleting a pool
//! checks for admitted nodes and writes a deleted record under the same lock heartbeats take, so a
//! delete and an admitting heartbeat cannot interleave; every heartbeat after it replies that
//! admission was lost.
//!
//! The namespace catalog keeps a copy of each configuration so its listing stays one scan. The
//! pool volume wins wherever they differ: a listing replaces each copy with the pool's own record
//! and drops pools whose record is deleted.
//!
//! Pools configured before this kept their configuration only in the catalog. Their volume has no
//! record, and the first operation that needs it reads the catalog and adopts that configuration
//! into the volume ([`legacy_adopt_catalog_pool`]). A pool nobody touches keeps its configuration
//! only in the catalog for good, so the adoption path stays.
//!
//! [`PoolRecordKey`]: crate::keys::kv::PoolRecordKey
use crate::envoy_ring::{lock_pool, put_record, read_record};
use crate::index::{is_volume_not_found, oldest_eligible_heartbeat};
use crate::keys::kv::{ActiveEnvoyStateKey, ActiveEnvoyStatePrefix, PoolEnvoyStatePrefix};
use crate::keys::partition::ActiveEnvoyIndexVolume;
use crate::{pool_wire, wire};
use anyhow::{Context, Result, bail};
use depot_protocol::command::{Command, CommandTag};
use depot_worker::command::{CommandRegistry, KvHandle, Transaction};
use rivet_engine_context::EngineContext;
use rivet_env::Clock;
use rivet_keys::keys::{
	KeyAddress,
	codec::{FormalKey, SubspaceRange},
	tags::depot_command::PEGBOARD_POOL_ADMIN,
};
use std::ops::Bound;
use std::sync::Arc;

/// Reads, writes, deletes or adopts the pool's record in its own volume.
pub struct PoolAdmin;

impl Command for PoolAdmin {
	const TAG: CommandTag = CommandTag(PEGBOARD_POOL_ADMIN);
	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::PoolAdminRequest;
	type Response = rivet_data::versioned::PoolAdminResponse;
}

/// States one admitted-node check reads per scan.
const ADMITTED_PAGE: usize = 100;

pub fn register(registry: &mut CommandRegistry, ctx: &EngineContext) -> Result<()> {
	let concurrency = ctx.config().pegboard().command_concurrency.index();
	let clock = ctx.env().clock();
	registry
		.register_with_concurrency::<PoolAdmin>(concurrency, move |db: KvHandle, request| {
			admin_handler(clock.clone(), db, request)
		})?;
	Ok(())
}

/// The reply that describes `record`.
fn describe(record: Option<Option<pool_wire::PoolConfig>>) -> pool_wire::PoolAdminResponse {
	match record {
		None => pool_wire::PoolAdminResponse {
			status: pool_wire::PoolAdminStatus::Unmigrated,
			config: None,
		},
		Some(None) => pool_wire::PoolAdminResponse {
			status: pool_wire::PoolAdminStatus::NotFound,
			config: None,
		},
		Some(Some(config)) => pool_wire::PoolAdminResponse {
			status: pool_wire::PoolAdminStatus::Ok,
			config: Some(config),
		},
	}
}

/// Whether some node of the pool is admitted now: a ring or legacy envoy state whose heartbeat is
/// inside the eligibility window. A stopping node is still admitted.
async fn any_admitted(txn: &mut Transaction, oldest_eligible: i64) -> Result<bool> {
	let ring = PoolEnvoyStatePrefix::new().range();
	let mut lower = Bound::Included(ring.start.clone());
	loop {
		let rows = txn
			.scan(
				(as_slice(&lower), Bound::Excluded(ring.end.as_slice())),
				Some(ADMITTED_PAGE),
				false,
			)
			.await?;
		let Some((last, _)) = rows.last() else {
			break;
		};
		lower = Bound::Excluded(last.clone());
		for (raw, value) in &rows {
			let key = crate::keys::kv::PoolEnvoyStateKey::unpack(raw)?;
			let state = key
				.deserialize(value.clone().into())
				.context("invalid pool envoy state")?;
			if state.heartbeat_unix_ms >= oldest_eligible {
				return Ok(true);
			}
		}
		if rows.len() < ADMITTED_PAGE {
			break;
		}
	}

	let legacy = ActiveEnvoyStatePrefix::new().range();
	let mut lower = Bound::Included(legacy.start.clone());
	loop {
		let rows = txn
			.scan(
				(as_slice(&lower), Bound::Excluded(legacy.end.as_slice())),
				Some(ADMITTED_PAGE),
				false,
			)
			.await?;
		let Some((last, _)) = rows.last() else {
			break;
		};
		lower = Bound::Excluded(last.clone());
		for (raw, value) in &rows {
			let key = ActiveEnvoyStateKey::unpack(raw)?;
			let state = key
				.deserialize(value.clone().into())
				.context("invalid active envoy state")?;
			if state.heartbeat_unix_ms >= oldest_eligible {
				return Ok(true);
			}
		}
		if rows.len() < ADMITTED_PAGE {
			break;
		}
	}
	Ok(false)
}

fn as_slice(bound: &Bound<Vec<u8>>) -> Bound<&[u8]> {
	match bound {
		Bound::Included(key) => Bound::Included(key.as_slice()),
		Bound::Excluded(key) => Bound::Excluded(key.as_slice()),
		Bound::Unbounded => Bound::Unbounded,
	}
}

async fn admin_handler(
	clock: Arc<dyn Clock>,
	db: KvHandle,
	request: pool_wire::PoolAdminRequest,
) -> Result<pool_wire::PoolAdminResponse> {
	ActiveEnvoyIndexVolume::from_volume_key(db.volume_key())?;
	let _lock = lock_pool(&db).await?;
	let mut txn = db.begin().await?;
	let record = read_record(&mut txn).await?;

	match request.operation {
		pool_wire::PoolAdminOperation::PoolAdminRead => Ok(describe(record)),
		pool_wire::PoolAdminOperation::PoolAdminPut(config) => {
			put_record(&db, &mut txn, Some(config.clone()))?;
			db.commit(txn).await?;
			Ok(describe(Some(Some(config))))
		}
		pool_wire::PoolAdminOperation::PoolAdminDelete => {
			let Some(Some(config)) = record else {
				return Ok(describe(record));
			};
			let oldest_eligible = oldest_eligible_heartbeat(clock.observe()?.unix_ms);
			if any_admitted(&mut txn, oldest_eligible).await? {
				return Ok(pool_wire::PoolAdminResponse {
					status: pool_wire::PoolAdminStatus::Connected,
					config: Some(config),
				});
			}
			// The envoy rows left behind all aged out of the window, so allocation skips them
			// and later picks remove them.
			put_record(&db, &mut txn, None)?;
			db.commit(txn).await?;
			Ok(pool_wire::PoolAdminResponse {
				status: pool_wire::PoolAdminStatus::Ok,
				config: None,
			})
		}
		pool_wire::PoolAdminOperation::LegacyPoolAdminAdopt(adopt) => {
			if record.is_some() {
				return Ok(describe(record));
			}
			put_record(&db, &mut txn, adopt.config.clone())?;
			db.commit(txn).await?;
			Ok(describe(Some(adopt.config)))
		}
	}
}

/// The configuration as the pool volume stores it.
pub fn config_to_pool(config: wire::NodePoolConfig) -> pool_wire::PoolConfig {
	pool_wire::PoolConfig {
		drain_on_version_upgrade: config.drain_on_version_upgrade,
		actor_eviction_delay: config.actor_eviction_delay,
		actor_eviction_period: config.actor_eviction_period,
		actor_eviction_rate: config.actor_eviction_rate,
		metadata: config.metadata.map(|metadata| metadata.to_vec().into()),
	}
}

/// The configuration as the catalog and the rest of pegboard read it.
pub fn config_from_pool(config: pool_wire::PoolConfig) -> wire::NodePoolConfig {
	wire::NodePoolConfig {
		drain_on_version_upgrade: config.drain_on_version_upgrade,
		actor_eviction_delay: config.actor_eviction_delay,
		actor_eviction_period: config.actor_eviction_period,
		actor_eviction_rate: config.actor_eviction_rate,
		metadata: config.metadata.map(|metadata| metadata.to_vec().into()),
	}
}

fn pool_volume(namespace: &str, pool_name: &str) -> rivet_keys::keys::VolumeKey {
	ActiveEnvoyIndexVolume::new(namespace.to_owned(), pool_name.to_owned()).volume_key()
}

/// Sends one admin operation to the pool's volume, as is: a pool whose configuration is still in
/// the catalog answers `Unmigrated`.
async fn send(
	ctx: &EngineContext,
	namespace: &str,
	pool_name: &str,
	operation: pool_wire::PoolAdminOperation,
) -> Result<pool_wire::PoolAdminResponse> {
	ctx.depot()
		.command::<PoolAdmin>(
			&pool_volume(namespace, pool_name),
			pool_wire::PoolAdminRequest { operation },
		)
		.await
}

/// Sends one admin operation, first adopting the pool's configuration from the catalog when its
/// volume has none yet.
async fn admin(
	ctx: &EngineContext,
	namespace: &str,
	pool_name: &str,
	operation: pool_wire::PoolAdminOperation,
) -> Result<pool_wire::PoolAdminResponse> {
	let response = send(ctx, namespace, pool_name, operation.clone()).await?;
	if response.status != pool_wire::PoolAdminStatus::Unmigrated {
		return Ok(response);
	}
	legacy_adopt_catalog_pool(ctx, namespace, pool_name).await?;
	let response = send(ctx, namespace, pool_name, operation).await?;
	if response.status == pool_wire::PoolAdminStatus::Unmigrated {
		bail!("pool volume has no record after adopting the catalog's");
	}
	Ok(response)
}

/// Legacy: copies a pool's configuration from its namespace's catalog into its own volume, or
/// records it deleted when the catalog has none. Pools configured before pools kept their
/// configuration in their own volume have it only in the catalog; new pools write it to their
/// volume first. Taken when an operation finds the pool's volume without a record. The adoption is
/// conditional on the volume still having none, so a concurrent put or delete wins.
pub async fn legacy_adopt_catalog_pool(
	ctx: &EngineContext,
	namespace: &str,
	pool_name: &str,
) -> Result<()> {
	let catalog = crate::catalog::catalog_pool_command(
		ctx,
		namespace,
		pool_name,
		wire::PoolOperation::PoolRead,
	)
	.await?;
	let config = catalog.config.map(config_to_pool);
	tracing::info!(
		%namespace,
		%pool_name,
		configured = config.is_some(),
		"moving a pool's configuration from its namespace catalog into its own volume"
	);
	send(
		ctx,
		namespace,
		pool_name,
		pool_wire::PoolAdminOperation::LegacyPoolAdminAdopt(pool_wire::LegacyPoolAdminAdopt {
			config,
		}),
	)
	.await?;
	Ok(())
}

/// Runs a pool operation against the pool's own volume, the path once every storage worker reads
/// `pegboard.pool`. Connects and renewals only check that the pool exists: the envoy's ring state
/// is its admission and its batched heartbeat renews it.
pub(crate) async fn pool(
	ctx: &EngineContext,
	namespace: &str,
	pool_name: &str,
	operation: wire::PoolOperation,
) -> Result<wire::PoolResponse> {
	let read = || {
		admin(
			ctx,
			namespace,
			pool_name,
			pool_wire::PoolAdminOperation::PoolAdminRead,
		)
	};
	let response = match operation {
		wire::PoolOperation::PoolRead
		| wire::PoolOperation::PoolConnect(_)
		| wire::PoolOperation::PoolRenew(_) => read().await?,
		wire::PoolOperation::PoolPut(config) => {
			let response = admin(
				ctx,
				namespace,
				pool_name,
				pool_wire::PoolAdminOperation::PoolAdminPut(config_to_pool(config.clone())),
			)
			.await?;
			// The catalog's copy keeps the namespace's pool listing one scan.
			crate::catalog::catalog_pool_command(
				ctx,
				namespace,
				pool_name,
				wire::PoolOperation::PoolPut(config),
			)
			.await?;
			response
		}
		wire::PoolOperation::PoolDelete => {
			let response = admin(
				ctx,
				namespace,
				pool_name,
				pool_wire::PoolAdminOperation::PoolAdminDelete,
			)
			.await?;
			if response.status == pool_wire::PoolAdminStatus::Ok {
				// The pool volume's deleted record is the authority; the catalog copy is only for
				// listing, which drops pools whose record is deleted.
				if let Err(error) = crate::catalog::catalog_pool_command(
					ctx,
					namespace,
					pool_name,
					wire::PoolOperation::PoolDelete,
				)
				.await
				{
					tracing::warn!(
						?error,
						%namespace,
						%pool_name,
						"removing a deleted pool's catalog copy failed; listings skip it by its own record"
					);
				}
			}
			response
		}
		wire::PoolOperation::PoolDisconnect(connection_id) => {
			// A connection admitted before the fleet read the pool volume holds a catalog lease.
			return crate::catalog::catalog_pool_command(
				ctx,
				namespace,
				pool_name,
				wire::PoolOperation::PoolDisconnect(connection_id),
			)
			.await;
		}
	};
	Ok(wire::PoolResponse {
		status: match response.status {
			pool_wire::PoolAdminStatus::Ok => wire::PoolStatus::Ok,
			pool_wire::PoolAdminStatus::NotFound => wire::PoolStatus::NotFound,
			pool_wire::PoolAdminStatus::Connected => wire::PoolStatus::Connected,
			pool_wire::PoolAdminStatus::Unmigrated => {
				bail!("pool admin answered unmigrated after adoption")
			}
		},
		config: response.config.map(config_from_pool),
	})
}

/// Replaces each catalog copy in a listing page with the pool's own record: drops a pool whose
/// record is deleted and keeps the catalog's copy for a pool not adopted yet.
pub(crate) async fn reconcile_listing(
	ctx: &EngineContext,
	namespace: &str,
	pools: Vec<wire::PoolEntry>,
) -> Result<Vec<wire::PoolEntry>> {
	let reads = pools.into_iter().map(|entry| async move {
		let response = send(
			ctx,
			namespace,
			&entry.name,
			pool_wire::PoolAdminOperation::PoolAdminRead,
		)
		.await;
		let response = match response {
			Ok(response) => response,
			Err(error) if is_volume_not_found(&error) => return Ok(Some(entry)),
			Err(error) => return Err(error),
		};
		match response.status {
			pool_wire::PoolAdminStatus::Ok => Ok(response.config.map(|config| wire::PoolEntry {
				name: entry.name,
				config: config_from_pool(config),
			})),
			pool_wire::PoolAdminStatus::NotFound => Ok(None),
			pool_wire::PoolAdminStatus::Unmigrated => Ok(Some(entry)),
			pool_wire::PoolAdminStatus::Connected => {
				bail!("pool admin read answered connected")
			}
		}
	});
	let entries = futures_util::future::try_join_all(reads).await?;
	Ok(entries.into_iter().flatten().collect())
}

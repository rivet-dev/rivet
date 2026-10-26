//! Cluster namespace catalog, with immutable slug identities and positive existence caching.
use crate::{
	keys::{
		kv::{
			EnvoyPoolKey, EnvoyPoolPrefix, NamespaceKey, NamespacePrefix, PoolConnectionKey,
			PoolConnectionPrefix, PoolHeartbeatKey, PoolHeartbeatPrefix,
		},
		partition::{EnvoyPoolIndexVolume, NamespaceCatalogVolume},
	},
	wire,
};
use anyhow::{Context, Result, ensure};
use depot_protocol::command::{Command, CommandTag};
use depot_worker::command::{CommandRegistry, KvHandle};
use rivet_engine_context::EngineContext;
use rivet_env::Clock;
use rivet_keys::keys::{KeyAddress, SubspaceRange, codec::FormalKey, tags::depot_command::*};
use std::{ops::Bound, sync::Arc};

#[derive(Default)]
struct CatalogLock(tokio::sync::Mutex<()>);

pub struct NamespaceCreate;
impl Command for NamespaceCreate {
	const TAG: CommandTag = CommandTag(PEGBOARD_NAMESPACE_CREATE);
	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::NamespaceCreateRequest;
	type Response = rivet_data::versioned::NamespaceCreateResponse;
}
pub struct NamespaceRead;
impl Command for NamespaceRead {
	const TAG: CommandTag = CommandTag(PEGBOARD_NAMESPACE_READ);
	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::NamespaceReadRequest;
	type Response = rivet_data::versioned::NamespaceReadResponse;
}
pub struct NamespaceList;
impl Command for NamespaceList {
	const TAG: CommandTag = CommandTag(PEGBOARD_NAMESPACE_LIST);
	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::NamespaceListRequest;
	type Response = rivet_data::versioned::NamespaceListResponse;
}
pub struct Pool;
impl Command for Pool {
	const TAG: CommandTag = CommandTag(PEGBOARD_POOL);
	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::PoolRequest;
	type Response = rivet_data::versioned::PoolResponse;
}

pub struct PoolList;
impl Command for PoolList {
	const TAG: CommandTag = CommandTag(PEGBOARD_POOL_LIST);
	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::PoolListRequest;
	type Response = rivet_data::versioned::PoolListResponse;
}
pub(crate) fn register(registry: &mut CommandRegistry, ctx: &EngineContext) -> Result<()> {
	// Commands share the volume-opening lock so concurrent creates cannot claim one name.
	let clock = ctx.env().clock();
	registry.register_with_concurrency::<NamespaceCreate>(1, move |db, request| {
		create_handler(clock.clone(), db, request)
	})?;
	let clock = ctx.env().clock();
	registry.register_with_concurrency::<NamespaceRead>(1, move |db, request| {
		read_handler(clock.clone(), db, request)
	})?;
	let clock = ctx.env().clock();
	registry.register_with_concurrency::<NamespaceList>(1, move |db, request| {
		list_handler(clock.clone(), db, request)
	})?;
	let clock = ctx.env().clock();
	registry.register_with_concurrency::<Pool>(1, move |db, request| {
		pool_handler(clock.clone(), db, request)
	})?;
	registry.register_with_concurrency::<PoolList>(1, pool_list_handler)?;
	Ok(())
}

/// The catalog initializes only the default namespace, never names found in existing data.
async fn bootstrap(clock: &dyn Clock, db: &KvHandle) -> Result<()> {
	NamespaceCatalogVolume::from_volume_key(db.volume_key())?;
	let key = NamespaceKey::new(crate::DEFAULT_NAMESPACE.to_owned());
	let mut txn = db.begin().await?;
	if txn.get(&key.pack()).await?.is_none() {
		txn.put(
			&key.pack(),
			&key.serialize(wire::NamespaceRecord {
				create_unix_ms: clock.observe()?.unix_ms,
				create_operation_id: vec![0; 16].into(),
			})?,
		)?;
		db.commit(txn).await?;
	}
	Ok(())
}

async fn create_handler(
	clock: Arc<dyn Clock>,
	db: KvHandle,
	request: wire::NamespaceCreateRequest,
) -> Result<wire::NamespaceCreateResponse> {
	let serial = db.opening_state::<CatalogLock>().await?;
	let _serial = serial.0.lock().await;
	crate::validation::namespace_name(&request.name)?;
	bootstrap(clock.as_ref(), &db).await?;
	let key = NamespaceKey::new(request.name.to_string());
	let mut txn = db.begin().await?;
	if let Some(stored) = txn.get(&key.pack()).await? {
		let record = key.deserialize(stored.into())?;
		return Ok(wire::NamespaceCreateResponse {
			created: record.create_operation_id.as_ref() == db.operation_id(),
			record,
		});
	}
	let record = wire::NamespaceRecord {
		create_unix_ms: request.create_unix_ms,
		create_operation_id: db.operation_id().to_vec().into(),
	};
	txn.put(&key.pack(), &key.serialize(record.clone())?)?;
	db.commit(txn).await?;
	Ok(wire::NamespaceCreateResponse {
		created: true,
		record,
	})
}

async fn read_handler(
	clock: Arc<dyn Clock>,
	db: KvHandle,
	request: wire::NamespaceReadRequest,
) -> Result<wire::NamespaceReadResponse> {
	let serial = db.opening_state::<CatalogLock>().await?;
	let _serial = serial.0.lock().await;
	bootstrap(clock.as_ref(), &db).await?;
	let key = NamespaceKey::new(request.name.to_string());
	let value = db.snapshot().await?.get(&key.pack()).await?;
	Ok(wire::NamespaceReadResponse {
		record: value
			.map(|value| key.deserialize(value.into()))
			.transpose()?,
	})
}

async fn list_handler(
	clock: Arc<dyn Clock>,
	db: KvHandle,
	request: wire::NamespaceListRequest,
) -> Result<wire::NamespaceListResponse> {
	let serial = db.opening_state::<CatalogLock>().await?;
	let _serial = serial.0.lock().await;
	bootstrap(clock.as_ref(), &db).await?;
	ensure!(request.limit > 0, "namespace list limit must be positive");
	let range = NamespacePrefix::new().range();
	let start = request
		.after
		.map(|name| Bound::Excluded(NamespaceKey::new(name.to_string()).pack().to_vec()))
		.unwrap_or(Bound::Included(range.start));
	let limit = (request.limit as usize).min(100);
	let entries = db
		.snapshot()
		.await?
		.scan(
			(
				start.as_ref().map(Vec::as_slice),
				Bound::Excluded(range.end.as_slice()),
			),
			Some(limit + 1),
			false,
		)
		.await?;
	let has_more = entries.len() > limit;
	let mut namespaces = Vec::new();
	for (key, value) in entries.into_iter().take(limit) {
		let key = NamespaceKey::unpack(&key)?;
		namespaces.push(wire::NamespaceEntry {
			name: key.name.clone().into(),
			record: key.deserialize(value.into())?,
		});
	}
	let cursor = if has_more {
		namespaces.last().map(|entry| entry.name.clone())
	} else {
		None
	};
	Ok(wire::NamespaceListResponse { namespaces, cursor })
}

async fn pool_handler(
	clock: Arc<dyn Clock>,
	db: KvHandle,
	request: wire::PoolRequest,
) -> Result<wire::PoolResponse> {
	let serial = db.opening_state::<CatalogLock>().await?;
	let _serial = serial.0.lock().await;
	EnvoyPoolIndexVolume::from_volume_key(db.volume_key())?;
	crate::validation::pool_name(&request.pool_name)?;
	let name = request.pool_name.to_string();
	let key = EnvoyPoolKey::new(name.clone());
	let mut txn = db.begin().await?;
	let mut config = txn
		.get(&key.pack())
		.await?
		.map(|value| key.deserialize(value.into()))
		.transpose()?;
	let mut status = if config.is_some() {
		wire::PoolStatus::Ok
	} else {
		wire::PoolStatus::NotFound
	};
	let oldest = clock
		.observe()?
		.unix_ms
		.saturating_sub(crate::ENVOY_HEARTBEAT_TIMEOUT_MS);
	match request.operation {
		wire::PoolOperation::PoolPut(value) => {
			txn.put(&key.pack(), &key.serialize(value.clone())?)?;
			config = Some(value);
			status = wire::PoolStatus::Ok;
		}
		wire::PoolOperation::PoolRead => {}
		wire::PoolOperation::PoolDelete => {
			// Newest lease first: one row proves whether any connected node remains.
			let range = PoolHeartbeatPrefix::new(name.clone()).range();
			let newest = txn
				.scan(
					(
						Bound::Included(range.start.as_slice()),
						Bound::Excluded(range.end.as_slice()),
					),
					Some(1),
					true,
				)
				.await?;
			let connected = newest
				.first()
				.map(|(raw, _)| PoolHeartbeatKey::unpack(raw))
				.transpose()?
				.is_some_and(|lease| lease.heartbeat_unix_ms > oldest);
			if connected {
				status = wire::PoolStatus::Connected;
			} else {
				txn.delete(&key.pack())?;
				txn.delete_range((
					Bound::Included(range.start.as_slice()),
					Bound::Excluded(range.end.as_slice()),
				))?;
				let range = PoolConnectionPrefix::new(name.clone()).range();
				txn.delete_range((
					Bound::Included(range.start.as_slice()),
					Bound::Excluded(range.end.as_slice()),
				))?;
			}
		}
		operation @ (wire::PoolOperation::PoolConnect(_) | wire::PoolOperation::PoolRenew(_)) => {
			let renew = matches!(operation, wire::PoolOperation::PoolRenew(_));
			let connection = match operation {
				wire::PoolOperation::PoolConnect(connection)
				| wire::PoolOperation::PoolRenew(connection) => connection,
				_ => unreachable!(),
			};
			let id = uuid::Uuid::from_slice(&connection.connection_id)
				.context("invalid connection id")?;
			let lease_key = PoolConnectionKey::new(name.clone(), id);
			let previous = txn
				.get(&lease_key.pack())
				.await?
				.map(|raw| lease_key.deserialize(raw.into()))
				.transpose()?;
			// A renewal cannot grant a new admission after deletion, disconnect or expiry.
			let valid = connection.heartbeat_unix_ms > oldest
				&& (!renew
					|| previous
						.as_ref()
						.is_some_and(|lease| lease.heartbeat_unix_ms > oldest));
			if config.is_some() && valid {
				let heartbeat_unix_ms = previous
					.as_ref()
					.map_or(connection.heartbeat_unix_ms, |lease| {
						lease.heartbeat_unix_ms.max(connection.heartbeat_unix_ms)
					});
				if let Some(previous) = previous {
					txn.delete(
						&PoolHeartbeatKey::new(name.clone(), previous.heartbeat_unix_ms, id).pack(),
					)?;
				}
				txn.put(
					&lease_key.pack(),
					&lease_key.serialize(wire::PoolLease { heartbeat_unix_ms })?,
				)?;
				txn.put(
					&PoolHeartbeatKey::new(name, heartbeat_unix_ms, id).pack(),
					&[],
				)?;
			} else {
				status = wire::PoolStatus::NotFound;
			}
		}
		wire::PoolOperation::PoolDisconnect(id) => {
			let id = uuid::Uuid::from_slice(&id).context("invalid connection id")?;
			let lease_key = PoolConnectionKey::new(name.clone(), id);
			if let Some(raw) = txn.get(&lease_key.pack()).await? {
				let lease = lease_key.deserialize(raw.into())?;
				txn.delete(&PoolHeartbeatKey::new(name, lease.heartbeat_unix_ms, id).pack())?;
				txn.delete(&lease_key.pack())?;
			}
		}
	}
	db.commit(txn).await?;
	Ok(wire::PoolResponse { status, config })
}

async fn pool_list_handler(
	db: KvHandle,
	request: wire::PoolListRequest,
) -> Result<wire::PoolListResponse> {
	EnvoyPoolIndexVolume::from_volume_key(db.volume_key())?;
	ensure!(request.limit > 0, "pool list limit must be positive");
	let range = EnvoyPoolPrefix::new().range();
	let start = request
		.after
		.map(|name| Bound::Excluded(EnvoyPoolKey::new(name.to_string()).pack().to_vec()))
		.unwrap_or(Bound::Included(range.start));
	let limit = (request.limit as usize).min(100);
	let entries = db
		.snapshot()
		.await?
		.scan(
			(
				start.as_ref().map(Vec::as_slice),
				Bound::Excluded(range.end.as_slice()),
			),
			Some(limit + 1),
			false,
		)
		.await?;
	let more = entries.len() > limit;
	let mut pools = Vec::new();
	for (key, value) in entries.into_iter().take(limit) {
		let key = EnvoyPoolKey::unpack(&key)?;
		pools.push(wire::PoolEntry {
			name: key.pool_name.clone().into(),
			config: key.deserialize(value.into())?,
		});
	}
	let cursor = if more {
		pools.last().map(|entry| entry.name.clone())
	} else {
		None
	};
	Ok(wire::PoolListResponse { pools, cursor })
}

/// Positive results are permanent because namespaces cannot be renamed or deleted.
pub async fn ensure_namespace(ctx: &EngineContext, name: &str) -> Result<()> {
	crate::validation::namespace_name(name)?;
	if ctx.knows_namespace(name) {
		return Ok(());
	}
	let response = ctx
		.depot()
		.command::<NamespaceRead>(
			&NamespaceCatalogVolume::new().volume_key(),
			wire::NamespaceReadRequest {
				name: name.to_owned().into(),
			},
		)
		.await?;
	if response.record.is_none() {
		return Err(crate::validation::NamespaceNotFound.build());
	}
	ctx.remember_namespace(name);
	Ok(())
}

/// Executes pool administration or connection registration in its namespace's catalog.
pub async fn pool(
	ctx: &EngineContext,
	namespace: &str,
	name: &str,
	operation: wire::PoolOperation,
) -> Result<wire::PoolResponse> {
	ensure_namespace(ctx, namespace).await?;
	crate::validation::pool_name(name)?;
	ctx.depot()
		.command::<Pool>(
			&EnvoyPoolIndexVolume::new(namespace.to_owned()).volume_key(),
			wire::PoolRequest {
				pool_name: name.to_owned().into(),
				operation,
			},
		)
		.await
}





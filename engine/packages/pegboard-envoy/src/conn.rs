use std::{
	sync::{
		Arc,
		atomic::{AtomicI64, AtomicU32},
	},
	time::{Duration, Instant},
};

use anyhow::Context;
use futures_util::StreamExt;
use futures_util::TryStreamExt;
use gas::prelude::*;
use hyper_tungstenite::tungstenite::Message;
use rivet_data::converted::{ActorNameKeyData, MetadataKeyData};
use rivet_envoy_protocol::{self as protocol, versioned};
use rivet_guard_core::WebSocketHandle;
use rivet_types::runner_configs::RunnerConfigKind;
use universaldb::prelude::*;
use vbare::OwnedVersionedData;

use crate::{errors::WsError, metrics, utils::UrlData};

pub struct Conn {
	pub namespace_id: Id,
	pub pool_name: String,
	pub envoy_key: String,
	pub protocol_version: u16,
	pub ws_handle: WebSocketHandle,
	pub is_serverless: bool,
	pub last_rtt: AtomicU32,
	/// Timestamp (epoch ms) of the last pong received from the envoy.
	pub last_ping_ts: AtomicI64,
}

#[tracing::instrument(skip_all)]
pub async fn init_conn(
	ctx: &StandaloneCtx,
	ws_handle: WebSocketHandle,
	UrlData {
		protocol_version,
		namespace,
		pool_name,
		envoy_key,
	}: UrlData,
) -> Result<Arc<Conn>> {
	let start = Instant::now();
	let namespace_name = namespace.clone();
	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input { name: namespace })
		.await
		.with_context(|| format!("failed to resolve namespace: {}", namespace_name))?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())
		.with_context(|| format!("namespace not found: {}", namespace_name))?;

	tracing::debug!(namespace_id=?namespace.namespace_id, "new envoy connection");

	let ws_rx = ws_handle.recv();
	let mut ws_rx = ws_rx.lock().await;

	// Receive init packet
	let Ok(msg) = tokio::time::timeout(Duration::from_secs(5), ws_rx.next()).await else {
		return Err(WsError::TimedOutWaitingForInit.build());
	};

	let Some(msg) = msg else {
		return Err(WsError::ConnectionClosed.build());
	};

	let buf = match msg? {
		Message::Binary(buf) => buf,
		Message::Close(_) => return Err(WsError::ConnectionClosed.build()),
		msg => {
			tracing::debug!(?msg, "invalid initial message");
			return Err(WsError::InvalidInitialPacket("must be a binary blob").build());
		}
	};

	let init = versioned::ToRivet::deserialize(&buf, protocol_version)
		.map_err(|err| WsError::InvalidPacket(err.to_string()).build())
		.context("failed to deserialize initial packet from client")?;

	let protocol::ToRivet::ToRivetInit(init) = init else {
		tracing::debug!(?init, "invalid initial packet");
		return Err(WsError::InvalidInitialPacket("must be `ToRivet::Init`").build());
	};

	metrics::CONNECTION_TOTAL
		.with_label_values(&[
			namespace.namespace_id.to_string().as_str(),
			&pool_name,
			protocol_version.to_string().as_str(),
		])
		.inc();
	metrics::RECEIVE_INIT_PACKET_DURATION
		.with_label_values(&[namespace.namespace_id.to_string().as_str(), &pool_name])
		.observe(start.elapsed().as_secs_f64());

	let mut conn = Conn {
		namespace_id: namespace.namespace_id,
		pool_name,
		envoy_key,
		protocol_version,
		ws_handle,
		is_serverless: false,
		last_rtt: AtomicU32::new(0),
		last_ping_ts: AtomicI64::new(util::timestamp::now()),
	};

	handle_init(ctx, &mut conn, init).await?;

	if conn.is_serverless {
		report_success(ctx, namespace.namespace_id, &conn.pool_name).await;
	}

	Ok(Arc::new(conn))
}

#[tracing::instrument(skip_all)]
pub async fn handle_init(
	ctx: &StandaloneCtx,
	conn: &mut Conn,
	init: protocol::ToRivetInit,
) -> Result<()> {
	let udb = ctx.udb()?;
	let namespace_id = conn.namespace_id;
	let envoy_key = &conn.envoy_key;
	let pool_name = &conn.pool_name;
	let protocol_version = conn.protocol_version;
	let (pool_res, missed_commands) = tokio::try_join!(
		ctx.op(pegboard::ops::runner_config::get::Input {
			runners: vec![(namespace_id, pool_name.clone())],
			bypass_cache: false,
		}),
		// TODO: Move to op
		udb.run(|tx| {
			let init = init.clone();
			async move {
				let tx = tx.with_subspace(pegboard::keys::subspace());

				let create_ts_key =
					pegboard::keys::envoy::CreateTsKey::new(namespace_id, envoy_key.clone());
				let last_ping_ts_key =
					pegboard::keys::envoy::LastPingTsKey::new(namespace_id, envoy_key.clone());
				let version_key =
					pegboard::keys::envoy::VersionKey::new(namespace_id, envoy_key.clone());

				// Read existing data
				let (create_ts_entry, old_last_ping_ts_entry, version_entry) = tokio::try_join!(
					tx.read_opt(&create_ts_key, Serializable),
					tx.read_opt(&create_ts_key, Serializable),
					tx.read_opt(&version_key, Serializable),
				)?;

				// Write init data
				tx.write(
					&pegboard::keys::envoy::PoolNameKey::new(namespace_id, envoy_key.clone()),
					pool_name.clone(),
				)?;
				tx.write(
					&pegboard::keys::envoy::VersionKey::new(namespace_id, envoy_key.clone()),
					init.version,
				)?;
				tx.atomic_op(
					&pegboard::keys::envoy::SlotsKey::new(namespace_id, envoy_key.clone()),
					&0i64.to_le_bytes(),
					MutationType::Add,
				);
				let create_ts = if let Some(create_ts) = create_ts_entry {
					create_ts
				} else {
					let create_ts = util::timestamp::now();
					tx.write(&create_ts_key, util::timestamp::now())?;

					create_ts
				};
				tx.write(
					&pegboard::keys::envoy::LastPingTsKey::new(namespace_id, envoy_key.clone()),
					util::timestamp::now(),
				)?;
				tx.write(
					&pegboard::keys::envoy::ProtocolVersionKey::new(
						namespace_id,
						envoy_key.clone(),
					),
					protocol_version,
				)?;
				let last_ping_ts = util::timestamp::now();
				// Write new ping
				tx.write(&last_ping_ts_key, last_ping_ts)?;

				// Populate ns indexes
				tx.write(
					&pegboard::keys::ns::ActiveEnvoyKey::new(
						namespace_id,
						create_ts,
						envoy_key.clone(),
					),
					(),
				)?;
				tx.write(
					&pegboard::keys::ns::ActiveEnvoyByNameKey::new(
						namespace_id,
						pool_name.clone(),
						create_ts,
						envoy_key.clone(),
					),
					(),
				)?;

				// Unset expired (upon reconnection)
				if create_ts_entry.is_some() {
					tx.delete(&pegboard::keys::envoy::ExpiredTsKey::new(
						namespace_id,
						envoy_key.clone(),
					));
				}

				// Remove old LB entry if exists
				if let (Some(old_last_ping_ts), Some(version)) =
					(old_last_ping_ts_entry, version_entry)
				{
					let old_lb_key = pegboard::keys::ns::EnvoyLoadBalancerIdxKey::new(
						namespace_id,
						pool_name.clone(),
						version,
						old_last_ping_ts,
						envoy_key.clone(),
					);

					tx.add_conflict_key(&old_lb_key, ConflictRangeType::Read)?;
				}

				// Insert into LB
				tx.write(
					&pegboard::keys::ns::EnvoyLoadBalancerIdxKey::new(
						namespace_id,
						pool_name.clone(),
						init.version,
						last_ping_ts,
						envoy_key.clone(),
					),
					(),
				)?;

				// Populate actor names if provided
				if let Some(actor_names) = &init.prepopulate_actor_names {
					// Write each actor name into the namespace actor names list
					for (name, data) in actor_names {
						let metadata = serde_json::from_str::<
							serde_json::Map<String, serde_json::Value>,
						>(&data.metadata)
						.unwrap_or_default();

						tx.write(
							&pegboard::keys::ns::ActorNameKey::new(namespace_id, name.clone()),
							ActorNameKeyData { metadata },
						)?;
					}
				}

				if let Some(metadata) = &init.metadata {
					let metadata = MetadataKeyData {
						metadata:
							serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(
								&metadata,
							)
							.unwrap_or_default(),
					};

					let metadata_key =
						pegboard::keys::envoy::MetadataKey::new(namespace_id, envoy_key.clone());

					// Clear old metadata
					tx.delete_key_subspace(&metadata_key);

					// Write metadata
					for (i, chunk) in metadata_key.split(metadata)?.into_iter().enumerate() {
						let chunk_key = metadata_key.chunk(i);

						tx.set(&tx.pack(&chunk_key), &chunk);
					}
				}

				let envoy_actor_commands_subspace = pegboard::keys::subspace().subspace(
					&pegboard::keys::envoy::ActorCommandKey::subspace(
						namespace_id,
						envoy_key.clone(),
					),
				);

				// Read missed commands
				tx.get_ranges_keyvalues(
					RangeOption {
						mode: StreamingMode::WantAll,
						..(&envoy_actor_commands_subspace).into()
					},
					Serializable,
				)
				.map(|res| {
					let (key, command) =
						tx.read_entry::<pegboard::keys::envoy::ActorCommandKey>(&res?)?;
					match command {
						protocol::ActorCommandKeyData::CommandStartActor(x) => {
							Ok(protocol::CommandWrapper {
								checkpoint: protocol::ActorCheckpoint {
									actor_id: key.actor_id.to_string(),
									generation: key.generation,
									index: key.index,
								},
								inner: protocol::Command::CommandStartActor(x),
							})
						}
						protocol::ActorCommandKeyData::CommandStopActor(x) => {
							Ok(protocol::CommandWrapper {
								checkpoint: protocol::ActorCheckpoint {
									actor_id: key.actor_id.to_string(),
									generation: key.generation,
									index: key.index,
								},
								inner: protocol::Command::CommandStopActor(x),
							})
						}
					}
				})
				.try_collect::<Vec<_>>()
				.await
			}
		})
		.custom_instrument(tracing::info_span!("envoy_process_init_tx")),
	)?;

	conn.is_serverless = pool_res.first().map_or(false, |c| {
		matches!(c.config.kind, RunnerConfigKind::Serverless { .. })
	});
	let pb = ctx.config().pegboard();

	// Send init packet
	let init_msg =
		versioned::ToEnvoy::wrap_latest(protocol::ToEnvoy::ToEnvoyInit(protocol::ToEnvoyInit {
			metadata: protocol::ProtocolMetadata {
				envoy_lost_threshold: pb.envoy_lost_threshold(),
				actor_stop_threshold: pb.actor_stop_threshold(),
				serverless_drain_grace_period: conn
					.is_serverless
					.then(|| pb.serverless_drain_grace_period() as i64),
				max_response_payload_size: pb.envoy_max_response_payload_size() as u64,
			},
		}));
	let init_msg_serialized = init_msg.serialize(conn.protocol_version)?;
	conn.ws_handle
		.send(Message::Binary(init_msg_serialized.into()))
		.await?;

	// Send missed commands
	if !missed_commands.is_empty() {
		let msg =
			versioned::ToEnvoy::wrap_latest(protocol::ToEnvoy::ToEnvoyCommands(missed_commands));
		let msg_serialized = msg.serialize(conn.protocol_version)?;
		conn.ws_handle
			.send(Message::Binary(msg_serialized.into()))
			.await?;
	}

	Ok(())
}

/// Report success to the error tracker workflow.
async fn report_success(ctx: &StandaloneCtx, namespace_id: Id, pool_name: &str) {
	if let Err(err) = ctx
		.signal(pegboard::workflows::runner_pool_error_tracker::ReportSuccess {})
		.to_workflow::<pegboard::workflows::runner_pool_error_tracker::Workflow>()
		.tag("namespace_id", namespace_id)
		.tag("runner_name", pool_name)
		.graceful_not_found()
		.send()
		.await
	{
		tracing::warn!(?err, "failed to report serverless success");
	}
}

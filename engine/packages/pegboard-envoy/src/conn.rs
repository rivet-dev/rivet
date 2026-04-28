use std::{
	sync::{
		Arc,
		atomic::{AtomicI64, AtomicU32},
	},
	time::Instant,
};

use anyhow::Context;
use futures_util::StreamExt;
use futures_util::TryStreamExt;
use gas::prelude::*;
use hyper_tungstenite::tungstenite::Message;
use rivet_envoy_protocol::{self as protocol, versioned};
use rivet_guard_core::WebSocketHandle;
use rivet_types::runner_configs::RunnerConfigKind;
use scc::HashMap;
use sqlite_storage::engine::SqliteEngine;
use universaldb::prelude::*;
use vbare::OwnedVersionedData;

use crate::{actor_lifecycle, errors, metrics, utils::UrlData};

pub struct Conn {
	pub namespace_id: Id,
	pub pool_name: String,
	pub envoy_key: String,
	pub protocol_version: u16,
	pub ws_handle: WebSocketHandle,
	pub authorized_tunnel_routes: HashMap<(protocol::GatewayId, protocol::RequestId), ()>,
	pub sqlite_engine: Arc<SqliteEngine>,
	pub active_actors: HashMap<String, actor_lifecycle::ActiveActor>,
	pub serverless_sqlite_actors: HashMap<String, u64>,
	pub is_serverless: bool,
	pub last_rtt: AtomicU32,
	/// Timestamp (epoch ms) of the last pong received from the envoy.
	pub last_ping_ts: AtomicI64,
}

#[tracing::instrument(skip_all)]
pub async fn init_conn(
	ctx: &StandaloneCtx,
	ws_handle: WebSocketHandle,
	sqlite_engine: Arc<SqliteEngine>,
	UrlData {
		protocol_version,
		namespace,
		pool_name,
		envoy_key,
		version,
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

	let pool_res = ctx
		.op(pegboard::ops::runner_config::get::Input {
			runners: vec![(namespace.namespace_id, pool_name.clone())],
			bypass_cache: false,
		})
		.await?;

	let Some(pool) = pool_res.into_iter().next() else {
		return Err(errors::WsError::NoRunnerConfig {
			pool_name: pool_name.clone(),
		}
		.build());
	};
	let is_serverless = matches!(pool.config.kind, RunnerConfigKind::Serverless { .. });

	tracing::debug!(namespace_id=?namespace.namespace_id, "new envoy connection");

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

	let udb = ctx.udb()?;
	let (_, (mut missed_commands, runner_config_protocol_changed)) = tokio::try_join!(
		// Send init packet as soon as possible
		async {
			let pb = ctx.config().pegboard();

			// Send init packet
			let init_msg = versioned::ToEnvoy::wrap_latest(protocol::ToEnvoy::ToEnvoyInit(
				protocol::ToEnvoyInit {
					metadata: protocol::ProtocolMetadata {
						envoy_lost_threshold: pb.envoy_lost_threshold(),
						actor_stop_threshold: pb.actor_stop_threshold(),
						max_response_payload_size: pb.envoy_max_response_payload_size() as u64,
					},
				},
			));
			let init_msg_serialized = init_msg.serialize(protocol_version)?;
			ws_handle
				.send(Message::Binary(init_msg_serialized.into()))
				.await
		},
		udb.run(|tx| {
			let namespace_id = namespace.namespace_id;
			let envoy_key = &envoy_key;
			let pool_name = &pool_name;
			async move {
				let tx = tx.with_subspace(pegboard::keys::subspace());

				let create_ts_key =
					pegboard::keys::envoy::CreateTsKey::new(namespace_id, envoy_key.to_string());
				let last_ping_ts_key =
					pegboard::keys::envoy::LastPingTsKey::new(namespace_id, envoy_key.to_string());
				let version_key =
					pegboard::keys::envoy::VersionKey::new(namespace_id, envoy_key.to_string());

				// Read existing data
				let (create_ts_entry, old_last_ping_ts_entry, version_entry) = tokio::try_join!(
					tx.read_opt(&create_ts_key, Serializable),
					tx.read_opt(&last_ping_ts_key, Serializable),
					tx.read_opt(&version_key, Serializable),
				)?;

				// Write init data
				tx.write(
					&pegboard::keys::envoy::PoolNameKey::new(namespace_id, envoy_key.to_string()),
					pool_name.to_string(),
				)?;
				tx.write(
					&pegboard::keys::envoy::VersionKey::new(namespace_id, envoy_key.to_string()),
					version,
				)?;
				tx.atomic_op(
					&pegboard::keys::envoy::SlotsKey::new(namespace_id, envoy_key.to_string()),
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
					&pegboard::keys::envoy::LastPingTsKey::new(namespace_id, envoy_key.to_string()),
					util::timestamp::now(),
				)?;
				tx.write(
					&pegboard::keys::envoy::ProtocolVersionKey::new(
						namespace_id,
						envoy_key.to_string(),
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
						envoy_key.to_string(),
					),
					(),
				)?;
				tx.write(
					&pegboard::keys::ns::ActiveEnvoyByNameKey::new(
						namespace_id,
						pool_name.to_string(),
						create_ts,
						envoy_key.to_string(),
					),
					(),
				)?;

				// Unset expired (upon reconnection)
				if create_ts_entry.is_some() {
					tx.delete(&pegboard::keys::envoy::ExpiredTsKey::new(
						namespace_id,
						envoy_key.to_string(),
					));
				}

				// Remove old LB entry if exists
				if let (Some(old_last_ping_ts), Some(version)) =
					(old_last_ping_ts_entry, version_entry)
				{
					let old_lb_key = pegboard::keys::ns::EnvoyLoadBalancerIdxKey::new(
						namespace_id,
						pool_name.to_string(),
						version,
						old_last_ping_ts,
						envoy_key.to_string(),
					);

					tx.add_conflict_key(&old_lb_key, ConflictRangeType::Read)?;
					tx.delete(&old_lb_key);
				}

				// Insert into LB
				tx.write(
					&pegboard::keys::ns::EnvoyLoadBalancerIdxKey::new(
						namespace_id,
						pool_name.to_string(),
						version,
						last_ping_ts,
						envoy_key.to_string(),
					),
					(),
				)?;

				// Update the pool's protocol version. This is required for serverful pools because normally
				// the pool's protocol version is updated via the metadata_poller wf but that only runs for
				// serverless pools.
				let ns_tx = tx.with_subspace(namespace::keys::subspace());
				let runner_config_protocol_version_key =
					pegboard::keys::runner_config::ProtocolVersionKey::new(
						namespace_id,
						pool_name.clone(),
					);

				let envoy_actor_commands_subspace = pegboard::keys::subspace().subspace(
					&pegboard::keys::envoy::ActorCommandKey::subspace(
						namespace_id,
						envoy_key.to_string(),
					),
				);

				let (existing_runner_config_protocol_version, missed_commands) = tokio::try_join!(
					ns_tx.read_opt(&runner_config_protocol_version_key, Serializable),
					// Read missed commands
					tx.get_ranges_keyvalues(
						RangeOption {
							mode: StreamingMode::WantAll,
							..(&envoy_actor_commands_subspace).into()
						},
						Serializable,
					)
					.map(|res| -> anyhow::Result<protocol::CommandWrapper> {
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
					.try_collect::<Vec<_>>(),
				)?;

				let runner_config_protocol_changed =
					existing_runner_config_protocol_version != Some(protocol_version);
				if runner_config_protocol_changed {
					ns_tx.write(&runner_config_protocol_version_key, protocol_version)?;
				}

				Ok((missed_commands, runner_config_protocol_changed))
			}
		})
		.custom_instrument(tracing::info_span!("envoy_init_tx")),
	)?;

	if runner_config_protocol_changed {
		pegboard::utils::purge_runner_config_caches(
			ctx.cache(),
			namespace.namespace_id,
			&pool_name,
		)
		.await?;
	}

	let conn = Arc::new(Conn {
		namespace_id: namespace.namespace_id,
		pool_name,
		envoy_key,
		protocol_version,
		ws_handle,
		authorized_tunnel_routes: HashMap::new(),
		sqlite_engine,
		active_actors: HashMap::new(),
		serverless_sqlite_actors: HashMap::new(),
		is_serverless,
		last_rtt: AtomicU32::new(0),
		last_ping_ts: AtomicI64::new(util::timestamp::now()),
	});

	// Send missed commands (must be after init packet). If any step fails
	// after one or more `start_actor` calls already opened SQLite dbs, close
	// every actor in `conn.active_actors` before returning so we do not leak
	// process-wide `SqliteEngine.open_dbs` entries that would block re-opening
	// these actors until the process restarts.
	if !missed_commands.is_empty() {
		let replay_result: Result<()> = async {
			for cmd_wrapper in &mut missed_commands {
				if let protocol::Command::CommandStartActor(ref mut start) = cmd_wrapper.inner {
					actor_lifecycle::start_actor(ctx, &conn, &cmd_wrapper.checkpoint, start)
						.await?;
				} else if let protocol::Command::CommandStopActor(_) = cmd_wrapper.inner {
					actor_lifecycle::stop_actor(&conn, &cmd_wrapper.checkpoint).await?;
				}
			}
			Ok(())
		}
		.await;
		if let Err(err) = replay_result {
			actor_lifecycle::shutdown_conn_actors(&conn).await;
			return Err(err);
		}

		let msg =
			versioned::ToEnvoy::wrap_latest(protocol::ToEnvoy::ToEnvoyCommands(missed_commands));
		let msg_serialized = msg.serialize(protocol_version)?;
		if let Err(err) = conn
			.ws_handle
			.send(Message::Binary(msg_serialized.into()))
			.await
		{
			actor_lifecycle::shutdown_conn_actors(&conn).await;
			return Err(err.into());
		}
	}

	if is_serverless {
		report_success(ctx, namespace.namespace_id, &conn.pool_name).await;
	}

	Ok(conn)
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

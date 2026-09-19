use std::{
	sync::{
		Arc,
		atomic::{AtomicBool, AtomicI64, AtomicU32},
	},
	time::Instant,
};

use anyhow::Context;
use depot::conveyer::Db;
use depot_client::database::NativeDatabaseHandle;
use gas::prelude::*;
use hyper_tungstenite::tungstenite::Message;
use pegboard::keys::{envoy::VirtualNodesKey, ns::EnvoyHashIdxKey};
use rivet_config::config::pegboard::EnvoyLoadBalancer;
use rivet_envoy_protocol::{self as protocol, versioned};
use rivet_guard_core::WebSocketHandle;
use rivet_metrics::GaugeGuardExt;
use rivet_pools::NodeId;
use rivet_types::runner_configs::{RunnerConfig, RunnerConfigKind};
use scc::HashMap;
use universaldb::prelude::*;
use vbare::OwnedVersionedData;
use xxhash_rust::xxh3::xxh3_128_with_seed;

use crate::{actor_lifecycle, errors, hibernating_requests, metrics, utils::UrlData};

pub type RemoteSqliteExecutors =
	HashMap<(String, u64), Arc<tokio::sync::OnceCell<NativeDatabaseHandle>>>;

pub struct Conn {
	pub namespace_id: Id,
	pub namespace_name: String,
	pub pool_name: String,
	pub envoy_key: String,
	pub envoy_conn_id: Id,
	pub protocol_version: u16,
	pub ws_handle: WebSocketHandle,
	pub authorized_tunnel_routes: HashMap<(protocol::GatewayId, protocol::RequestId), ()>,
	/// Tracks in-flight ToEnvoyWebSocketOpen sends so we can observe envoy-side
	/// actor wake duration when the matching ToRivetWebSocketOpen (or
	/// ToRivetWebSocketClose) reply arrives back from the envoy.
	pub pending_websocket_opens: HashMap<(protocol::GatewayId, protocol::RequestId), Instant>,
	pub udb: Arc<universaldb::Database>,
	pub node_id: NodeId,
	/// This is a perf-only SQLite conveyer cache, not authoritative actor presence tracking.
	/// Envoys can reconnect to different worker nodes mid-flight, so request handlers
	/// lazily populate it and lifecycle commands only evict stale cache entries.
	pub actor_dbs: HashMap<String, Arc<Db>>,
	pub remote_sqlite_executors: RemoteSqliteExecutors,
	pub pool: RunnerConfig,
	pub connected_at: Instant,
	pub last_rtt: AtomicU32,
	/// Timestamp (epoch ms) of the last pong received from the envoy.
	pub last_ping_ts: AtomicI64,
	pub reported_stopping: AtomicBool,
}

impl Conn {
	pub fn is_serverless(&self) -> bool {
		matches!(self.pool.kind, RunnerConfigKind::Serverless { .. })
	}

	/// Returns whether this websocket still owns the persisted envoy registration.
	///
	/// Eviction pubsub messages are only wakeups: concurrent connections can observe messages in a
	/// different order than their registration transactions committed, so the database owner is the
	/// authority for deciding which connection must exit.
	pub async fn is_current_registration(&self) -> Result<bool> {
		self.udb
			.txn("envoy_conn_is_current", |tx| async move {
				let tx = tx.with_subspace(pegboard::keys::subspace());
				let current_envoy_conn_id = tx
					.read_opt(
						&pegboard::keys::envoy::EnvoyConnIdKey::new(
							self.namespace_id,
							self.envoy_key.clone(),
						),
						Serializable,
					)
					.await?;

				Ok(current_envoy_conn_id == Some(self.envoy_conn_id))
			})
			.await
	}
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
	let envoy_conn_id = Id::new_v1(ctx.config().dc_label());
	let conn_udb = Arc::new((*udb).clone());
	let node_id = ctx.pools().node_id();
	// Only populate the hash-ring index when the operator has selected the Hash strategy.
	let virtual_nodes = match ctx.config().pegboard().envoy_load_balancer() {
		EnvoyLoadBalancer::Hash { virtual_nodes, .. } => Some(virtual_nodes),
		_ => None,
	};
	let (_, runner_config_protocol_changed, missed_commands) = tokio::try_join!(
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
		udb.txn("envoy_conn_prepare", |tx| {
			let namespace_id = namespace.namespace_id;
			let pool_name = &pool_name;
			async move {
				// Detect whether the serverful-pool protocol cache must be purged before the final
				// registration claim updates the persisted version.
				let existing_runner_config_protocol_version = tx
					.with_subspace(namespace::keys::subspace())
					.read_opt(
						&pegboard::keys::runner_config::ProtocolVersionKey::new(
							namespace_id,
							pool_name.clone(),
						),
						Serializable,
					)
					.await?;

				Ok(existing_runner_config_protocol_version != Some(protocol_version))
			}
		})
		.custom_instrument(tracing::info_span!("envoy_prepare_tx")),
		// Read missed commands
		pegboard::keys::envoy::read_actor_commands(
			&udb,
			namespace.namespace_id,
			&envoy_key,
			pegboard::keys::envoy::ACTOR_COMMAND_PAGE_BYTES,
		),
	)?;

	let mut missed_commands = missed_commands
		.into_iter()
		.map(|(key, command)| protocol::CommandWrapper {
			checkpoint: protocol::ActorCheckpoint {
				actor_id: key.actor_id.to_string(),
				generation: key.generation,
				index: key.index,
			},
			inner: match command {
				protocol::ActorCommandKeyData::CommandStartActor(x) => {
					protocol::Command::CommandStartActor(x)
				}
				protocol::ActorCommandKeyData::CommandStopActor(x) => {
					protocol::Command::CommandStopActor(x)
				}
			},
		})
		.collect::<Vec<_>>();

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
		namespace_name,
		pool_name,
		envoy_key,
		envoy_conn_id,
		protocol_version,
		ws_handle,
		authorized_tunnel_routes: HashMap::new(),
		pending_websocket_opens: HashMap::new(),
		udb: conn_udb,
		node_id,
		actor_dbs: HashMap::new(),
		remote_sqlite_executors: HashMap::new(),
		pool: pool.config,
		connected_at: Instant::now(),
		last_rtt: AtomicU32::new(0),
		last_ping_ts: AtomicI64::new(util::timestamp::now()),
		reported_stopping: AtomicBool::new(false),
	});

	// Prepare missed commands after the init packet, but do not transmit actor lifecycle work until
	// this connection owns the registration. The previous connection remains authoritative here.
	let missed_commands_prepared = if missed_commands.is_empty() {
		None
	} else {
		let replay_result: Result<Vec<protocol::ActorCheckpoint>> = async {
			let mut stop_checkpoints = Vec::new();
			for cmd_wrapper in &mut missed_commands {
				hibernating_requests::hydrate_command_wrapper(ctx, cmd_wrapper).await?;
				if matches!(&cmd_wrapper.inner, protocol::Command::CommandStopActor(_)) {
					stop_checkpoints.push(cmd_wrapper.checkpoint.clone());
				}
			}
			Ok(stop_checkpoints)
		}
		.await;
		let stop_checkpoints = match replay_result {
			Ok(stop_checkpoints) => stop_checkpoints,
			Err(err) => {
				actor_lifecycle::shutdown_conn_actors(&conn).await;
				return Err(err);
			}
		};

		let msg =
			versioned::ToEnvoy::wrap_latest(protocol::ToEnvoy::ToEnvoyCommands(missed_commands));
		Some((msg.serialize(protocol_version)?, stop_checkpoints))
	};

	// Transfer ownership only after every fallible initialization step that can run while the
	// previous connection remains authoritative has succeeded. This transaction installs the owner
	// and every scheduler-visible index together, so a failed prepare cannot expose a provisional
	// connection to allocation.
	let runner_config_protocol_changed_at_claim = claim_registration(
		&udb,
		conn.namespace_id,
		&conn.envoy_key,
		conn.envoy_conn_id,
		&conn.pool_name,
		version,
		conn.protocol_version,
		virtual_nodes,
	)
	.await?;

	// The replay can start actors, so send it only after the connection owns the registration. A
	// failed websocket write may have delivered a prefix; fail closed by expiring only this claimed
	// connection instead of restoring its predecessor.
	if let Some((msg_serialized, stop_checkpoints)) = missed_commands_prepared {
		for checkpoint in &stop_checkpoints {
			if let Err(err) = actor_lifecycle::stop_actor(&conn, checkpoint).await {
				fail_closed_post_claim(ctx, &conn, "stop actor cache invalidation").await;
				return Err(err);
			}
		}

		let _in_flight = metrics::WS_RESPONSES_IN_FLIGHT.inc_guard();
		if let Err(err) = conn
			.ws_handle
			.send(Message::Binary(msg_serialized.into()))
			.await
		{
			drop(_in_flight);
			fail_closed_post_claim(ctx, &conn, "command replay websocket send").await;
			return Err(err.into());
		}
		drop(_in_flight);
	}
	if runner_config_protocol_changed_at_claim {
		// The required pre-claim purge keeps initialization failure-safe. Purge again after the
		// persisted version changes so a concurrent reader cannot repopulate the old value in the
		// purge-to-claim gap. The connection is ready and authoritative now, so a transient cache
		// failure must not tear it down.
		if let Err(err) = pegboard::utils::purge_runner_config_caches(
			ctx.cache(),
			conn.namespace_id,
			&conn.pool_name,
		)
		.await
		{
			tracing::warn!(
				namespace_id = ?conn.namespace_id,
				pool_name = %conn.pool_name,
				?err,
				"failed to purge runner config caches after envoy registration"
			);
		}
	}

	if conn.is_serverless() {
		report_success(ctx, namespace.namespace_id, &conn.pool_name).await;
	}

	Ok(conn)
}

async fn fail_closed_post_claim(ctx: &StandaloneCtx, conn: &Conn, failure_stage: &'static str) {
	if let Err(expire_err) = ctx
		.op(pegboard::ops::envoy::expire::Input {
			namespace_id: conn.namespace_id,
			envoy_key: conn.envoy_key.clone(),
			expected_envoy_conn_id: Some(conn.envoy_conn_id),
			skip_if_fresh: false,
		})
		.await
	{
		tracing::error!(
			namespace_id = ?conn.namespace_id,
			envoy_key = %conn.envoy_key,
			envoy_conn_id = ?conn.envoy_conn_id,
			failure_stage,
			?expire_err,
			"failed to expire envoy after post-claim initialization failure",
		);
	}
	actor_lifecycle::shutdown_conn_actors(conn).await;
}

#[allow(clippy::too_many_arguments)]
async fn claim_registration(
	udb: &universaldb::Database,
	namespace_id: Id,
	envoy_key: &str,
	envoy_conn_id: Id,
	pool_name: &str,
	version: u32,
	protocol_version: u16,
	virtual_nodes: Option<u8>,
) -> Result<bool> {
	udb.txn("envoy_conn_register", |tx| async move {
		let tx = tx.with_subspace(pegboard::keys::subspace());
		let create_ts_key =
			pegboard::keys::envoy::CreateTsKey::new(namespace_id, envoy_key.to_string());
		let last_ping_ts_key =
			pegboard::keys::envoy::LastPingTsKey::new(namespace_id, envoy_key.to_string());
		let version_key =
			pegboard::keys::envoy::VersionKey::new(namespace_id, envoy_key.to_string());

		let (create_ts_entry, old_last_ping_ts_entry, version_entry) = tokio::try_join!(
			tx.read_opt(&create_ts_key, Serializable),
			tx.read_opt(&last_ping_ts_key, Serializable),
			tx.read_opt(&version_key, Serializable),
		)?;

		if let Some(old_version) = version_entry {
			if old_version != version {
				tracing::warn!(
					?namespace_id,
					%envoy_key,
					old_version,
					new_version = version,
					"envoy_key reconnected with changed version; operationally prohibited - investigate"
				);
			}
		}

		tx.write(
			&pegboard::keys::envoy::EnvoyConnIdKey::new(namespace_id, envoy_key.to_string()),
			envoy_conn_id,
		)?;
		tx.write(
			&pegboard::keys::envoy::PoolNameKey::new(namespace_id, envoy_key.to_string()),
			pool_name.to_string(),
		)?;
		tx.write(&version_key, version)?;
		tx.atomic_op(
			&pegboard::keys::envoy::SlotsKey::new(namespace_id, envoy_key.to_string()),
			&0i64.to_le_bytes(),
			MutationType::Add,
		);

		let create_ts = if let Some(create_ts) = create_ts_entry {
			create_ts
		} else {
			let create_ts = util::timestamp::now();
			tx.write(&create_ts_key, create_ts)?;
			create_ts
		};
		let last_ping_ts = util::timestamp::now();
		tx.write(&last_ping_ts_key, last_ping_ts)?;
		tx.write(
			&pegboard::keys::envoy::ProtocolVersionKey::new(namespace_id, envoy_key.to_string()),
			protocol_version,
		)?;

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

		if create_ts_entry.is_some() {
			tx.delete(&pegboard::keys::envoy::ExpiredTsKey::new(
				namespace_id,
				envoy_key.to_string(),
			));
		}

		if let (Some(old_last_ping_ts), Some(old_version)) = (old_last_ping_ts_entry, version_entry)
		{
			let old_lb_key = pegboard::keys::ns::EnvoyLoadBalancerIdxKey::new(
				namespace_id,
				pool_name.to_string(),
				old_version,
				old_last_ping_ts,
				envoy_key.to_string(),
			);
			tx.add_conflict_key(&old_lb_key, ConflictRangeType::Read)?;
			tx.delete(&old_lb_key);
		}

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

		if let Some(virtual_nodes) = virtual_nodes {
			tx.write(
				&VirtualNodesKey::new(namespace_id, envoy_key.to_string()),
				virtual_nodes,
			)?;
			for i in 0..virtual_nodes {
				tx.write(
					&EnvoyHashIdxKey::new(
						namespace_id,
						pool_name.to_string(),
						version,
						xxh3_128_with_seed(envoy_key.as_bytes(), i as u64).to_be_bytes(),
						envoy_key.to_string(),
					),
					(),
				)?;
			}
		}

		// Serverful pools do not run the metadata poller, so registration owns this update. Read
		// the value in the claim transaction so concurrent registrations cannot hide a cache change.
		let ns_tx = tx.with_subspace(namespace::keys::subspace());
		let runner_config_protocol_version_key =
			pegboard::keys::runner_config::ProtocolVersionKey::new(
				namespace_id,
				pool_name.to_string(),
			);
		let old_runner_config_protocol_version = ns_tx
			.read_opt(&runner_config_protocol_version_key, Serializable)
			.await?;
		ns_tx.write(&runner_config_protocol_version_key, protocol_version)?;

		Ok(old_runner_config_protocol_version != Some(protocol_version))
	})
	.custom_instrument(tracing::info_span!("envoy_register_tx"))
	.await
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

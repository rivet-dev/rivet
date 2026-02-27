use std::{
	sync::{Arc, atomic::AtomicU32},
	time::{Duration, Instant},
};

use anyhow::Context;
use futures_util::StreamExt;
use futures_util::TryStreamExt;
use gas::prelude::*;
use hyper_tungstenite::tungstenite::Message;
use pegboard::ops::runner::update_alloc_idx::{Action, RunnerEligibility};
use rivet_data::converted::{ActorNameKeyData, MetadataKeyData};
use rivet_guard_core::WebSocketHandle;
use rivet_runner_protocol::{self as protocol, versioned};
use rivet_types::runner_configs::RunnerConfigKind;
use universaldb::prelude::*;
use vbare::OwnedVersionedData;

use crate::{errors::WsError, metrics, utils::UrlData};

pub struct Conn {
	pub namespace_id: Id,
	pub runner_name: String,
	pub runner_key: String,
	pub runner_id: Id,
	pub workflow_id: Id,
	pub protocol_version: u16,
	pub ws_handle: WebSocketHandle,
	pub last_rtt: AtomicU32,
}

#[tracing::instrument(skip_all)]
pub async fn init_conn(
	ctx: &StandaloneCtx,
	ws_handle: WebSocketHandle,
	UrlData {
		protocol_version,
		namespace,
		runner_key,
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

	tracing::debug!(namespace_id=?namespace.namespace_id, "new runner connection");

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

	let init = Init::new(&buf, protocol_version)?;

	metrics::CONNECTION_TOTAL
		.with_label_values(&[
			namespace.namespace_id.to_string().as_str(),
			init.name(),
			protocol_version.to_string().as_str(),
		])
		.inc();
	metrics::RECEIVE_INIT_PACKET_DURATION
		.with_label_values(&[namespace.namespace_id.to_string().as_str(), init.name()])
		.observe(start.elapsed().as_secs_f64());

	// Look up existing runner by key
	let existing_runner = ctx
		.op(pegboard::ops::runner::get_by_key::Input {
			namespace_id: namespace.namespace_id,
			name: init.name().to_string(),
			key: runner_key.clone(),
		})
		.await
		.with_context(|| {
			format!(
				"failed to get existing runner by key: {}:{}",
				init.name(),
				runner_key
			)
		})?;

	let runner_id = if let Some(runner) = existing_runner.runner {
		// IMPORTANT: Before we spawn/get the workflow, we try to update the runner's last ping ts.
		// This ensures if the workflow is currently checking for expiry that it will not expire
		// (because we are about to send signals to it) and if it is already expired (but not
		// completed) we can choose a new runner id.
		let update_ping_res = ctx
			.op(pegboard::ops::runner::update_alloc_idx::Input {
				runners: vec![pegboard::ops::runner::update_alloc_idx::Runner {
					runner_id: runner.runner_id,
					action: Action::UpdatePing { rtt: 0 },
				}],
			})
			.await
			.with_context(|| format!("failed to update ping for runner: {}", runner.runner_id))?;

		if update_ping_res
			.notifications
			.into_iter()
			.next()
			.map(|notif| matches!(notif.eligibility, RunnerEligibility::Expired))
			.unwrap_or_default()
		{
			// Runner expired, create a new one
			Id::new_v1(ctx.config().dc_label())
		} else {
			// Use existing runner
			runner.runner_id
		}
	} else {
		// No existing runner for this key, create a new one
		Id::new_v1(ctx.config().dc_label())
	};

	// Spawn a new runner workflow if one doesn't already exist
	let workflow_id = if protocol::is_mk2(protocol_version) {
		ctx.workflow(pegboard::workflows::runner2::Input {
			runner_id,
			namespace_id: namespace.namespace_id,
			name: init.name().to_string(),
			key: runner_key.clone(),
			version: init.version(),
			total_slots: init.total_slots(),
			protocol_version,
		})
		.tag("runner_id", runner_id)
		.unique()
		.dispatch()
		.await
		.with_context(|| {
			format!(
				"failed to dispatch runner workflow for runner: {}",
				runner_id
			)
		})?
	} else {
		ctx.workflow(pegboard::workflows::runner::Input {
			runner_id,
			namespace_id: namespace.namespace_id,
			name: init.name().to_string(),
			key: runner_key.clone(),
			version: init.version(),
			total_slots: init.total_slots(),
		})
		.tag("runner_id", runner_id)
		.unique()
		.dispatch()
		.await
		.with_context(|| {
			format!(
				"failed to dispatch runner workflow for runner: {}",
				runner_id
			)
		})?
	};

	let conn = Arc::new(Conn {
		namespace_id: namespace.namespace_id,
		runner_name: init.name().to_string(),
		runner_key,
		runner_id,
		workflow_id,
		protocol_version,
		ws_handle,
		last_rtt: AtomicU32::new(0),
	});

	match init {
		Init::Mk2(init) => handle_init(ctx, &conn, init).await?,
		Init::Mk1(init) => {
			// Forward to runner wf
			ctx.signal(pegboard::workflows::runner::Forward {
				inner: protocol::ToServer::ToServerInit(init),
			})
			.to_workflow_id(workflow_id)
			.send()
			.await
			.with_context(|| {
				format!(
					"failed to forward initial packet to workflow: {}",
					workflow_id
				)
			})?;
		}
	}

	Ok(conn)
}

enum Init {
	Mk2(protocol::mk2::ToServerInit),
	Mk1(protocol::ToServerInit),
}

impl Init {
	fn new(buf: &[u8], protocol_version: u16) -> Result<Self> {
		if protocol::is_mk2(protocol_version) {
			let init_packet = versioned::ToServerMk2::deserialize(&buf, protocol_version)
				.map_err(|err| WsError::InvalidPacket(err.to_string()).build())
				.context("failed to deserialize initial packet from client")?;

			let protocol::mk2::ToServer::ToServerInit(init) = init_packet else {
				tracing::debug!(?init_packet, "invalid initial packet");
				return Err(WsError::InvalidInitialPacket("must be `ToServer::Init`").build());
			};

			Ok(Init::Mk2(init))
		} else {
			let init_packet = versioned::ToServer::deserialize(&buf, protocol_version)
				.map_err(|err| WsError::InvalidPacket(err.to_string()).build())
				.context("failed to deserialize initial packet from client")?;

			let protocol::ToServer::ToServerInit(init) = init_packet else {
				tracing::debug!(?init_packet, "invalid initial packet");
				return Err(WsError::InvalidInitialPacket("must be `ToServer::Init`").build());
			};

			Ok(Init::Mk1(init))
		}
	}

	fn name(&self) -> &str {
		match self {
			Init::Mk2(init) => &init.name,
			Init::Mk1(init) => &init.name,
		}
	}

	fn version(&self) -> u32 {
		match self {
			Init::Mk2(init) => init.version,
			Init::Mk1(init) => init.version,
		}
	}

	fn total_slots(&self) -> u32 {
		match self {
			Init::Mk2(init) => init.total_slots,
			Init::Mk1(init) => init.total_slots,
		}
	}
}

#[tracing::instrument(skip_all)]
pub async fn handle_init(
	ctx: &StandaloneCtx,
	conn: &Conn,
	init: protocol::mk2::ToServerInit,
) -> Result<()> {
	// We send the signal first because we don't want to continue if this fails
	ctx.signal(pegboard::workflows::runner2::Init {})
		.to_workflow_id(conn.workflow_id)
		.send()
		.await
		.with_context(|| {
			format!(
				"failed to send signal to runner workflow: {}",
				conn.workflow_id
			)
		})?;

	let udb = ctx.udb()?;
	let (runner_config_res, missed_commands) = tokio::try_join!(
		ctx.op(pegboard::ops::runner_config::get::Input {
			runners: vec![(conn.namespace_id, conn.runner_name.clone())],
			bypass_cache: false,
		}),
		udb.run(|tx| {
			let init = init.clone();
			async move {
				let tx = tx.with_subspace(pegboard::keys::subspace());

				// Populate actor names if provided
				if let Some(actor_names) = &init.prepopulate_actor_names {
					// Write each actor name into the namespace actor names list
					for (name, data) in actor_names {
						let metadata = serde_json::from_str::<
							serde_json::Map<String, serde_json::Value>,
						>(&data.metadata)
						.unwrap_or_default();

						tx.write(
							&pegboard::keys::ns::ActorNameKey::new(conn.namespace_id, name.clone()),
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

					let metadata_key = pegboard::keys::runner::MetadataKey::new(conn.runner_id);

					// Clear old metadata
					tx.delete_key_subspace(&metadata_key);

					// Write metadata
					for (i, chunk) in metadata_key.split(metadata)?.into_iter().enumerate() {
						let chunk_key = metadata_key.chunk(i);

						tx.set(&tx.pack(&chunk_key), &chunk);
					}
				}

				let runner_actor_commands_subspace = pegboard::keys::subspace().subspace(
					&pegboard::keys::runner::ActorCommandKey::subspace(conn.runner_id),
				);

				// Read missed commands
				tx.get_ranges_keyvalues(
					RangeOption {
						mode: StreamingMode::WantAll,
						..(&runner_actor_commands_subspace).into()
					},
					Serializable,
				)
				.map(|res| {
					let (key, command) =
						tx.read_entry::<pegboard::keys::runner::ActorCommandKey>(&res?)?;
					match command {
						protocol::mk2::ActorCommandKeyData::CommandStartActor(x) => {
							Ok(protocol::mk2::CommandWrapper {
								checkpoint: protocol::mk2::ActorCheckpoint {
									actor_id: key.actor_id.to_string(),
									generation: key.generation,
									index: key.index,
								},
								inner: protocol::mk2::Command::CommandStartActor(x),
							})
						}
						protocol::mk2::ActorCommandKeyData::CommandStopActor => {
							Ok(protocol::mk2::CommandWrapper {
								checkpoint: protocol::mk2::ActorCheckpoint {
									actor_id: key.actor_id.to_string(),
									generation: key.generation,
									index: key.index,
								},
								inner: protocol::mk2::Command::CommandStopActor,
							})
						}
					}
				})
				.try_collect::<Vec<_>>()
				.await
			}
		})
		.custom_instrument(tracing::info_span!("runner_process_init_tx")),
	)?;

	let is_serverless = runner_config_res.first().map_or(false, |c| {
		matches!(c.config.kind, RunnerConfigKind::Serverless { .. })
	});
	let pb = ctx.config().pegboard();

	// Send init packet
	let init_msg = versioned::ToClientMk2::wrap_latest(protocol::mk2::ToClient::ToClientInit(
		protocol::mk2::ToClientInit {
			runner_id: conn.runner_id.to_string(),
			metadata: protocol::mk2::ProtocolMetadata {
				runner_lost_threshold: pb.runner_lost_threshold(),
				actor_stop_threshold: pb.actor_stop_threshold(),
				serverless_drain_grace_period: is_serverless
					.then(|| pb.serverless_drain_grace_period() as i64),
			},
		},
	));
	let init_msg_serialized = init_msg.serialize(conn.protocol_version)?;
	conn.ws_handle
		.send(Message::Binary(init_msg_serialized.into()))
		.await?;

	// Send missed commands
	if !missed_commands.is_empty() {
		let msg = versioned::ToClientMk2::wrap_latest(protocol::mk2::ToClient::ToClientCommands(
			missed_commands,
		));
		let msg_serialized = msg.serialize(conn.protocol_version)?;
		conn.ws_handle
			.send(Message::Binary(msg_serialized.into()))
			.await?;
	}

	Ok(())
}

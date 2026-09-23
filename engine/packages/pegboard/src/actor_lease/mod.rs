//! Experimental serverful actor leases. Enable only on a fresh benchmark database.
//! The transaction owns state; pubsub is a retryable delivery hint, never the commit.
use gas::prelude::*;
use rivet_envoy_protocol::{self as protocol, versioned};
use universaldb::prelude::*;
use universalpubsub::PublishOpts;
use vbare::OwnedVersionedData;

use crate::keys;

pub fn enabled() -> bool {
	static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
	*ENABLED.get_or_init(|| std::env::var("RIVET_ACTOR_LEASE_POC").as_deref() == Ok("1"))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Phase {
	Sleeping,
	Starting,
	Running,
	Stopping,
}

// Wire layout is documented in v1.bare. Never change this persisted layout in place.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lease {
	pub namespace_id: Id,
	pub pool_name: String,
	pub config: protocol::ActorConfig,
	pub generation: u32,
	pub protocol_version: u16,
	pub envoy_key: Option<String>,
	pub connection_id: Option<Id>,
	pub phase: Phase,
	pub last_event: i64,
	pub command: Option<protocol::CommandWrapper>,
	pub sleep_ts: Option<i64>,
	pub start_ts: Option<i64>,
	pub connectable_ts: Option<i64>,
	pub destroy_ts: Option<i64>,
	pub alarm_ts: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LeaseV1 {
	pub namespace_id: Id,
	pub pool_name: String,
	pub config: protocol::generated::v8::ActorConfig,
	pub generation: u32,
	pub protocol_version: u16,
	pub envoy_key: Option<String>,
	pub connection_id: Option<Id>,
	pub phase: Phase,
	pub last_event: i64,
	pub command: Option<protocol::generated::v8::CommandWrapper>,
	pub sleep_ts: Option<i64>,
	pub start_ts: Option<i64>,
	pub connectable_ts: Option<i64>,
	pub destroy_ts: Option<i64>,
	pub alarm_ts: Option<i64>,
}

impl OwnedVersionedData for Lease {
	type Latest = Self;
	fn wrap_latest(value: Self) -> Self {
		value
	}
	fn unwrap_latest(self) -> Result<Self> {
		Ok(self)
	}
	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		if version == 2 {
			return Ok(serde_bare::from_slice(payload)?);
		}
		ensure!(version == 1, "unsupported actor lease version");
		let old: LeaseV1 = serde_bare::from_slice(payload)?;
		Ok(Self {
			namespace_id: old.namespace_id,
			pool_name: old.pool_name,
			config: versioned::v8_to_v9::convert_actor_config_v8_to_v9(old.config)?,
			generation: old.generation,
			protocol_version: old.protocol_version,
			envoy_key: old.envoy_key,
			connection_id: old.connection_id,
			phase: old.phase,
			last_event: old.last_event,
			command: old
				.command
				.map(versioned::v8_to_v9::convert_command_wrapper_v8_to_v9)
				.transpose()?,
			sleep_ts: old.sleep_ts,
			start_ts: old.start_ts,
			connectable_ts: old.connectable_ts,
			destroy_ts: old.destroy_ts,
			alarm_ts: old.alarm_ts,
		})
	}
	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		ensure!(version == 2, "unsupported actor lease version");
		Ok(serde_bare::to_vec(&self)?)
	}
	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		Vec::<fn(Self) -> Result<Self>>::new()
	}
	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		// vbare derives the latest writable version from this converter count.
		vec![|_: Self| bail!("actor lease v2 cannot be downgraded")]
	}
}

fn lease_key(actor_id: Id) -> impl TuplePack {
	(ACTOR, DATA, actor_id, "guard_lease_poc")
}

pub async fn read(tx: &universaldb::Transaction, actor_id: Id) -> Result<Option<Lease>> {
	tx.get(&tx.pack(&lease_key(actor_id)), Serializable)
		.await?
		.map(|raw| Lease::deserialize_with_embedded_version(&raw)?.unwrap_latest())
		.transpose()
}

fn write(tx: &universaldb::Transaction, actor_id: Id, lease: &Lease) -> Result<()> {
	tx.set(
		&tx.pack(&lease_key(actor_id)),
		&lease.clone().serialize_with_embedded_version(2)?,
	);
	Ok(())
}

pub fn initialize(
	tx: &universaldb::Transaction,
	actor_id: Id,
	namespace_id: Id,
	pool_name: String,
	config: protocol::ActorConfig,
) -> Result<()> {
	let lease = Lease {
		namespace_id,
		pool_name,
		config,
		generation: 0,
		protocol_version: 0,
		envoy_key: None,
		connection_id: None,
		phase: Phase::Sleeping,
		last_event: -1,
		command: None,
		sleep_ts: Some(util::timestamp::now()),
		start_ts: None,
		connectable_ts: None,
		destroy_ts: None,
		alarm_ts: None,
	};
	write(tx, actor_id, &lease)?;
	tx.write(
		&keys::actor::SleepTsKey::new(actor_id),
		lease.sleep_ts.unwrap(),
	)?;
	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
	Acquire,
	Sleep,
	Destroy,
	Read,
	Events {
		namespace_id: Id,
		envoy_key: String,
		connection_id: Id,
		events: Vec<protocol::EventWrapper>,
	},
}

#[derive(Debug)]
pub struct Input {
	pub actor_id: Id,
	pub action: Action,
}

#[operation]
pub async fn pegboard_actor_lease(ctx: &OperationCtx, input: &Input) -> Result<Lease> {
	let lease = commit(ctx.config(), ctx.pools(), input).await?;

	// Cancellation here leaves a durable command. Any acquire retry sends it again.
	if !matches!(input.action, Action::Read) {
		if let (Some(envoy), Some(command)) = (&lease.envoy_key, &lease.command) {
			publish(
				ctx,
				lease.namespace_id,
				envoy,
				protocol::ToEnvoyConn::ToEnvoyCommands(vec![command.clone()]),
			)
			.await?;
		}
	}
	if let Action::Events {
		namespace_id,
		envoy_key,
		events,
		..
	} = &input.action
	{
		// Schedule delivery precedes ACK; replay after interruption reads durable alarm state.
		if events.iter().any(|e| {
			matches!(
				e.inner,
				protocol::Event::EventActorSetAlarm(_)
					| protocol::Event::EventActorStateUpdate(protocol::EventActorStateUpdate {
						state: protocol::ActorState::ActorStateStopped(_),
						..
					})
			)
		}) {
			ctx.signal(crate::workflows::actor2::LeaseMaintenance {})
				.to_workflow::<crate::workflows::actor2::Workflow>()
				.tag("actor_id", input.actor_id)
				.graceful_not_found()
				.send()
				.await?;
		}
		if lease.phase == Phase::Running {
			ctx.msg(crate::workflows::actor2::Ready {
				envoy_key: lease.envoy_key.clone().unwrap(),
			})
			.topic(("actor_id", input.actor_id))
			.send()
			.await?;
		} else if lease.phase == Phase::Sleeping {
			ctx.msg(crate::workflows::actor2::Stopped {})
				.topic(("actor_id", input.actor_id))
				.send()
				.await?;
		}
		if let Some(last) = events.last() {
			publish(
				ctx,
				*namespace_id,
				envoy_key,
				protocol::ToEnvoyConn::ToEnvoyAckEvents(protocol::ToEnvoyAckEvents {
					last_event_checkpoints: vec![last.checkpoint.clone()],
				}),
			)
			.await?;
		}
	}
	Ok(lease)
}

fn stop(tx: &universaldb::Transaction, actor_id: Id, lease: &mut Lease) -> Result<()> {
	if matches!(lease.phase, Phase::Starting | Phase::Running) {
		lease.phase = Phase::Stopping;
		lease.connectable_ts = None;
		tx.delete(&keys::actor::ConnectableKey::new(actor_id));
		lease.command = Some(protocol::CommandWrapper {
			checkpoint: protocol::ActorCheckpoint {
				actor_id: actor_id.to_string(),
				generation: lease.generation,
				index: 2,
			},
			inner: protocol::Command::CommandStopActor(protocol::CommandStopActor {
				reason: if lease.destroy_ts.is_some() {
					protocol::StopActorReason::Destroy
				} else {
					protocol::StopActorReason::SleepIntent
				},
			}),
		});
		persist_command(tx, lease)?;
	}
	Ok(())
}

fn release(tx: &universaldb::Transaction, actor_id: Id, lease: &mut Lease, now: i64) -> Result<()> {
	if let Some(envoy) = lease.envoy_key.take() {
		tx.delete(&keys::envoy::ActorKey::new(
			lease.namespace_id,
			envoy.clone(),
			actor_id,
		));
		tx.atomic_op(
			&keys::envoy::SlotsKey::new(lease.namespace_id, envoy),
			&(-1i64).to_le_bytes(),
			MutationType::Add,
		);
		tx.atomic_op(
			&keys::ns::ActorSlotsKey::new(lease.namespace_id, lease.pool_name.clone()),
			&(-1i64).to_le_bytes(),
			MutationType::Add,
		);
	}
	lease.phase = Phase::Sleeping;
	lease.connection_id = None;
	lease.command = None;
	lease.connectable_ts = None;
	lease.sleep_ts = Some(now);
	tx.delete(&keys::actor::ConnectableKey::new(actor_id));
	tx.delete(&keys::actor::EnvoyKeyKey::new(actor_id));
	tx.write(&keys::actor::SleepTsKey::new(actor_id), now)?;
	Ok(())
}

fn persist_command(tx: &universaldb::Transaction, lease: &Lease) -> Result<()> {
	let command = lease.command.as_ref().context("missing lease command")?;
	let actor_id = command.checkpoint.actor_id.parse()?;
	let envoy = lease.envoy_key.clone().context("missing lease owner")?;
	let key = keys::envoy::ActorCommandKey::new(
		lease.namespace_id,
		envoy.clone(),
		actor_id,
		lease.generation,
		command.checkpoint.index,
	);
	let data = match &command.inner {
		protocol::Command::CommandStartActor(c) => {
			protocol::ActorCommandKeyData::CommandStartActor(c.clone())
		}
		protocol::Command::CommandStopActor(c) => {
			protocol::ActorCommandKeyData::CommandStopActor(c.clone())
		}
	};
	for (i, bytes) in key.split(data)?.into_iter().enumerate() {
		tx.set(&tx.pack(&key.chunk(i)), &bytes);
	}
	tx.write(
		&keys::envoy::ActorLastCommandIdxKey::new(
			lease.namespace_id,
			envoy,
			actor_id,
			lease.generation,
		),
		command.checkpoint.index,
	)?;
	Ok(())
}

async fn publish(
	ctx: &OperationCtx,
	namespace_id: Id,
	envoy: &str,
	message: protocol::ToEnvoyConn,
) -> Result<()> {
	let bytes = versioned::ToEnvoyConn::wrap_latest(message)
		.serialize_with_embedded_version(protocol::PROTOCOL_VERSION)?;
	ctx.ups()?
		.publish(
			&crate::pubsub_subjects::EnvoyReceiverSubject::new(namespace_id, envoy.to_owned()),
			&bytes,
			PublishOpts::one(),
		)
		.await?;
	Ok(())
}

/// The entire recoverable transition. A caller may disappear immediately after this returns;
/// the next acquire recovers the command from the committed lease.
pub async fn commit(
	config: &rivet_config::Config,
	pools: &rivet_pools::PoolsHandle,
	input: &Input,
) -> Result<Lease> {
	pools
		.udb()?
		.txn("pegboard_actor_lease", |tx| async move {
			let tx = tx.with_subspace(keys::subspace());
			let mut lease = read(&tx, input.actor_id)
				.await?
				.context("actor lease not initialized")?;
			let original = lease.clone().serialize_with_embedded_version(2)?;
			let now = util::timestamp::now();
			match &input.action {
				Action::Read => return Ok(lease),
				Action::Acquire => {
					ensure!(lease.destroy_ts.is_none(), "actor destroyed");
					if let Some(envoy) = lease.envoy_key.clone() {
						let last_ping = tx
							.read_opt(
								&keys::envoy::LastPingTsKey::new(lease.namespace_id, envoy.clone()),
								Serializable,
							)
							.await?;
						let connection = tx
							.read_opt(
								&keys::envoy::EnvoyConnIdKey::new(
									lease.namespace_id,
									envoy.clone(),
								),
								Serializable,
							)
							.await?;
						// Fail closed on changed registration: no proof the old incarnation stopped.
						ensure!(
							connection == lease.connection_id,
							"lease owner registration changed without confirmed stop"
						);
						let lost_window = config.pegboard().envoy_ping_timeout()
							+ config.pegboard().envoy_lost_threshold()
							+ config.pegboard().actor_stop_threshold();
						if last_ping.is_some_and(|ping| now - ping > lost_window) {
							// Serializes against heartbeat renewal. An expired connection cannot revive.
							tx.write(
								&keys::envoy::ExpiredTsKey::new(lease.namespace_id, envoy),
								now,
							)?;
							release(&tx, input.actor_id, &mut lease, now)?;
						}
					}
					if lease.phase == Phase::Sleeping {
						let envoy = crate::workflows::actor2::alloc_serverful::allocate_serverful(
							lease.namespace_id,
							&lease.pool_name,
							&tx,
							pools,
							now,
							config.pegboard().envoy_eligible_threshold(),
							&config.pegboard().envoy_load_balancer(),
						)
						.await?
						.context("no eligible envoy for lease")?;
						let connection_key =
							keys::envoy::EnvoyConnIdKey::new(lease.namespace_id, envoy.clone());
						let ping_key =
							keys::envoy::LastPingTsKey::new(lease.namespace_id, envoy.clone());
						let expired_key =
							keys::envoy::ExpiredTsKey::new(lease.namespace_id, envoy.clone());
						let (connection, ping, expired) = tokio::try_join!(
							tx.read_opt(&connection_key, Serializable),
							tx.read_opt(&ping_key, Serializable),
							tx.exists(&expired_key, Serializable),
						)?;
						ensure!(
							!expired
								&& connection.is_some() && ping.is_some_and(
								|p| now - p < config.pegboard().envoy_eligible_threshold()
							),
							"selected envoy is no longer eligible"
						);
						lease.generation = lease
							.generation
							.checked_add(1)
							.context("actor generation exhausted")?;
						lease.envoy_key = Some(envoy.clone());
						lease.connection_id = connection;
						lease.protocol_version = tx
							.read(
								&keys::envoy::ProtocolVersionKey::new(
									lease.namespace_id,
									envoy.clone(),
								),
								Serializable,
							)
							.await?;
						lease.phase = Phase::Starting;
						lease.last_event = -1;
						lease.sleep_ts = None;
						lease.connectable_ts = None;
						lease.command = Some(protocol::CommandWrapper {
							checkpoint: protocol::ActorCheckpoint {
								actor_id: input.actor_id.to_string(),
								generation: lease.generation,
								index: 1,
							},
							inner: protocol::Command::CommandStartActor(
								protocol::CommandStartActor {
									config: lease.config.clone(),
									hibernating_requests: Vec::new(),
									preloaded_kv: None,
									sqlite_fence: None,
									sqlite_startup: None,
									waiting_requests: Vec::new(),
								},
							),
						});
						tx.write(
							&keys::actor::GenerationKey::new(input.actor_id),
							lease.generation,
						)?;
						tx.write(
							&keys::actor::EnvoyKeyKey::new(input.actor_id),
							envoy.clone(),
						)?;
						tx.delete(&keys::actor::SleepTsKey::new(input.actor_id));
						tx.write(
							&keys::envoy::ActorKey::new(
								lease.namespace_id,
								envoy.clone(),
								input.actor_id,
							),
							lease.generation,
						)?;
						tx.atomic_op(
							&keys::envoy::SlotsKey::new(lease.namespace_id, envoy),
							&1i64.to_le_bytes(),
							MutationType::Add,
						);
						tx.atomic_op(
							&keys::ns::ActorSlotsKey::new(
								lease.namespace_id,
								lease.pool_name.clone(),
							),
							&1i64.to_le_bytes(),
							MutationType::Add,
						);
						persist_command(&tx, &lease)?;
					}
				}
				Action::Sleep | Action::Destroy => {
					if matches!(input.action, Action::Destroy) {
						lease.destroy_ts.get_or_insert(now);
					}
					stop(&tx, input.actor_id, &mut lease)?;
				}
				Action::Events {
					namespace_id,
					envoy_key,
					connection_id,
					events,
				} => {
					ensure!(
						*namespace_id == lease.namespace_id,
						"actor namespace mismatch"
					);
					if lease.envoy_key.as_ref() != Some(envoy_key)
						|| lease.connection_id != Some(*connection_id)
					{
						return Ok(lease);
					}
					let current = tx
						.read_opt(
							&keys::envoy::EnvoyConnIdKey::new(*namespace_id, envoy_key.clone()),
							Serializable,
						)
						.await?;
					ensure!(current == Some(*connection_id), "stale envoy connection");
					for event in events {
						ensure!(
							event.checkpoint.actor_id == input.actor_id.to_string(),
							"event actor mismatch"
						);
						if event.checkpoint.generation != lease.generation
							|| event.checkpoint.index <= lease.last_event
						{
							continue;
						}
						lease.last_event = event.checkpoint.index;
						match &event.inner {
							protocol::Event::EventActorStateUpdate(e) => match &e.state {
								protocol::ActorState::ActorStateRunning
									if lease.phase == Phase::Starting =>
								{
									lease.phase = Phase::Running;
									lease.start_ts.get_or_insert(now);
									lease.connectable_ts = Some(now);
									lease.command = None;
									tx.write(
										&keys::actor::ConnectableKey::new(input.actor_id),
										(),
									)?;
								}
								protocol::ActorState::ActorStateStopped(_) => {
									release(&tx, input.actor_id, &mut lease, now)?
								}
								_ => {}
							},
							protocol::Event::EventActorIntent(_) => {
								stop(&tx, input.actor_id, &mut lease)?
							}
							protocol::Event::EventActorSetAlarm(e) => lease.alarm_ts = e.alarm_ts,
						}
					}
				}
			}
			if lease.clone().serialize_with_embedded_version(2)? != original {
				write(&tx, input.actor_id, &lease)?;
			}
			Ok(lease)
		})
		.await
}

#[cfg(test)]
#[path = "../../tests/inline/actor_lease_codec.rs"]
mod lease_codec_tests;

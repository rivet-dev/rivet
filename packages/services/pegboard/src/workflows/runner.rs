use futures_util::{FutureExt, StreamExt, TryStreamExt};
use gas::prelude::*;
use rivet_key_data::{
	converted::{ActorNameKeyData, MetadataKeyData, RunnerByKeyKeyData},
	generated::pegboard_runner_address_v1::Data as AddressKeyData,
};
use rivet_runner_protocol::protocol;
use udb_util::{FormalChunkedKey, SERIALIZABLE, SNAPSHOT, TxnExt};
use universaldb::{
	self as udb,
	options::{ConflictRangeType, StreamingMode},
};

use crate::{keys, workflows::actor::Allocate};

/// How long after last ping before considering a runner ineligible for allocation.
pub const RUNNER_ELIGIBLE_THRESHOLD_MS: i64 = util::duration::seconds(10);
/// How long to wait after last ping before forcibly removing a runner from the database and deleting its
/// workflow, evicting all actors. Note that the runner may still be running and can reconnect.
const RUNNER_LOST_THRESHOLD_MS: i64 = util::duration::minutes(2);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Input {
	pub runner_id: Id,
	pub namespace_id: Id,
	pub name: String,
	pub key: String,
	pub version: u32,
	pub total_slots: u32,

	pub addresses_http: util::serde::HashableMap<String, protocol::RunnerAddressHttp>,
	pub addresses_tcp: util::serde::HashableMap<String, protocol::RunnerAddressTcp>,
	pub addresses_udp: util::serde::HashableMap<String, protocol::RunnerAddressUdp>,
}

#[derive(Debug, Serialize, Deserialize)]
struct State {
	namespace_id: Id,
	create_ts: i64,

	last_event_idx: i64,
	last_command_idx: i64,
	commands: Vec<CommandRow>,
	// events: Vec<EventRow>,
}

impl State {
	fn new(namespace_id: Id, create_ts: i64) -> Self {
		State {
			namespace_id,
			create_ts,
			last_event_idx: -1,
			last_command_idx: -1,
			commands: Vec::new(),
		}
	}
}

#[derive(Debug, Serialize, Deserialize)]
struct CommandRow {
	index: i64,
	command: protocol::Command,
	create_ts: i64,
}

#[workflow]
pub async fn pegboard_runner(ctx: &mut WorkflowCtx, input: &Input) -> Result<()> {
	let init_res = ctx
		.activity(InitInput {
			runner_id: input.runner_id,
			name: input.name.clone(),
			key: input.key.clone(),
			namespace_id: input.namespace_id,
			create_ts: ctx.create_ts(),
			addresses_http: input.addresses_http.clone(),
			addresses_tcp: input.addresses_tcp.clone(),
			addresses_udp: input.addresses_udp.clone(),
		})
		.await?;

	// Evict other workflow if there was a key conflict
	if let Some(evict_workflow_id) = init_res.evict_workflow_id {
		ctx.signal(protocol::ToServer::Stopping)
			.to_workflow_id(evict_workflow_id)
			.send()
			.await?;
	}

	ctx.loope(LifecycleState::new(), |ctx, state| {
		let input = input.clone();

		async move {
			match ctx
				.listen_with_timeout::<Main>(RUNNER_LOST_THRESHOLD_MS)
				.await?
			{
				Some(Main::Forward(sig)) => {
					match sig {
						protocol::ToServer::Init {
							last_command_idx,
							prepopulate_actor_names,
							metadata,
							..
						} => {
							let init_data = ctx
								.activity(ProcessInitInput {
									runner_id: input.runner_id,
									namespace_id: input.namespace_id,
									last_command_idx: last_command_idx.unwrap_or(-1),
									prepopulate_actor_names,
									metadata,
								})
								.await?;

							// Send init packet
							ctx.msg(ToWs {
								runner_id: input.runner_id,
								inner: protocol::ToClient::Init {
									runner_id: input.runner_id,
									last_event_idx: init_data.last_event_idx,
									metadata: protocol::ProtocolMetadata {
										runner_lost_threshold: RUNNER_LOST_THRESHOLD_MS,
									},
								},
							})
							.send()
							.await?;

							// Send missed commands
							if !init_data.missed_commands.is_empty() {
								ctx.msg(ToWs {
									runner_id: input.runner_id,
									inner: protocol::ToClient::Commands(init_data.missed_commands),
								})
								.send()
								.await?;
							}

							if !state.draining {
								ctx.activity(InsertFdbInput {
									runner_id: input.runner_id,
									namespace_id: input.namespace_id,
									name: input.name.clone(),
									key: input.key.clone(),
									version: input.version,
									total_slots: input.total_slots,
									create_ts: ctx.create_ts(),
								})
								.await?;
							}

							let res = ctx
								.activity(AllocatePendingActorsInput {
									namespace_id: input.namespace_id,
									name: input.name.clone(),
								})
								.await?;

							// Dispatch pending allocs
							for alloc in res.allocations {
								ctx.signal(alloc.signal)
									.to_workflow::<crate::workflows::actor::Workflow>()
									.tag("actor_id", alloc.actor_id)
									.send()
									.await?;
							}
						}
						protocol::ToServer::Events(events) => {
							let last_event_idx = events.last().map(|event| event.index);

							// NOTE: This should not be parallelized because signals should be sent in order
							// Forward to actor workflows
							for event in events {
								let actor_id = event.inner.actor_id();
								let res = ctx
									.signal(event.inner)
									.to_workflow::<crate::workflows::actor::Workflow>()
									.tag("actor_id", actor_id)
									.send()
									.await;

								if let Some(WorkflowError::WorkflowNotFound) =
									res.as_ref().err().and_then(|x| {
										x.chain().find_map(|x| x.downcast_ref::<WorkflowError>())
									}) {
									tracing::warn!(
										?actor_id,
										"actor workflow not found, likely already stopped"
									);
								} else {
									res?;
								}
							}

							// Ack every 500 events
							if let Some(last_event_idx) = last_event_idx {
								if last_event_idx > state.last_event_ack_idx.saturating_add(500) {
									state.last_event_ack_idx = last_event_idx;

									ctx.msg(ToWs {
										runner_id: input.runner_id,
										inner: protocol::ToClient::AckEvents {
											last_event_idx: state.last_event_ack_idx,
										},
									})
									.send()
									.await?;
								}
							}
						}
						protocol::ToServer::AckCommands { last_command_idx } => {
							ctx.activity(AckCommandsInput { last_command_idx }).await?;
						}
						protocol::ToServer::Stopping => {
							// The workflow will enter a draining state where it can still process signals if
							// needed. After RUNNER_LOST_THRESHOLD_MS it will exit this loop and stop.
							state.draining = true;

							// Can't parallelize these two, requires reading from state
							ctx.activity(ClearFdbInput {
								runner_id: input.runner_id,
								name: input.name.clone(),
								key: input.key.clone(),
								update_state: RunnerState::Draining,
							})
							.await?;

							let actors = ctx
								.activity(FetchRemainingActorsInput {
									runner_id: input.runner_id,
								})
								.await?;

							// Set all remaining actors to lost immediately and send stop commands to the
							// runner. We do both so that the actor's reschedule immediately and the runner is
							// informed that the actors should be stopped (if it is still connected)
							if !actors.is_empty() {
								for (actor_id, generation) in &actors {
									ctx.signal(crate::workflows::actor::Lost {
										generation: *generation,
									})
									.to_workflow::<crate::workflows::actor::Workflow>()
									.tag("actor_id", actor_id)
									.send()
									.await?;
								}

								let commands = actors
									.into_iter()
									.map(|(actor_id, generation)| protocol::Command::StopActor {
										actor_id,
										generation,
									})
									.collect::<Vec<_>>();

								let index = ctx
									.activity(InsertCommandsInput {
										commands: commands.clone(),
									})
									.await?;

								ctx.msg(ToWs {
									runner_id: input.runner_id,
									inner: protocol::ToClient::Commands(
										commands
											.into_iter()
											.enumerate()
											.map(|(i, cmd)| protocol::CommandWrapper {
												index: index + i as i64,
												inner: cmd,
											})
											.collect(),
									),
								})
								.send()
								.await?;
							}
						}
					}
				}
				Some(Main::Command(command)) => {
					// If draining, ignore start actor command and inform the actor wf that it is lost
					if let (
						protocol::Command::StartActor {
							actor_id,
							generation,
							..
						},
						true,
					) = (&command, state.draining)
					{
						tracing::warn!(?actor_id, "attempt to schedule actor to draining runner");

						let res = ctx
							.signal(crate::workflows::actor::Lost {
								generation: *generation,
							})
							.to_workflow::<crate::workflows::actor::Workflow>()
							.tag("actor_id", actor_id)
							.send()
							.await;

						if let Some(WorkflowError::WorkflowNotFound) = res
							.as_ref()
							.err()
							.and_then(|x| x.chain().find_map(|x| x.downcast_ref::<WorkflowError>()))
						{
							tracing::warn!(
								?actor_id,
								"actor workflow not found, likely already stopped"
							);
						} else {
							res?;
						}
					} else {
						let index = ctx
							.activity(InsertCommandsInput {
								commands: vec![command.clone()],
							})
							.await?;

						// Forward
						ctx.msg(ToWs {
							runner_id: input.runner_id,
							inner: protocol::ToClient::Commands(vec![protocol::CommandWrapper {
								index,
								inner: command,
							}]),
						})
						.send()
						.await?;
					}
				}
				Some(Main::CheckQueue(_)) => {
					// Check for pending actors
					let res = ctx
						.activity(AllocatePendingActorsInput {
							namespace_id: input.namespace_id,
							name: input.name.clone(),
						})
						.await?;

					// Dispatch pending allocs
					for alloc in res.allocations {
						ctx.signal(alloc.signal)
							.to_workflow::<crate::workflows::actor::Workflow>()
							.tag("actor_id", alloc.actor_id)
							.send()
							.await?;
					}
				}
				None => {
					if state.draining
						|| ctx
							.activity(CheckExpiredInput {
								runner_id: input.runner_id,
							})
							.await?
					{
						return Ok(Loop::Break(()));
					}
				}
			}

			Ok(Loop::Continue)
		}
		.boxed()
	})
	.await?;

	ctx.activity(ClearFdbInput {
		runner_id: input.runner_id,
		name: input.name.clone(),
		key: input.key.clone(),
		update_state: RunnerState::Stopped,
	})
	.await?;

	let actors = ctx
		.activity(FetchRemainingActorsInput {
			runner_id: input.runner_id,
		})
		.await?;

	// Set all remaining actors as lost
	for (actor_id, generation) in actors {
		let res = ctx
			.signal(crate::workflows::actor::Lost { generation })
			.to_workflow::<crate::workflows::actor::Workflow>()
			.tag("actor_id", actor_id)
			.send()
			.await;

		if let Some(WorkflowError::WorkflowNotFound) = res
			.as_ref()
			.err()
			.and_then(|x| x.chain().find_map(|x| x.downcast_ref::<WorkflowError>()))
		{
			tracing::warn!(
				?actor_id,
				"actor workflow not found, likely already stopped"
			);
		} else {
			res?;
		}
	}

	// Close websocket connection (its unlikely to be open)
	ctx.msg(CloseWs {
		runner_id: input.runner_id,
	})
	.send()
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct LifecycleState {
	draining: bool,
	last_event_ack_idx: i64,
}

impl LifecycleState {
	fn new() -> Self {
		LifecycleState {
			draining: false,
			last_event_ack_idx: -1,
		}
	}
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct InitInput {
	runner_id: Id,
	name: String,
	key: String,
	namespace_id: Id,
	create_ts: i64,

	addresses_http: util::serde::HashableMap<String, protocol::RunnerAddressHttp>,
	addresses_tcp: util::serde::HashableMap<String, protocol::RunnerAddressTcp>,
	addresses_udp: util::serde::HashableMap<String, protocol::RunnerAddressUdp>,
}

#[derive(Debug, Serialize, Deserialize)]
struct InitOutput {
	/// The workflow id of another runner that has the same key.
	evict_workflow_id: Option<Id>,
}

#[activity(Init)]
async fn init(ctx: &ActivityCtx, input: &InitInput) -> Result<InitOutput> {
	let mut state = ctx.state::<Option<State>>()?;

	*state = Some(State::new(input.namespace_id, input.create_ts));

	let evict_workflow_id = ctx
		.udb()?
		.run(|tx, _mc| async move {
			let txs = tx.subspace(keys::subspace());

			for (name, port_http) in &input.addresses_http {
				txs.write(
					&keys::runner::AddressKey::new(input.runner_id, name.into()),
					AddressKeyData::Http(port_http.clone().into()),
				)?;
			}

			for (name, port_tcp) in &input.addresses_tcp {
				txs.write(
					&keys::runner::AddressKey::new(input.runner_id, name.into()),
					AddressKeyData::Tcp(port_tcp.clone().into()),
				)?;
			}

			for (name, port_udp) in &input.addresses_udp {
				txs.write(
					&keys::runner::AddressKey::new(input.runner_id, name.into()),
					AddressKeyData::Udp(port_udp.clone().into()),
				)?;
			}

			let runner_by_key_key = keys::ns::RunnerByKeyKey::new(
				input.namespace_id,
				input.name.clone(),
				input.key.clone(),
			);

			// Read existing runner by key slot
			let evict_workflow_id = txs
				.read_opt(&runner_by_key_key, SERIALIZABLE)
				.await?
				.map(|x| x.workflow_id);

			// Allocate self
			txs.write(
				&runner_by_key_key,
				RunnerByKeyKeyData {
					runner_id: input.runner_id,
					workflow_id: ctx.workflow_id(),
				},
			)?;

			Ok(evict_workflow_id)
		})
		.await?;

	Ok(InitOutput { evict_workflow_id })
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct InsertFdbInput {
	runner_id: Id,
	namespace_id: Id,
	name: String,
	key: String,
	version: u32,
	total_slots: u32,
	create_ts: i64,
}

#[activity(InsertFdb)]
async fn insert_fdb(ctx: &ActivityCtx, input: &InsertFdbInput) -> Result<()> {
	ctx.udb()?
		.run(|tx, _mc| async move {
			let txs = tx.subspace(keys::subspace());

			let remaining_slots_key = keys::runner::RemainingSlotsKey::new(input.runner_id);
			let last_ping_ts_key = keys::runner::LastPingTsKey::new(input.runner_id);
			let workflow_id_key = keys::runner::WorkflowIdKey::new(input.runner_id);

			let (remaining_slots_entry, last_ping_ts_entry) = tokio::try_join!(
				txs.read_opt(&remaining_slots_key, SERIALIZABLE),
				txs.read_opt(&last_ping_ts_key, SERIALIZABLE),
			)?;
			let now = util::timestamp::now();

			// See if key already exists
			let existing = if let (Some(remaining_slots), Some(last_ping_ts)) =
				(remaining_slots_entry, last_ping_ts_entry)
			{
				Some((remaining_slots, last_ping_ts))
			} else {
				// Initial insert
				None
			};

			let (remaining_slots, last_ping_ts) = if let Some(existing) = existing {
				existing
			}
			// NOTE: These properties are only inserted once
			else {
				txs.write(&workflow_id_key, ctx.workflow_id())?;

				txs.write(
					&keys::runner::NamespaceIdKey::new(input.runner_id),
					input.namespace_id,
				)?;

				txs.write(
					&keys::runner::NameKey::new(input.runner_id),
					input.name.clone(),
				)?;

				txs.write(
					&keys::runner::KeyKey::new(input.runner_id),
					input.key.clone(),
				)?;

				txs.write(
					&keys::runner::VersionKey::new(input.runner_id),
					input.version,
				)?;

				txs.write(&remaining_slots_key, input.total_slots)?;

				txs.write(
					&keys::runner::TotalSlotsKey::new(input.runner_id),
					input.total_slots,
				)?;

				txs.write(
					&keys::runner::CreateTsKey::new(input.runner_id),
					input.create_ts,
				)?;

				txs.write(&last_ping_ts_key, now)?;

				// Populate ns indexes
				txs.write(
					&keys::ns::ActiveRunnerKey::new(
						input.namespace_id,
						input.create_ts,
						input.runner_id,
					),
					ctx.workflow_id(),
				)?;
				txs.write(
					&keys::ns::ActiveRunnerByNameKey::new(
						input.namespace_id,
						input.name.clone(),
						input.create_ts,
						input.runner_id,
					),
					ctx.workflow_id(),
				)?;
				txs.write(
					&keys::ns::AllRunnerKey::new(
						input.namespace_id,
						input.create_ts,
						input.runner_id,
					),
					ctx.workflow_id(),
				)?;
				txs.write(
					&keys::ns::AllRunnerByNameKey::new(
						input.namespace_id,
						input.name.clone(),
						input.create_ts,
						input.runner_id,
					),
					ctx.workflow_id(),
				)?;

				// Write name into namespace runner names list
				txs.write(
					&keys::ns::RunnerNameKey::new(input.namespace_id, input.name.clone()),
					(),
				)?;

				(input.total_slots, now)
			};

			// Set last connect ts
			txs.write(&keys::runner::ConnectedTsKey::new(input.runner_id), now)?;

			let remaining_millislots = (remaining_slots * 1000) / input.total_slots;

			// Insert into index (same as the `update_alloc_idx` op with `AddIdx`)
			txs.write(
				&keys::datacenter::RunnerAllocIdxKey::new(
					input.namespace_id,
					input.name.clone(),
					input.version,
					remaining_millislots,
					last_ping_ts,
					input.runner_id,
				),
				rivet_key_data::converted::RunnerAllocIdxKeyData {
					workflow_id: ctx.workflow_id(),
					remaining_slots,
					total_slots: input.total_slots,
				},
			)?;

			Ok(())
		})
		.custom_instrument(tracing::info_span!("runner_insert_tx"))
		.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ClearFdbInput {
	runner_id: Id,
	name: String,
	key: String,
	update_state: RunnerState,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
enum RunnerState {
	Draining,
	Stopped,
}

#[activity(ClearFdb)]
async fn clear_fdb(ctx: &ActivityCtx, input: &ClearFdbInput) -> Result<()> {
	let state = ctx.state::<State>()?;
	let namespace_id = state.namespace_id;
	let create_ts = state.create_ts;

	// TODO: Combine into a single fdb txn
	ctx.udb()?
		.run(|tx, _mc| async move {
			let txs = tx.subspace(keys::subspace());
			let now = util::timestamp::now();

			// Clear runner by key idx if its still the current runner
			let runner_by_key_key =
				keys::ns::RunnerByKeyKey::new(namespace_id, input.name.clone(), input.key.clone());
			let runner_id = txs
				.read_opt(&runner_by_key_key, SERIALIZABLE)
				.await?
				.map(|x| x.runner_id);
			if runner_id == Some(input.runner_id) {
				txs.delete(&runner_by_key_key);
			}

			match input.update_state {
				RunnerState::Draining => {
					txs.write(&keys::runner::DrainTsKey::new(input.runner_id), now)?;
					txs.write(&keys::runner::ExpiredTsKey::new(input.runner_id), now)?;
				}
				RunnerState::Stopped => {
					txs.write(&keys::runner::StopTsKey::new(input.runner_id), now)?;

					// Update namespace indexes
					txs.delete(&keys::ns::ActiveRunnerKey::new(
						namespace_id,
						create_ts,
						input.runner_id,
					));
					txs.delete(&keys::ns::ActiveRunnerByNameKey::new(
						namespace_id,
						input.name.clone(),
						create_ts,
						input.runner_id,
					));
				}
			}

			Ok(())
		})
		.await?;

	// Does not clear the data keys like last ping ts, just the allocation idx
	ctx.op(crate::ops::runner::update_alloc_idx::Input {
		runners: vec![crate::ops::runner::update_alloc_idx::Runner {
			runner_id: input.runner_id,
			action: crate::ops::runner::update_alloc_idx::Action::ClearIdx,
		}],
	})
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ProcessInitInput {
	runner_id: Id,
	namespace_id: Id,
	last_command_idx: i64,
	prepopulate_actor_names: Option<util::serde::HashableMap<String, protocol::ActorName>>,
	metadata: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProcessInitOutput {
	last_event_idx: i64,
	missed_commands: Vec<protocol::CommandWrapper>,
}

#[activity(ProcessInit)]
async fn process_init(ctx: &ActivityCtx, input: &ProcessInitInput) -> Result<ProcessInitOutput> {
	let state = ctx.state::<State>()?;

	ctx.udb()?
		.run(|tx, _mc| async move {
			let txs = tx.subspace(keys::subspace());

			// Populate actor names if provided
			if let Some(actor_names) = &input.prepopulate_actor_names {
				// Write each actor name into the namespace actor names list
				for (name, data) in actor_names {
					let metadata =
						serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(
							&data.metadata,
						)
						.unwrap_or_default();

					txs.write(
						&keys::ns::ActorNameKey::new(input.namespace_id, name.clone()),
						ActorNameKeyData { metadata },
					)?;
				}
			}

			if let Some(metadata) = &input.metadata {
				let metadata = MetadataKeyData {
					metadata: serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(
						&metadata,
					)
					.unwrap_or_default(),
				};

				let metadata_key = keys::runner::MetadataKey::new(input.runner_id);

				// Write metadata
				for (i, chunk) in metadata_key
					.split(metadata)
					.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?
					.into_iter()
					.enumerate()
				{
					let chunk_key = metadata_key.chunk(i);

					txs.set(&txs.pack(&chunk_key), &chunk);
				}
			}

			Ok(())
		})
		.custom_instrument(tracing::info_span!("runner_populate_actor_names_tx"))
		.await?;

	Ok(ProcessInitOutput {
		last_event_idx: state.last_event_idx,
		missed_commands: state
			.commands
			.iter()
			.filter(|row| row.index > input.last_command_idx)
			.map(|row| protocol::CommandWrapper {
				index: row.index,
				inner: row.command.clone(),
			})
			.collect(),
	})
}

// TODO: Added while sqlite flushing system is in place. As the database grows, flushes get slower
// and slower.
#[derive(Debug, Serialize, Deserialize, Hash)]
struct AckCommandsInput {
	last_command_idx: i64,
}

#[activity(AckCommands)]
async fn ack_commands(ctx: &ActivityCtx, input: &AckCommandsInput) -> Result<()> {
	let mut state = ctx.state::<State>()?;

	state
		.commands
		.retain(|row| row.index > input.last_command_idx);

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct InsertEventsInput {
	runner_id: Id,
	events: Vec<protocol::EventWrapper>,
}

#[activity(InsertEvents)]
async fn insert_events(ctx: &ActivityCtx, input: &InsertEventsInput) -> Result<()> {
	let last_event_idx = if let Some(last_event_wrapper) = input.events.last() {
		last_event_wrapper.index
	} else {
		return Ok(());
	};

	let mut state = ctx.state::<State>()?;

	// TODO: Storing events is disabled for now, otherwise state will grow indefinitely. This is only used
	// for debugging anyway
	// state.events.extend(input.events.into_iter().enumerate().map(|(i, event)| EventRow {
	// 	index: event.index,
	// 	event: event.inner,
	// 	ack_ts: util::timestamp::now(),
	// }));

	state.last_event_idx = last_event_idx;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct InsertCommandsInput {
	commands: Vec<protocol::Command>,
}

#[activity(InsertCommands)]
async fn insert_commands(ctx: &ActivityCtx, input: &InsertCommandsInput) -> Result<i64> {
	let mut state = ctx.state::<State>()?;

	let last_command_idx = state.last_command_idx;
	state.commands.extend(
		input
			.commands
			.iter()
			.enumerate()
			.map(|(i, command)| CommandRow {
				index: last_command_idx + i as i64 + 1,
				command: command.clone(),
				create_ts: util::timestamp::now(),
			}),
	);

	let old = state.last_command_idx;
	state.last_command_idx += input.commands.len() as i64;

	Ok(old + 1)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct FetchRemainingActorsInput {
	runner_id: Id,
}

#[activity(FetchRemainingActors)]
async fn fetch_remaining_actors(
	ctx: &ActivityCtx,
	input: &FetchRemainingActorsInput,
) -> Result<Vec<(Id, u32)>> {
	let actors = ctx
		.udb()?
		.run(|tx, _mc| async move {
			let txs = tx.subspace(keys::subspace());

			let actor_subspace =
				keys::subspace().subspace(&keys::runner::ActorKey::subspace(input.runner_id));

			tx.get_ranges_keyvalues(
				udb::RangeOption {
					mode: StreamingMode::WantAll,
					..(&actor_subspace).into()
				},
				SERIALIZABLE,
			)
			.map(|res| match res {
				Ok(entry) => {
					let (key, generation) = txs.read_entry::<keys::runner::ActorKey>(&entry)?;

					Ok((key.actor_id.into(), generation))
				}
				Err(err) => Err(Into::<udb::FdbBindingError>::into(err)),
			})
			.try_collect::<Vec<_>>()
			.await
		})
		.custom_instrument(tracing::info_span!("runner_fetch_remaining_actors_tx"))
		.await?;

	Ok(actors)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct CheckExpiredInput {
	runner_id: Id,
}

#[activity(CheckExpired)]
async fn check_expired(ctx: &ActivityCtx, input: &CheckExpiredInput) -> Result<bool> {
	ctx.udb()?
		.run(|tx, _mc| async move {
			let txs = tx.subspace(keys::subspace());

			let last_ping_ts = txs
				.read(
					&keys::runner::LastPingTsKey::new(input.runner_id),
					SERIALIZABLE,
				)
				.await?;

			let now = util::timestamp::now();
			let expired = last_ping_ts < now - RUNNER_LOST_THRESHOLD_MS;

			if expired {
				txs.write(&keys::runner::ExpiredTsKey::new(input.runner_id), now)?;
			}

			Ok(expired)
		})
		.custom_instrument(tracing::info_span!("runner_check_expired_tx"))
		.await
		.map_err(Into::into)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub(crate) struct AllocatePendingActorsInput {
	pub namespace_id: Id,
	pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct AllocatePendingActorsOutput {
	pub allocations: Vec<ActorAllocation>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ActorAllocation {
	pub actor_id: Id,
	pub signal: Allocate,
}

#[activity(AllocatePendingActors)]
pub(crate) async fn allocate_pending_actors(
	ctx: &ActivityCtx,
	input: &AllocatePendingActorsInput,
) -> Result<AllocatePendingActorsOutput> {
	// NOTE: This txn should closely resemble the one found in the allocate_actor activity of the actor wf
	let res = ctx
		.udb()?
		.run(|tx, _mc| async move {
			let txs = tx.subspace(keys::subspace());
			let mut results = Vec::new();

			let pending_actor_subspace = txs.subspace(
				&keys::datacenter::PendingActorByRunnerNameSelectorKey::subspace(
					input.namespace_id,
					input.name.clone(),
				),
			);
			let mut queue_stream = txs.get_ranges_keyvalues(
				udb::RangeOption {
					mode: StreamingMode::Iterator,
					..(&pending_actor_subspace).into()
				},
				// NOTE: This is not SERIALIZABLE because we don't want to conflict with all of the keys, just
				// the one we choose
				SNAPSHOT,
			);
			let ping_threshold_ts = util::timestamp::now() - RUNNER_ELIGIBLE_THRESHOLD_MS;

			'queue_loop: loop {
				let Some(queue_entry) = queue_stream.try_next().await? else {
					break;
				};

				let (queue_key, generation) =
					txs.read_entry::<keys::datacenter::PendingActorByRunnerNameSelectorKey>(
						&queue_entry,
					)?;

				let runner_alloc_subspace =
					txs.subspace(&keys::datacenter::RunnerAllocIdxKey::subspace(
						input.namespace_id,
						input.name.clone(),
					));

				let mut stream = txs.get_ranges_keyvalues(
					udb::RangeOption {
						mode: StreamingMode::Iterator,
						// Containers bin pack so we reverse the order
						reverse: true,
						..(&runner_alloc_subspace).into()
					},
					// NOTE: This is not SERIALIZABLE because we don't want to conflict with all of the
					// keys, just the one we choose
					SNAPSHOT,
				);

				let mut highest_version = None;

				loop {
					let Some(entry) = stream.try_next().await? else {
						break;
					};

					let (old_runner_alloc_key, old_runner_alloc_key_data) =
						txs.read_entry::<keys::datacenter::RunnerAllocIdxKey>(&entry)?;

					if let Some(highest_version) = highest_version {
						// We have passed all of the runners with the highest version. This is reachable if
						// the ping of the highest version workers makes them ineligible
						if old_runner_alloc_key.version < highest_version {
							break;
						}
					} else {
						highest_version = Some(old_runner_alloc_key.version);
					}

					// An empty runner means we have reached the end of the runners with the highest version
					if old_runner_alloc_key.remaining_millislots == 0 {
						break;
					}

					// Scan by last ping
					if old_runner_alloc_key.last_ping_ts < ping_threshold_ts {
						continue;
					}

					// Add read conflict only for this runner key
					txs.add_conflict_key(&old_runner_alloc_key, ConflictRangeType::Read)?;
					txs.delete(&old_runner_alloc_key);

					// Add read conflict for the queue key
					txs.add_conflict_key(&queue_key, ConflictRangeType::Read)?;
					txs.delete(&queue_key);

					let new_remaining_slots =
						old_runner_alloc_key_data.remaining_slots.saturating_sub(1);
					let new_remaining_millislots =
						(new_remaining_slots * 1000) / old_runner_alloc_key_data.total_slots;

					// Write new allocation key with 1 less slot
					txs.write(
						&keys::datacenter::RunnerAllocIdxKey::new(
							input.namespace_id,
							input.name.clone(),
							old_runner_alloc_key.version,
							new_remaining_millislots,
							old_runner_alloc_key.last_ping_ts,
							old_runner_alloc_key.runner_id,
						),
						rivet_key_data::converted::RunnerAllocIdxKeyData {
							workflow_id: old_runner_alloc_key_data.workflow_id,
							remaining_slots: new_remaining_slots,
							total_slots: old_runner_alloc_key_data.total_slots,
						},
					)?;

					// Update runner record
					txs.write(
						&keys::runner::RemainingSlotsKey::new(old_runner_alloc_key.runner_id),
						new_remaining_slots,
					)?;

					// Set runner id of actor
					txs.write(
						&keys::actor::RunnerIdKey::new(queue_key.actor_id),
						old_runner_alloc_key.runner_id,
					)?;

					// Insert actor index key
					txs.write(
						&keys::runner::ActorKey::new(
							old_runner_alloc_key.runner_id,
							queue_key.actor_id,
						),
						generation,
					)?;

					results.push(ActorAllocation {
						actor_id: queue_key.actor_id,
						signal: Allocate {
							runner_id: old_runner_alloc_key.runner_id,
							runner_workflow_id: old_runner_alloc_key_data.workflow_id,
						},
					});
					continue 'queue_loop;
				}
			}

			Ok(results)
		})
		.custom_instrument(tracing::info_span!("runner_allocate_pending_actors_tx"))
		.await?;

	Ok(AllocatePendingActorsOutput { allocations: res })
}

#[message("pegboard_runner_to_ws")]
pub struct ToWs {
	pub runner_id: Id,
	pub inner: protocol::ToClient,
}

#[signal("pegboard_runner_check_queue")]
pub struct CheckQueue {}

#[message("pegboard_runner_close_ws")]
pub struct CloseWs {
	pub runner_id: Id,
}

join_signal!(Main {
	Command(protocol::Command),
	// Forwarded from the ws to this workflow
	Forward(protocol::ToServer),
	CheckQueue,
});

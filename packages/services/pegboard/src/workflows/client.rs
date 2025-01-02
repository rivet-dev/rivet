use std::convert::TryInto;

use chirp_workflow::prelude::*;
use futures_util::FutureExt;
use nix::sys::signal::Signal;

use crate::{metrics, protocol};

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
	pub client_id: Uuid,
}

#[workflow]
pub async fn pegboard_client(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	// Whatever started this client should be listening for this
	ctx.signal(Registered {})
		.tag("client_id", input.client_id)
		.send()
		.await?;

	ctx.repeat(|ctx| {
		let client_id = input.client_id;

		async move {
			match ctx.listen::<Main>().await? {
				Main::Forward(sig) => {
					match sig {
						protocol::ToServer::Init {
							last_command_idx,
							config,
							system,
						} => {
							let init_data = ctx
								.activity(ProcessInitInput {
									client_id,
									config,
									system,
									last_command_idx,
								})
								.await?;

							// Send init packet
							ctx.msg(ToWs {
								client_id,
								inner: protocol::ToClient::Init {
									last_event_idx: init_data.last_event_idx,
								},
							})
							.send()
							.await?;

							// Send missed commands
							if !init_data.missed_commands.is_empty() {
								ctx.msg(ToWs {
									client_id,
									inner: protocol::ToClient::Commands(init_data.missed_commands),
								})
								.send()
								.await?;
							}
						}
						// We assume events are in order by index
						protocol::ToServer::Events(events) => {
							// Write to db
							ctx.activity(InsertEventsInput {
								client_id,
								events: events.clone(),
							})
							.await?;

							ctx.activity(UpdateActorStateInput {
								events: events.iter().map(|x| x.inner.clone()).collect(),
							})
							.await?;

							// NOTE: This should not be parallelized because signals should be sent in order
							for event in events {
								#[allow(irrefutable_let_patterns)]
								if let protocol::Event::ActorStateUpdate { actor_id, state } =
									serde_json::from_str(event.inner.get())?
								{
									ctx.signal(ActorStateUpdate { state })
										.tag("actor_id", actor_id)
										.send()
										.await?;
								}
							}
						}
						protocol::ToServer::FetchStateResponse {} => todo!(),
					}
				}
				Main::Command(command) => {
					handle_commands(ctx, client_id, vec![command]).await?;
				}
				Main::Drain(_) => {
					ctx.activity(SetDrainInput {
						client_id,
						drain: true,
					})
					.await?;
				}
				Main::Undrain(_) => {
					ctx.activity(SetDrainInput {
						client_id,
						drain: false,
					})
					.await?;
				}
				Main::Destroy(_) => return Ok(Loop::Break(())),
			}

			Ok(Loop::Continue)
		}
		.boxed()
	})
	.await?;

	let actor_ids = ctx
		.activity(FetchAllActorsInput {
			client_id: input.client_id,
		})
		.await?;

	// Evict all actors.
	// Note that even if this client is unresponsive and does not process the signal commands, the
	// pegboard-gc service will manually set the actors as closed after 30 seconds.
	handle_commands(
		ctx,
		input.client_id,
		actor_ids
			.into_iter()
			.map(|actor_id| protocol::Command::SignalActor {
				actor_id,
				signal: Signal::SIGKILL as i32,
				persist_state: false,
			})
			.collect(),
	)
	.await?;

	// Close websocket connection
	ctx.msg(CloseWs {
		client_id: input.client_id,
	})
	.send()
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ProcessInitInput {
	client_id: Uuid,
	last_command_idx: i64,
	config: crate::client_config::ClientConfig,
	system: crate::system_info::SystemInfo,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProcessInitOutput {
	last_event_idx: i64,
	missed_commands: Vec<protocol::CommandWrapper>,
}

#[activity(ProcessInit)]
async fn process_init(
	ctx: &ActivityCtx,
	input: &ProcessInitInput,
) -> GlobalResult<ProcessInitOutput> {
	let ((last_event_idx,), commands) = tokio::try_join!(
		sql_fetch_one!(
			[ctx, (i64,)]
			"
			UPDATE db_pegboard.clients
			SET config = $2, system_info = $3
			WHERE client_id = $1
			RETURNING last_event_idx
			",
			input.client_id,
			serde_json::to_value(&input.config)?,
			serde_json::to_value(&input.system)?,
		),
		sql_fetch_all!(
			[ctx, (i64, String)]
			"
			SELECT index, payload::TEXT
			FROM db_pegboard.client_commands
			WHERE client_id = $1 AND index > $2
			ORDER BY index ASC
			",
			input.client_id,
			input.last_command_idx,
		),
	)?;

	Ok(ProcessInitOutput {
		last_event_idx,
		missed_commands: commands
			.into_iter()
			.map(|(index, payload)| {
				Ok(protocol::CommandWrapper {
					index,
					inner: protocol::Raw::from_string(payload)?,
				})
			})
			.collect::<GlobalResult<_>>()?,
	})
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct InsertEventsInput {
	client_id: Uuid,
	events: Vec<protocol::EventWrapper>,
}

#[activity(InsertEvents)]
async fn insert_events(ctx: &ActivityCtx, input: &InsertEventsInput) -> GlobalResult<()> {
	let last_event_idx = if let Some(last_event_wrapper) = input.events.last() {
		last_event_wrapper.index
	} else {
		return Ok(());
	};

	// TODO(RVT-4450): `last_event_idx < $2` and `ON CONFLICT DO NOTHING` is a workaround
	let inserted_rows = sql_fetch_all!(
		[ctx, (i64,)]
		"
		WITH
			update_last_event_idx AS (
				UPDATE db_pegboard.clients
				SET last_event_idx = $2
				WHERE
					client_id = $1 AND
					last_event_idx < $2
				RETURNING 1
			),
			insert_events AS (
				INSERT INTO db_pegboard.client_events (client_id, index, payload, ack_ts)
				SELECT $1, index, payload, $5
				FROM UNNEST($3, $4) AS e(index, payload)
				ON CONFLICT DO NOTHING
				RETURNING index
			)
		SELECT index FROM insert_events
		",
		input.client_id,
		last_event_idx,
		input.events.iter().map(|wrapper| wrapper.index).collect::<Vec<_>>(),
		input.events.iter().map(|wrapper| &wrapper.inner).collect::<Vec<_>>(),
		util::timestamp::now(),
	)
	.await?;

	// TODO(RVT-4450): Check for duplicate events
	for event in &input.events {
		if inserted_rows.iter().all(|(idx,)| &event.index != idx) {
			continue;
		}

		metrics::PEGBOARD_DUPLICATE_CLIENT_EVENT
			.with_label_values(&[&input.client_id.to_string(), &event.index.to_string()])
			.inc();
	}

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateActorStateInput {
	events: Vec<protocol::Raw<protocol::Event>>,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateActorStateOutput {
	stopping_actor_ids: Vec<Uuid>,
}

#[activity(UpdateActorState)]
async fn update_actor_state(
	ctx: &ActivityCtx,
	input: &UpdateActorStateInput,
) -> GlobalResult<UpdateActorStateOutput> {
	use protocol::ActorState::*;

	let mut stopping_actor_ids = Vec::new();

	// TODO: Parallelize
	for event in &input.events {
		// Update actors table with actor state updates
		match event.deserialize()? {
			protocol::Event::ActorStateUpdate { actor_id, state } => match state {
				Starting => {
					sql_execute!(
						[ctx]
						"
						UPDATE db_pegboard.actors
						SET start_ts = $2
						WHERE actor_id = $1
						",
						actor_id,
						util::timestamp::now(),
					)
					.await?;
				}
				Running { pid, .. } => {
					sql_execute!(
						[ctx]
						"
						UPDATE db_pegboard.actors
						SET
							running_ts = $2,
							pid = $3
						WHERE actor_id = $1
						",
						actor_id,
						util::timestamp::now(),
						pid as i64,
					)
					.await?;
				}
				Stopping => {
					let set_stopping_ts = sql_fetch_optional!(
						[ctx, (i64,)]
						"
						UPDATE db_pegboard.actors
						SET stopping_ts = $2
						WHERE actor_id = $1 AND stopping_ts IS NULL
						RETURNING 1
						",
						actor_id,
						util::timestamp::now(),
					)
					.await?
					.is_some();

					if set_stopping_ts {
						stopping_actor_ids.push(actor_id);
					}
				}
				Stopped => {
					sql_execute!(
						[ctx]
						"
						UPDATE db_pegboard.actors
						SET stop_ts = $2
						WHERE actor_id = $1
						",
						actor_id,
						util::timestamp::now(),
					)
					.await?;
				}
				Lost => {
					sql_execute!(
						[ctx]
						"
						UPDATE db_pegboard.actors
						SET
							lost_ts = $2
						WHERE actor_id = $1
						",
						actor_id,
						util::timestamp::now(),
					)
					.await?;
				}
				Exited { exit_code } => {
					sql_execute!(
						[ctx]
						"
						UPDATE db_pegboard.actors
						SET
							exit_ts = $2,
							exit_code = $3
						WHERE actor_id = $1
						",
						actor_id,
						util::timestamp::now(),
						exit_code,
					)
					.await?;
				}
				// These updates should never reach this workflow
				Allocated { .. } | FailedToAllocate => bail!("invalid state for updating db"),
			},
		}
	}

	Ok(UpdateActorStateOutput { stopping_actor_ids })
}

pub async fn handle_commands(
	ctx: &mut WorkflowCtx,
	client_id: Uuid,
	commands: Vec<protocol::Command>,
) -> GlobalResult<()> {
	if commands.is_empty() {
		return Ok(());
	}

	let raw_commands = commands
		.iter()
		.map(protocol::Raw::new)
		.collect::<Result<Vec<_>, _>>()?;

	// Write to db
	let index = ctx
		.activity(InsertCommandsInput {
			client_id,
			commands: raw_commands.clone(),
		})
		.await?;

	// TODO: Send as a single message
	for (i, raw_command) in raw_commands.into_iter().enumerate() {
		let wrapped_command = protocol::CommandWrapper {
			index: index + i as i64,
			inner: raw_command,
		};

		// Forward signal to ws as message
		ctx.msg(ToWs {
			client_id,
			inner: protocol::ToClient::Commands(vec![wrapped_command]),
		})
		.send()
		.await?;
	}

	// Update actor state based on commands
	for command in commands {
		if let protocol::Command::SignalActor {
			actor_id, signal, ..
		} = command
		{
			if matches!(signal.try_into()?, Signal::SIGTERM | Signal::SIGKILL) {
				let res = ctx
					.activity(UpdateActorStateInput {
						events: vec![protocol::Raw::new(&protocol::Event::ActorStateUpdate {
							actor_id,
							state: protocol::ActorState::Stopping,
						})?],
					})
					.await?;

				// Publish signal if stopping_ts was not set before
				if !res.stopping_actor_ids.is_empty() {
					ctx.signal(crate::workflows::client::ActorStateUpdate {
						state: protocol::ActorState::Stopping,
					})
					.tag("actor_id", actor_id)
					.send()
					.await?;
				}
			}
		}
	}

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct InsertCommandsInput {
	client_id: Uuid,
	commands: Vec<protocol::Raw<protocol::Command>>,
}

#[activity(InsertCommands)]
async fn insert_commands(ctx: &ActivityCtx, input: &InsertCommandsInput) -> GlobalResult<i64> {
	let (index,) = sql_fetch_one!(
		[ctx, (i64,)]
		"
		WITH
			last_command_idx(idx) AS (
				UPDATE db_pegboard.clients
				SET last_command_idx = last_command_idx + array_length($2, 1)
				WHERE client_id = $1
				RETURNING last_command_idx - array_length($2, 1)
			),
			insert_commands AS (
				INSERT INTO db_pegboard.client_commands (
					client_id,
					index,
					payload,
					create_ts
				)
				SELECT $1, l.idx + p.index, p.payload, $3
				FROM last_command_idx AS l
				CROSS JOIN UNNEST($2) WITH ORDINALITY AS p(payload, index)
				RETURNING 1
			)
		SELECT idx FROM last_command_idx
		",
		input.client_id,
		&input.commands,
		util::timestamp::now(),
	)
	.await?;

	// Postgres is 1-based
	Ok(index + 1)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct SetDrainInput {
	client_id: Uuid,
	drain: bool,
}

#[activity(SetDrain)]
async fn set_drain(ctx: &ActivityCtx, input: &SetDrainInput) -> GlobalResult<()> {
	sql_execute!(
		[ctx]
		"
		UPDATE db_pegboard.clients
		SET drain_ts = $2
		WHERE client_id = $1
		",
		input.client_id,
		input.drain.then(util::timestamp::now),
	)
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct FetchAllActorsInput {
	client_id: Uuid,
}

#[activity(FetchAllActors)]
async fn fetch_all_actors(
	ctx: &ActivityCtx,
	input: &FetchAllActorsInput,
) -> GlobalResult<Vec<Uuid>> {
	let actor_ids = sql_fetch_all!(
		[ctx, (Uuid,)]
		"
		SELECT actor_id
		FROM db_pegboard.actors
		WHERE
			client_id = $1 AND
			stopping_ts IS NULL AND
			stop_ts IS NULL AND
			exit_ts IS NULL
		",
		input.client_id,
	)
	.await?
	.into_iter()
	.map(|(id,)| id)
	.collect();

	Ok(actor_ids)
}

#[signal("pegboard_client_registered")]
pub struct Registered {}

#[message("pegboard_client_to_ws")]
pub struct ToWs {
	pub client_id: Uuid,
	pub inner: protocol::ToClient,
}

#[message("pegboard_client_close_ws")]
pub struct CloseWs {
	pub client_id: Uuid,
}

#[signal("pegboard_actor_state_update")]
pub struct ActorStateUpdate {
	pub state: protocol::ActorState,
}

#[signal("pegboard_client_drain")]
pub struct Drain {}

#[signal("pegboard_client_undrain")]
pub struct Undrain {}

#[signal("pegboard_client_destroy")]
pub struct Destroy {}

join_signal!(Main {
	Command(protocol::Command),
	// Forwarded from the ws to this workflow
	Forward(protocol::ToServer),
	Drain,
	Undrain,
	Destroy,
});

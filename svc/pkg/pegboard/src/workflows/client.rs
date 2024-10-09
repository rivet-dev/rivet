use std::convert::TryInto;

use chirp_workflow::prelude::*;
use futures_util::FutureExt;
use nix::sys::signal::Signal;
use serde_json::json;

use crate::protocol;

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
	pub client_id: Uuid,
}

#[workflow]
pub async fn pegboard_client(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	ctx.repeat(|ctx| {
		let client_id = input.client_id;

		async move {
			match ctx.listen::<Main>().await? {
				Main::Forward(sig) => {
					match sig {
						protocol::ToServer::Init {
							last_command_idx,
							system,
						} => {
							let init_data = ctx
								.activity(ProcessInitInput {
									client_id,
									system,
									last_command_idx,
								})
								.await?;

							// Send init packet
							ctx.msg(ToWs {
								client_id,
								inner: protocol::ToClient::Init {
									last_event_idx: init_data.last_event_idx,
									api_endpoint: util::env::origin_api().to_string(),
								},
							})
							.tags(json!({}))
							.send()
							.await?;

							// Send missed commands
							if !init_data.missed_commands.is_empty() {
								ctx.msg(ToWs {
									client_id,
									inner: protocol::ToClient::Commands(init_data.missed_commands),
								})
								.tags(json!({}))
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

							ctx.activity(UpdateContainerStateInput {
								events: events.iter().map(|x| x.inner.clone()).collect(),
							})
							.await?;

							// NOTE: This should not be parallelized because signals should be sent in order
							for event in events {
								#[allow(irrefutable_let_patterns)]
								if let protocol::Event::ContainerStateUpdate {
									container_id,
									state,
								} = serde_json::from_str(event.inner.get())?
								{
									ctx.signal(ContainerStateUpdate { state })
										.tag("container_id", container_id)
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

	let container_ids = ctx
		.activity(FetchAllContainersInput {
			client_id: input.client_id,
		})
		.await?;

	// Evict all containers.
	// Note that even if this client is unresponsive and does not process the signal commands, the
	// pegboard-gc service will manually set the containers as closed after 30 seconds.
	handle_commands(
		ctx,
		input.client_id,
		container_ids
			.into_iter()
			.map(|container_id| protocol::Command::SignalContainer {
				container_id,
				signal: Signal::SIGKILL as i32,
			})
			.collect(),
	)
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ProcessInitInput {
	client_id: Uuid,
	last_command_idx: i64,
	system: protocol::SystemInfo,
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
			SET
				cpu = $2 AND
				memory = $3
			WHERE client_id = $1
			RETURNING last_event_idx 
			",
			input.client_id,
			input.system.cpu as i64,
			input.system.memory as i64
		),
		sql_fetch_all!(
			[ctx, (i64, String)]
			"
			SELECT index, payload
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
	// TODO: Parallelize
	for event_wrapper in &input.events {
		sql_execute!(
			[ctx]
			"
			INSERT INTO db_pegboard.client_events (client_id, index, payload, ack_ts)
			VALUES ($1, $2, $3, $4)
			",
			input.client_id,
			event_wrapper.index,
			&event_wrapper.inner,
			util::timestamp::now(),
		)
		.await?;
	}

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateContainerStateInput {
	events: Vec<protocol::Raw<protocol::Event>>,
}

#[activity(UpdateContainerState)]
async fn update_container_state(
	ctx: &ActivityCtx,
	input: &UpdateContainerStateInput,
) -> GlobalResult<()> {
	use protocol::ContainerState::*;

	// TODO: Parallelize
	for event in &input.events {
		// Update containers table with container state updates
		match event.deserialize()? {
			protocol::Event::ContainerStateUpdate {
				container_id,
				state,
			} => match state {
				Starting => {
					sql_execute!(
						[ctx]
						"
						UPDATE db_pegboard.containers
						SET start_ts = $2
						WHERE container_id = $1
						",
						container_id,
						util::timestamp::now(),
					)
					.await?;
				}
				Running { pid, .. } => {
					sql_execute!(
						[ctx]
						"
						UPDATE db_pegboard.containers
						SET
							running_ts = $2 AND
							pid = $3
						WHERE container_id = $1
						",
						container_id,
						util::timestamp::now(),
						pid as i64,
					)
					.await?;
				}
				Stopping => {
					sql_execute!(
						[ctx]
						"
						UPDATE db_pegboard.containers
						SET stopping_ts = $2
						WHERE container_id = $1
						",
						container_id,
						util::timestamp::now(),
					)
					.await?;
				}
				Stopped => {
					sql_execute!(
						[ctx]
						"
						UPDATE db_pegboard.containers
						SET stop_ts = $2
						WHERE container_id = $1
						",
						container_id,
						util::timestamp::now(),
					)
					.await?;
				}
				Exited { exit_code } => {
					sql_execute!(
						[ctx]
						"
						UPDATE db_pegboard.containers
						SET
							exit_ts = $2 AND
							exit_code = $3
						WHERE container_id = $1
						",
						container_id,
						util::timestamp::now(),
						exit_code,
					)
					.await?;
				}
				Allocated { .. } | FailedToAllocate => bail!("invalid state for updating db"),
			},
		}
	}

	Ok(())
}

pub async fn handle_commands(
	ctx: &mut WorkflowCtx,
	client_id: Uuid,
	commands: Vec<protocol::Command>,
) -> GlobalResult<()> {
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
		.tags(json!({}))
		.send()
		.await?;
	}

	// Update container state based on commands
	for command in commands {
		if let protocol::Command::SignalContainer {
			container_id,
			signal,
		} = command
		{
			if let Signal::SIGTERM = signal.try_into()? {
				ctx.activity(UpdateContainerStateInput {
					events: vec![protocol::Raw::new(
						&protocol::Event::ContainerStateUpdate {
							container_id,
							state: protocol::ContainerState::Stopping,
						},
					)?],
				})
				.await?;

				ctx.signal(crate::workflows::client::ContainerStateUpdate {
					state: protocol::ContainerState::Stopping,
				})
				.tag("container_id", container_id)
				.send()
				.await?;
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
			last_idx AS (
				UPDATE db_pegboard.clients
				SET last_command_idx = last_command_idx + 1
				WHERE client_id = $1
				RETURNING last_command_idx - 1
			),
			insert_commands AS (
				INSERT INTO db_pegboard.client_commands (
					client_id,
					payload,
					index,
					create_ts
				)
				SELECT $1, p.payload, last_idx.last_command_idx + p.index - 1, $3
				FROM last_idx
				CROSS JOIN UNNEST($2) WITH ORDINALITY AS p(payload, index)
				RETURNING 1
			)
		SELECT last_event_idx FROM last_idx
		",
		input.client_id,
		&input.commands,
		util::timestamp::now(),
	)
	.await?;

	Ok(index)
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
struct FetchAllContainersInput {
	client_id: Uuid,
}

#[activity(FetchAllContainers)]
async fn fetch_all_containers(
	ctx: &ActivityCtx,
	input: &FetchAllContainersInput,
) -> GlobalResult<Vec<Uuid>> {
	let container_ids = sql_fetch_all!(
		[ctx, (Uuid,)]
		"
		SELECT container_id
		FROM db_pegboard.clients
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

	Ok(container_ids)
}

#[message("pegboard_client_to_ws")]
pub struct ToWs {
	pub client_id: Uuid,
	pub inner: protocol::ToClient,
}

#[signal("pegboard_container_state_update")]
pub struct ContainerStateUpdate {
	pub state: protocol::ContainerState,
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

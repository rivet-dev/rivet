use chirp_workflow::prelude::*;
use futures_util::{FutureExt, StreamExt, TryStreamExt};
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
						protocol::ToServer::Init { last_command_idx } => {
							let init_data = ctx
								.activity(FetchInitDataInput {
									client_id,
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
						protocol::ToServer::Events(events) => {
							// Write to db
							ctx.activity(InsertEventsInput {
								client_id,
								events: events.clone(),
							})
							.await?;

							// TODO: Update containers table with container state updates

							// Re-dispatch container state updates
							futures_util::stream::iter(events.into_iter())
								.map(|event| {
									let mut ctx = ctx.step();

									async move {
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

										GlobalResult::Ok(())
									}
								})
								.buffer_unordered(16)
								.try_collect::<Vec<_>>()
								.await?;
						}
						protocol::ToServer::FetchStateResponse {} => todo!(),
					}
				}
				Main::Command(command) => {
					let raw_command = protocol::Raw::new(&command)?;

					// Write to db
					let index = ctx
						.activity(InsertCommandInput {
							client_id,
							command: raw_command.clone(),
						})
						.await?;

					let wrapped_command = protocol::CommandWrapper {
						index,
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
			}

			Ok(Loop::<()>::Continue)
		}
		.boxed()
	})
	.await?;

	Ok(())
}

// TODO: This does not need to be retryable
#[derive(Debug, Serialize, Deserialize, Hash)]
struct FetchInitDataInput {
	client_id: Uuid,
	last_command_idx: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct FetchInitDataOutput {
	last_event_idx: i64,
	missed_commands: Vec<protocol::CommandWrapper>,
}

#[activity(FetchInitData)]
async fn fetch_init_data(
	ctx: &ActivityCtx,
	input: &FetchInitDataInput,
) -> GlobalResult<FetchInitDataOutput> {
	let ((last_event_idx,), commands) = tokio::try_join!(
		sql_fetch_one!(
			[ctx, (i64,)]
			"
			SELECT last_event_idx
			FROM db_pegboard.clients
			WHERE client_id = $1
			",
			input.client_id,
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

	Ok(FetchInitDataOutput {
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
struct InsertCommandInput {
	client_id: Uuid,
	command: protocol::Raw<protocol::Command>,
}

#[activity(InsertCommand)]
async fn insert_command(ctx: &ActivityCtx, input: &InsertCommandInput) -> GlobalResult<i64> {
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
			insert_command AS (
				INSERT INTO db_pegboard.client_commands (
					client_id,
					payload,
					index,
					create_ts
				)
				SELECT $1, $2, last_idx.last_command_idx, $3
				FROM last_idx
				LIMIT 1
				RETURNING 1
			)
		SELECT last_event_idx FROM last_idx
		",
		input.client_id,
		&input.command,
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

#[message("pegboard_client_to_ws")]
pub struct ToWs {
	pub client_id: Uuid,
	pub inner: protocol::ToClient,
}

#[signal("pegboard_container_state_update")]
struct ContainerStateUpdate {
	pub state: protocol::ContainerState,
}

#[signal("pegboard_client_drain")]
pub struct Drain {}

#[signal("pegboard_client_undrain")]
pub struct Undrain {}

join_signal!(Main {
	Command(protocol::Command),
	// Forwarded from the ws to this workflow
	Forward(protocol::ToServer),
	Drain,
	Undrain,
});

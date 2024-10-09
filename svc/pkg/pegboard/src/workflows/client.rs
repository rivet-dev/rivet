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
						protocol::ToServer::Init {} => todo!(),
						protocol::ToServer::Events(events) => {
							// Write to db
							ctx.activity(InsertEventsInput {
								client_id,
								events: events.clone(),
							})
							.await?;

							// Re-dispatch container state updates
							let iter = events.into_iter().filter_map(|event| {
								if let protocol::Event::ContainerStateUpdate {
									container_id,
									state,
								} = event
								{
									let mut ctx = ctx.step();

									Some(
										async move {
											ctx.signal(ContainerStateUpdate { state })
												.tag("container_id", container_id)
												.send()
												.await
										}
										.boxed(),
									)
								} else {
									None
								}
							});

							futures_util::stream::iter(iter)
								.buffer_unordered(16)
								.try_collect::<Vec<_>>()
								.await?;
						}
						protocol::ToServer::FetchStateResponse {} => todo!(),
					}
				}
				Main::Command(sig) => {
					// Forward signal to ws as message
					ctx.msg(ToWs {
						client_id,
						inner: protocol::ToClient::Commands(vec![sig]),
					})
					.tags(json!({}))
					.send()
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

#[derive(Debug, Serialize, Deserialize, Hash)]
pub(crate) struct InsertEventsInput {
	client_id: Uuid,
	events: Vec<protocol::Event>,
}

#[activity(InsertEvents)]
pub(crate) async fn insert_events(
	ctx: &ActivityCtx,
	input: &InsertEventsInput,
) -> GlobalResult<()> {
	sql_execute!(
		[ctx]
		"
		INSERT INTO db_pegboard.client_events
		VALUES ($1, TODO)
		",
		input.client_id,
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

join_signal!(Main {
	Command(protocol::Command),
	// Forwarded from the ws to this workflow
	Forward(protocol::ToServer),
});

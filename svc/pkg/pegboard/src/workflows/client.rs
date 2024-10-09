use chirp_workflow::prelude::*;
use futures_util::FutureExt;

use crate::types::{ClientEvent, Command, ContainerState};

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
	pub client_id: Uuid,
}

#[workflow]
pub async fn pegboard_client(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	ctx.repeat(|ctx| {
		let client_id = input.client_id;

		async move {
			match ctx.listen::<Either>().await? {
				Either::Command(sig) => {
					// Forward signal to ws as message
					ctx.msg(sig).tag("target", "ws").send().await?;
				}
				Either::ClientEvent(sig) => {
					ctx.join((
						// Write to db
						activity(InsertEventInput {
							client_id,
							event: sig.clone(),
						}),
						// Re-dispatch container state updates
						closure(|ctx| {
							async move {
								if let ClientEvent::ContainerStateUpdate {
									container_id,
									state,
								} = sig
								{
									ctx.signal(ContainerStateUpdate { state })
										.tag("container_id", container_id)
										.send()
										.await?;
								}

								Ok(())
							}
							.boxed()
						}),
					))
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

#[signal("pegboard_container_state_update")]
struct ContainerStateUpdate {
	state: ContainerState,
}

join_signal!(Either, [Command, ClientEvent]);

#[derive(Debug, Serialize, Deserialize, Hash)]
pub(crate) struct InsertEventInput {
	client_id: Uuid,
	event: ClientEvent,
}

#[activity(InsertEvent)]
pub(crate) async fn insert_event(ctx: &ActivityCtx, input: &InsertEventInput) -> GlobalResult<()> {
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

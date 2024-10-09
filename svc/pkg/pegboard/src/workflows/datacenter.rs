use chirp_workflow::prelude::*;
use futures_util::FutureExt;

use crate::protocol;

/// How long after last ping before not considering a client for allocation.
const CLIENT_ELIGIBLE_THRESHOLD_MS: i64 = util::duration::seconds(10);

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
	pub datacenter_id: Uuid,
}

#[workflow]
pub async fn pegboard_datacenter(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	ctx.repeat(|ctx| {
		let datacenter_id = input.datacenter_id;

		async move {
			match ctx.listen::<protocol::Command>().await? {
				protocol::Command::StartContainer {
					container_id,
					config,
				} => {
					let client_id = ctx
						.activity(AllocateContainerInput {
							datacenter_id,
							container_id,
							config: *config.clone(),
						})
						.await?;

					if let Some(client_id) = client_id {
						// Forward signal to client
						ctx.signal(protocol::Command::StartContainer {
							container_id,
							config,
						})
						.tag("client_id", client_id)
						.send()
						.await?;
					} else {
						tracing::error!(
							?datacenter_id,
							?container_id,
							"failed to allocate container"
						);

						ctx.signal(crate::workflows::client::ContainerStateUpdate {
							state: protocol::ContainerState::FailedToAllocate,
						})
						.tag("container_id", container_id)
						.send()
						.await?;
					}
				}
				protocol::Command::SignalContainer {
					container_id,
					signal,
				} => {
					let client_id = ctx
						.activity(GetClientForContainerInput { container_id })
						.await?;

					if let Some(client_id) = client_id {
						// Forward signal to client
						ctx.signal(protocol::Command::SignalContainer {
							container_id,
							signal,
						})
						.tag("client_id", client_id)
						.send()
						.await?;
					} else {
						tracing::warn!(
							?container_id,
							"tried sending signal to container that doesn't exist"
						);
					}
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
struct AllocateContainerInput {
	datacenter_id: Uuid,
	container_id: Uuid,
	config: protocol::ContainerConfig,
}

/// Selects a client to allocate the container to. Attempts to find the most full client that has capacity for
/// this container.
#[activity(AllocateContainer)]
async fn allocate_container(
	ctx: &ActivityCtx,
	input: &AllocateContainerInput,
) -> GlobalResult<Option<Uuid>> {
	let row = sql_fetch_optional!(
		[ctx, (Uuid,)]
		"
		INSERT INTO db_pegboard.containers (container_id, client_id, config, create_ts)
		SELECT $3, client_id, $4, $5
		FROM db_pegboard.clients AS c
		WHERE
			datacenter_id = $1 AND
			last_ping_ts > $2 AND
			drain_ts IS NULL AND
			delete_ts IS NULL AND
			(
				SELECT SUM(((config->'resources'->'cpu')::INT))
				FROM db_pegboard.containers AS co
				WHERE co.client_id = c.client_id
			) + $6 <= cpu AND
			(
				SELECT SUM(((config->'resources'->'memory')::INT))
				FROM db_pegboard.containers AS co
				WHERE co.client_id = c.client_id
			) + $7 <= memory
		ORDER BY cpu, memory DESC
		LIMIT 1
		RETURNING client_id
		",
		input.datacenter_id,
		util::timestamp::now() - CLIENT_ELIGIBLE_THRESHOLD_MS,
		input.container_id,
		serde_json::to_string(&input.config)?,
		util::timestamp::now(),
		input.config.resources.cpu as i64,
		input.config.resources.memory as i64,
	)
	.await?;

	Ok(row.map(|(client_id,)| client_id))
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct GetClientForContainerInput {
	container_id: Uuid,
}

#[activity(GetClientForContainer)]
async fn get_client_for_container(
	ctx: &ActivityCtx,
	input: &GetClientForContainerInput,
) -> GlobalResult<Option<Uuid>> {
	let row = sql_fetch_optional!(
		[ctx, (Uuid,)]
		"
		SELECT db_pegboard.containers
		FROM db_pegboard.clients
		WHERE container_id = $1
		",
		input.container_id,
	)
	.await?;

	Ok(row.map(|(client_id,)| client_id))
}

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
				protocol::Command::StartActor { actor_id, config } => {
					let client_id = ctx
						.activity(AllocateActorInput {
							datacenter_id,
							actor_id,
							config: *config.clone(),
						})
						.await?;

					if let Some(client_id) = client_id {
						ctx.signal(crate::workflows::client::ActorStateUpdate {
							state: protocol::ActorState::Allocated { client_id },
						})
						.tag("actor_id", actor_id)
						.send()
						.await?;

						// Forward signal to client
						ctx.signal(protocol::Command::StartActor { actor_id, config })
							.tag("client_id", client_id)
							.send()
							.await?;
					} else {
						tracing::error!(?datacenter_id, ?actor_id, "failed to allocate actor");

						ctx.signal(crate::workflows::client::ActorStateUpdate {
							state: protocol::ActorState::FailedToAllocate,
						})
						.tag("actor_id", actor_id)
						.send()
						.await?;
					}
				}
				protocol::Command::SignalActor {
					actor_id,
					signal,
					persist_storage,
					ignore_future_state,
				} => {
					let client_id = ctx.activity(GetClientForActorInput { actor_id }).await?;

					if let Some(client_id) = client_id {
						// Forward signal to client
						ctx.signal(protocol::Command::SignalActor {
							actor_id,
							signal,
							persist_storage,
							ignore_future_state,
						})
						.tag("client_id", client_id)
						.send()
						.await?;
					} else {
						tracing::warn!(
							?actor_id,
							"tried sending signal to actor that doesn't exist"
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
struct AllocateActorInput {
	datacenter_id: Uuid,
	actor_id: Uuid,
	config: protocol::ActorConfig,
}

/// Selects a client to allocate the actor to. Attempts to find the most full client that has capacity for
/// this actor.
#[activity(AllocateActor)]
async fn allocate_actor(
	ctx: &ActivityCtx,
	input: &AllocateActorInput,
) -> GlobalResult<Option<Uuid>> {
	let datacenter_id = input.datacenter_id;
	let current_time = util::timestamp::now();
	let client_eligible_time = current_time - CLIENT_ELIGIBLE_THRESHOLD_MS;
	let actor_id = input.actor_id;
	let allocated_cpu = input.config.resources.cpu as i64;
	let allocated_memory_mib = (input.config.resources.memory / 1024 / 1024) as i64;
	let client_flavor = match input.config.image.kind {
		protocol::ImageKind::DockerImage | protocol::ImageKind::OciBundle => {
			protocol::ClientFlavor::Container
		}
		protocol::ImageKind::JavaScript => protocol::ClientFlavor::Isolate,
	} as i32;

	tracing::debug!(
		?datacenter_id,
		?current_time,
		?client_eligible_time,
		?actor_id,
		?allocated_cpu,
		?allocated_memory_mib,
		?client_flavor,
		"allocating actor"
	);

	// Even though isolates autoscale based on CPU, we allocate machines based on reservation in
	// balance proactively. Otherwise, we'd end up with bad scaling with retroactively choosing
	// nodes based on CPU load since actors will show the CPU load after a delay.
	let client_id = sql_fetch_optional!(
		[ctx, (Uuid,)]
		"
		WITH available_clients AS (
			SELECT
				c.client_id,
				-- Millicores
				(
					COALESCE((c.system_info->'cpu'->'physical_core_count')::INT, 0) * 1000 -
					COALESCE((c.config->'reserved_resources'->'cpu')::INT, 0)
				) AS available_cpu,
				-- MiB
				(
					-- Convert bytes to MiB
					COALESCE(((c.system_info->'memory'->'total_memory')::INT), 0) // 1048576 - 
					COALESCE(((c.config->'reserved_resources'->'memory')::INT), 0) 
				) AS available_memory,
				-- Millicores
				COALESCE(SUM_INT((a.config->'resources'->'cpu')::INT), 0) AS allocated_cpu,
				-- MiB
				COALESCE(SUM_INT((a.config->'resources'->'memory')::INT // 1048576), 0) AS allocated_memory
			FROM db_pegboard.clients AS c
			LEFT JOIN db_pegboard.actors AS a
			ON
				c.client_id = a.client_id AND
				-- Actor not stopped
				a.stop_ts IS NULL AND
				-- Not exited
				a.exit_ts IS NULL
			WHERE
				c.datacenter_id = $1 AND
				-- Within ping threshold
				c.last_ping_ts > $2 AND
				-- Not draining
				c.drain_ts IS NULL AND
				-- Not deleted
				c.delete_ts IS NULL AND
				-- Flavor match
				c.flavor = $8
			GROUP BY c.client_id
		)
		INSERT INTO db_pegboard.actors (actor_id, client_id, config, create_ts)
		SELECT $3, client_id, $4, $5
		FROM available_clients
		WHERE
			-- Containers (0): ensure node has available resources
			-- Isolates (1): don't limit resources since they scale on CPU
			CASE WHEN $8 = 0
				THEN (
					allocated_cpu + $6 <= available_cpu AND
					allocated_memory + $7 <= available_memory
				)
				ELSE TRUE
			END
		ORDER BY
			-- Container (0): binpack to the most-populated node to maximize density
			-- Isolate (1): allocate to the least-populated node since we autoscale on CPU
			CASE WHEN $8 = 0
				THEN allocated_cpu
				ELSE -allocated_cpu
			END DESC,
			CASE WHEN $8 = 0
				THEN allocated_memory
				ELSE -allocated_memory
			END DESC
		LIMIT 1
		RETURNING client_id
		",
		datacenter_id,
		client_eligible_time,
		actor_id,
		serde_json::to_value(&input.config)?,
		current_time,
		allocated_cpu,
		allocated_memory_mib,
		client_flavor,
	)
	.await?
	.map(|(client_id,)| client_id);

	Ok(client_id)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct GetClientForActorInput {
	actor_id: Uuid,
}

#[activity(GetClientForActor)]
async fn get_client_for_actor(
	ctx: &ActivityCtx,
	input: &GetClientForActorInput,
) -> GlobalResult<Option<Uuid>> {
	let row = sql_fetch_optional!(
		[ctx, (Uuid,)]
		"
		SELECT client_id
		FROM db_pegboard.actors
		WHERE actor_id = $1
		",
		input.actor_id,
	)
	.await?;

	Ok(row.map(|(client_id,)| client_id))
}

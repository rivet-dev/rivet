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
				protocol::Command::SignalActor { actor_id, signal } => {
					let client_id = ctx.activity(GetClientForActorInput { actor_id }).await?;

					if let Some(client_id) = client_id {
						// Forward signal to client
						ctx.signal(protocol::Command::SignalActor { actor_id, signal })
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
	let client_id = sql_fetch_optional!(
		[ctx, (Uuid,)]
		"
		WITH available_clients AS (
			SELECT
				c.client_id,
				c.cpu,
				c.memory,
				-- Millicores
				COALESCE(SUM_INT((a.config->'resources'->>'cpu')::INT), 0) AS total_cpu,
				-- MiB
				COALESCE(SUM_INT((a.config->'resources'->>'memory')::INT // 1024 // 1024), 0) AS total_memory
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
			-- Compare millicores
			total_cpu + $6 <= cpu * 1000 // $9 AND
			-- Compare memory MiB
			total_memory + $7 <= memory - $10
		ORDER BY total_cpu, total_memory DESC
		LIMIT 1
		RETURNING client_id
		",
		input.datacenter_id,
		util::timestamp::now() - CLIENT_ELIGIBLE_THRESHOLD_MS,
		input.actor_id,
		serde_json::to_string(&input.config)?,
		util::timestamp::now(), // $5
		input.config.resources.cpu as i64,
		// Bytes to MiB
		(input.config.resources.memory / 1024 / 1024) as i64,
		// Pegboard manager flavor
		match input.config.image.kind {
			protocol::ImageKind::DockerImage |
			protocol::ImageKind::OciBundle => protocol::ClientFlavor::Container,
			protocol::ImageKind::JavaScript => protocol::ClientFlavor::Isolate,
		} as i32,
		// NOTE: This should technically be reading from a tier config but for now its constant because linode
		// provides the same CPU per core for all instance types
		server_spec::CPU_PER_CORE as i32,
		// Subtract reserve memory from client memory
		(server_spec::RESERVE_LB_MEMORY + server_spec::PEGBOARD_RESERVE_MEMORY) as i32, // $10
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

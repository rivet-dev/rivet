use std::{collections::HashSet, time::Duration};

use chirp_workflow::prelude::*;
use serde_json::json;

/// Amount of time to wait after all servers have successfully polled to wait to return in order to
/// avoid a race condition.
///
/// This can likely be decreased to < 100 ms safely.
const TRAEFIK_POLL_COMPLETE_GRACE: Duration = Duration::from_millis(750);

/// Max time to wait for servers to poll their configs.
const TRAEFIK_POLL_TIMEOUT: Duration = Duration::from_secs(5);

/// How logn to wait if no GG servers were returned from the list. This is either from:
/// - Cluster without provisioning configured
/// - Edge case where all GG servers were destroyed and waiting for new servers to come up
const TRAFEIK_NO_SERVERS_GRACE: Duration = Duration::from_millis(500);

#[message("pegboard_actor_traefik_poll")]
pub struct TraefikPoll {
	/// Server ID will be `None` if:
	/// - Not using provisioning (i.e. self-hosted cluster) or
	/// - Older GG node that's being upgraded
	pub server_id: Option<Uuid>,
	pub latest_actor_create_ts: i64,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct WaitForTraefikPollInput {
	pub create_ts: i64,
}

/// Waits for all of the GG nodes to poll the Traefik config.
///
/// This is done by waiting for an event to be published for each of the GG server IDs with a
/// timestamp for the latest actor it's seen that's > than this actor's create ts.
#[activity(WaitForTraefikPoll)]
pub async fn wait_for_traefik_poll(
	ctx: &ActivityCtx,
	input: &WaitForTraefikPollInput,
) -> GlobalResult<()> {
	// TODO: This will only work with 1 node on self-hosted. RG 2 will be out by then which fixes
	// this issue.

	let cluster_id = ctx.config().server()?.rivet.edge()?.cluster_id;
	let dc_id = ctx.config().server()?.rivet.edge()?.datacenter_id;

	// Start sub first since the messages may arrive while fetching the server list
	let mut sub = ctx
		.subscribe::<TraefikPoll>(&json!({ "datacenter_id": dc_id }))
		.await?;

	// Fetch servers
	let servers_res = ctx
		.op(cluster::ops::server::list::Input {
			filter: cluster::types::Filter {
				pool_types: Some(vec![cluster::types::PoolType::Gg]),
				cluster_ids: Some(vec![cluster_id]),
				..Default::default()
			},
			include_destroyed: false,
			exclude_draining: true,
			exclude_no_vlan: false,
		})
		.await?;

	let mut remaining_servers = if servers_res.servers.is_empty() {
		// HACK: Will wait for a single server poll if we don't have the server list. Wait for a
		// static amount of time.
		tokio::time::sleep(TRAFEIK_NO_SERVERS_GRACE).await;
		return Ok(());
	} else {
		servers_res
			.servers
			.iter()
			.map(|s| s.server_id)
			.collect::<HashSet<Uuid>>()
	};

	tracing::debug!(
		servers=?remaining_servers,
		after_create_ts=?input.create_ts,
		"waiting for traefik servers",
	);
	let res = tokio::time::timeout(TRAEFIK_POLL_TIMEOUT, async {
		// Wait for servers to fetch their configs
		loop {
			let msg = sub.next().await?;

			if let Some(server_id) = msg.server_id {
				if msg.latest_actor_create_ts >= input.create_ts {
					let _did_remove = remaining_servers.remove(&server_id);

					tracing::debug!(
						server_id=?msg.server_id,
						latest_actor_create_ts=?msg.latest_actor_create_ts,
						servers=?remaining_servers, "received poll from traefik server",
					);

					// Break loop once all servers have polled
					if remaining_servers.is_empty() {
						return GlobalResult::Ok(());
					}
				}
			}
		}
	})
	.await;

	match res {
		Ok(_) => {
			tracing::debug!("received poll from all traefik servers, waiting for grace period");
			tokio::time::sleep(TRAEFIK_POLL_COMPLETE_GRACE).await;
		}
		Err(_) => {
			tracing::warn!(missing_server_ids = ?remaining_servers, "did not receive poll from all gg servers before deadline");
		}
	}

	Ok(())
}

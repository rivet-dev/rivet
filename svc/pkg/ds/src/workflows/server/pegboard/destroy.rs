use chirp_workflow::prelude::*;
use futures_util::FutureExt;
use nix::sys::signal::Signal;
use pegboard::protocol as pp;

use super::super::{DestroyComplete, DestroyStarted};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Input {
	pub server_id: Uuid,
	pub override_kill_timeout_ms: Option<i64>,
	/// Whether or not to send signals to the container. In the case that the container was already stopped
	/// or exited, signals are unnecessary.
	pub signal: bool,
}

#[workflow]
pub(crate) async fn ds_server_pegboard_destroy(
	ctx: &mut WorkflowCtx,
	input: &Input,
) -> GlobalResult<()> {
	let ds = ctx
		.activity(UpdateDbInput {
			server_id: input.server_id,
		})
		.await?;

	ctx.msg(DestroyStarted {})
		.tag("server_id", input.server_id)
		.send()
		.await?;

	if input.signal {
		ctx.signal(pp::Command::SignalContainer {
			container_id: ds.container_id,
			signal: Signal::SIGTERM as i32,
		})
		.tag("datacenter_id", ds.datacenter_id)
		.send()
		.await?;

		// See `docs/packages/job/JOB_DRAINING_AND_KILL_TIMEOUTS.md`
		ctx.sleep(input.override_kill_timeout_ms.unwrap_or(ds.kill_timeout_ms))
			.await?;

		ctx.signal(pp::Command::SignalContainer {
			container_id: ds.container_id,
			signal: Signal::SIGKILL as i32,
		})
		.tag("datacenter_id", ds.datacenter_id)
		.send()
		.await?;
	}

	ctx.msg(DestroyComplete {})
		.tag("server_id", input.server_id)
		.send()
		.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateDbInput {
	server_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize, Hash, sqlx::FromRow)]
struct UpdateDbOutput {
	datacenter_id: Uuid,
	kill_timeout_ms: i64,
	container_id: Uuid,
}

#[activity(UpdateDb)]
async fn update_db(ctx: &ActivityCtx, input: &UpdateDbInput) -> GlobalResult<UpdateDbOutput> {
	// Run in transaction for internal retryability
	let db_output = rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| {
		let ctx = ctx.clone();
		let server_id = input.server_id;

		async move {
			sql_fetch_one!(
				[ctx, UpdateDbOutput, @tx tx]
				"
				UPDATE db_ds.servers AS s1
				SET destroy_ts = $2
				FROM db_ds.servers AS s2
				JOIN db_ds.servers_pegboard AS spb
				ON s2.server_id = spb.server_id
				WHERE
					s1.server_id = $1 AND
					s1.server_id = s2.server_id AND
					s2.destroy_ts IS NULL
				RETURNING
					s1.datacenter_id,
					s1.kill_timeout_ms,
					spb.pegboard_container_id AS container_id
				",
				server_id,
				ctx.ts(),
			)
			.await
		}
		.boxed()
	})
	.await?;

	// NOTE: This call is infallible because redis is infallible. If it was not, it would be put in its own
	// workflow step
	// Invalidate cache when server is destroyed
	ctx.cache()
		.purge("servers_ports", [db_output.datacenter_id])
		.await?;

	Ok(db_output)
}

use std::convert::TryInto;

use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use serde_json::json;

#[worker(name = "ds-drain-all")]
async fn worker(ctx: &OperationContext<ds::msg::drain_all::Message>) -> GlobalResult<()> {
	let drain_timeout = ctx.drain_timeout.try_into()?;

	let server_rows = sql_fetch_all!(
		[ctx, (Uuid, i64)]
		"
		SELECT s.server_id, s.kill_timeout_ms
		FROM db_ds.servers AS s
		JOIN db_ds.server_nomad AS sn
		ON s.server_id = sn.server_id
		WHERE
			sn.nomad_node_id = $1 AND
			s.destroy_ts IS NULL
		",
		&ctx.nomad_node_id,
	)
	.await?;

	for (server_id, kill_timeout_ms) in server_rows {
		chirp_workflow::compat::tagged_signal(
			ctx,
			&json!({
				"server_id": server_id,
			}),
			crate::workflows::server::Destroy {
				override_kill_timeout_ms: (drain_timeout < kill_timeout_ms).then(|| drain_timeout),
			},
		)
		.await?;
	}

	Ok(())
}

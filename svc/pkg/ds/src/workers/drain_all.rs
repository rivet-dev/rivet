use std::convert::TryInto;

use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "ds-drain-all")]
async fn worker(ctx: &OperationContext<ds::msg::drain_all::Message>) -> GlobalResult<()> {
	let drain_timeout = ctx.drain_timeout.try_into()?;

	let server_rows = if let Some(nomad_node_id) = &ctx.nomad_node_id {
		sql_fetch_all!(
			[ctx, (Uuid,)]
			"
			SELECT s.server_id
			FROM db_ds.servers AS s
			JOIN db_ds.server_nomad AS sn
			ON s.server_id = sn.server_id
			WHERE
				sn.nomad_node_id = $1 AND
				s.destroy_ts IS NULL
			",
			nomad_node_id,
		)
		.await?
	} else if let Some(pegboard_client_id) = &ctx.pegboard_client_id {
		sql_fetch_all!(
			[ctx, (Uuid,)]
			"
			SELECT s.server_id
			FROM db_ds.servers AS s
			JOIN db_ds.servers_pegboard AS spb
			ON s.server_id = spb.server_id
			JOIN db_pegboard.containers AS co
			ON spb.pegboard_container_id = co.container_id
			WHERE
				co.client_id = $1 AND
				s.destroy_ts IS NULL
			",
			pegboard_client_id,
		)
		.await?
	} else {
		bail!("neither `nomad_node_id` nor `pegboard_client_id` set");
	};

	for (server_id,) in server_rows {
		chirp_workflow::compat::signal(ctx, crate::workflows::server::Drain { drain_timeout })
			.await?
			.tag("server_id", server_id)
			.send()
			.await?;
	}

	Ok(())
}

use std::convert::TryInto;

use chirp_workflow::prelude::*;

#[derive(sqlx::FromRow)]
struct ClientRow {
	client_id: Uuid,
	total_cpu: i64,
	total_memory: i64,
}

#[derive(Debug)]
pub struct Input {
	pub client_ids: Vec<Uuid>,
}

#[derive(Debug)]
pub struct Output {
	pub clients: Vec<Client>,
}

#[derive(Debug)]
pub struct Client {
	pub client_id: Uuid,
	pub usage: Stats,
	pub limits: Stats,
}

#[derive(Debug)]
pub struct Stats {
	/// Mhz
	pub cpu: u64,
	/// MiB
	pub memory: u64,
	/// MB
	pub disk: u64,
}

#[operation]
pub async fn pegboard_client_usage_get(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let clients = sql_fetch_all!(
		[ctx, ClientRow]
		"
		SELECT
			c.client_id,
			-- Millicores to MHz
			COALESCE(SUM_INT((co.config->'resources'->>'cpu')::INT), 0) * 1999 // 1000 AS total_cpu,
			-- Bytes to MiB
			COALESCE(SUM_INT((co.config->'resources'->>'memory')::INT // 1024 // 1024), 0) AS total_memory
		FROM db_pegboard.clients AS c
		LEFT JOIN db_pegboard.containers AS co
		ON
			c.client_id = co.client_id AND
			co.stop_ts IS NULL AND
			co.exit_ts IS NULL
		WHERE
			c.client_id = ANY($1)
		GROUP BY c.client_id
		",
		&input.client_ids,
	)
	.await?
	.into_iter()
	.map(|client| {
		Ok(Client {
			client_id: client.client_id,
			usage: Stats {
				cpu: client.total_cpu.try_into()?,
				memory: client.total_memory.try_into()?,
				disk: 0, // TODO:
			},
			limits: Stats {
				cpu: 0,    // TODO:
				memory: 0, // TODO:
				disk: 0,   // TODO:
			},
		})
	})
	.collect::<GlobalResult<_>>()?;

	Ok(Output { clients })
}

use std::convert::TryInto;

use chirp_workflow::prelude::*;

#[derive(sqlx::FromRow)]
struct ClientRow {
	client_id: Uuid,
	cpu: i64,
	memory: i64,
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
	/// MB
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
			client_id,
			SUM(config->'resources'->'cpu'::INT) AS cpu,
			SUM(config->'resources'->'memory'::INT) AS memory
		FROM db_pegboard.containers
		WHERE client_id = ANY($1)
		GROUP BY client_id
		",
		&input.client_ids,
	)
	.await?
	.into_iter()
	.map(|client| {
		Ok(Client {
			client_id: client.client_id,
			usage: Stats {
				cpu: client.cpu.try_into()?,
				memory: client.memory.try_into()?,
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

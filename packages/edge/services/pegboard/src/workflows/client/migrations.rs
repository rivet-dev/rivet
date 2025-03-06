use chirp_workflow::prelude::*;
use sqlx::Acquire;

pub async fn run(ctx: &mut WorkflowCtx) -> GlobalResult<()> {
	ctx.activity(MigrateInitInput {}).await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct MigrateInitInput {}

#[activity(MigrateInit)]
async fn migrate_init(ctx: &ActivityCtx, _input: &MigrateInitInput) -> GlobalResult<()> {
	let pool = ctx.sqlite().await?;
	let mut conn = pool.conn().await?;
	let mut tx = conn.begin().await?;

	sql_execute!(
		[ctx, @tx &mut tx]
		"
		CREATE TABLE state (
			create_ts INT NOT NULL,
			last_event_idx INT NOT NULL DEFAULT -1,
			last_command_idx INT NOT NULL DEFAULT -1,

			flavor INT NOT NULL, -- pegboard::protocol::ClientFlavor
			system_info BLOB, -- JSONB
			config BLOB -- JSONB
		) STRICT;

		CREATE TABLE events (
			idx INT PRIMARY KEY,
			payload BLOB NOT NULL, -- JSONB
			ack_ts INT NOT NULL
		) STRICT;

		CREATE TABLE commands (
			idx INT PRIMARY KEY,
			payload BLOB NOT NULL, -- JSONB
			create_ts INT NOT NULL
		) STRICT;
		",
	)
	.await?;

	tx.commit().await?;

	Ok(())
}

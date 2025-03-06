use chirp_workflow::prelude::*;
use sqlite_util::SqlitePoolExt;
use sqlx::Acquire;

pub async fn run(ctx: &mut WorkflowCtx) -> GlobalResult<()> {
	ctx.activity(MigrateInitInput {}).await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct MigrateInitInput {}

#[activity(MigrateInit)]
async fn migrate_init(ctx: &ActivityCtx, _input: &MigrateInitInput) -> GlobalResult<()> {
	let mut conn = ctx.sqlite().await?.conn().await?;
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

		CREATE TABLE actors (
			actor_id BLOB PRIMARY KEY, -- UUID
			config BLOB NOT NULL, -- pegboard::protocol::ActorConfig
			create_ts INT NOT NULL,

			-- See protocol.rs `ActorState` for info
			start_ts INT,
			running_ts INT,
			stopping_ts INT,
			stop_ts INT,
			exit_ts INT,
			lost_ts INT,

			pid INT,
			exit_code INT,

			ignore_future_state INT NOT NULL DEFAULT false -- BOOLEAN
		) STRICT;
		",
	)
	.await?;

	tx.commit().await?;

	Ok(())
}

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
			env_id BLOB NOT NULL, -- UUID
			tags BLOB NOT NULL, -- JSONB, map<string, string>
			resources_cpu_millicores INT NOT NULL,
			resources_memory_mib INT NOT NULL,

			client_id BLOB, -- UUID,
			client_workflow_id BLOB, -- UUID,
			client_wan_hostname TEXT,

			lifecycle_kill_timeout_ms INT NOT NULL,
			lifecycle_durable INT NOT NULL DEFAULT false, -- BOOLEAN
			
			create_ts INT NOT NULL,
			start_ts INT,
			connectable_ts INT,
			finish_ts INT,
			destroy_ts INT,

			image_id BLOB NOT NULL, -- UUID
			args BLOB NOT NULL, -- JSONB, list<string>
			network_mode INT NOT NULL, -- pegboard::types::NetworkMode
			environment BLOB NOT NULL -- JSONB, map<string, string>
		) STRICT;

		CREATE TABLE ports_ingress (
			port_name TEXT PRIMARY KEY,
			port_number INT,
			ingress_port_number INT NOT NULL,
			protocol INT NOT NULL -- pegboard::types::GameGuardProtocol
		) STRICT;

		CREATE TABLE ports_host (
			port_name TEXT PRIMARY KEY,
			port_number INT,
			protocol INT NOT NULL -- pegboard::types::HostProtocol
		) STRICT;

		CREATE TABLE ports_proxied (
			port_name TEXT PRIMARY KEY,
			ip TEXT NOT NULL,
			source INT NOT NULL
		) STRICT;
		",
	)
	.await?;

	tx.commit().await?;

	Ok(())
}

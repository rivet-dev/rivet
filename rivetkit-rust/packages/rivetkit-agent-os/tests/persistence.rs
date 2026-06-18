//! Validates the agent-os SQLite schema (`MIGRATION_SQL`) is well-formed,
//! idempotent, and round-trips the persisted tables — using an in-memory
//! rusqlite database (the same SQL the actor runs via `ctx.db_exec`).

use rivetkit_agent_os::persistence::MIGRATION_SQL;
use rusqlite::{Connection, params};

fn migrated_db() -> Connection {
	let conn = Connection::open_in_memory().expect("open in-memory db");
	conn.execute_batch(MIGRATION_SQL).expect("apply migration");
	conn
}

#[test]
fn migration_sql_is_valid_and_idempotent() {
	let conn = Connection::open_in_memory().expect("open in-memory db");
	// Applying twice must succeed (every statement is IF NOT EXISTS).
	conn.execute_batch(MIGRATION_SQL).expect("first migration");
	conn.execute_batch(MIGRATION_SQL)
		.expect("second migration must be idempotent");

	for table in [
		"agent_os_preview_tokens",
		"agent_os_fs_entries",
		"agent_os_sessions",
		"agent_os_session_events",
	] {
		let count: i64 = conn
			.query_row(
				"SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
				[table],
				|row| row.get(0),
			)
			.expect("query table presence");
		assert_eq!(count, 1, "table `{table}` should exist after migration");
	}

	for index in [
		"idx_preview_tokens_expires_at",
		"idx_fs_entries_parent",
		"idx_session_events_session_seq",
	] {
		let count: i64 = conn
			.query_row(
				"SELECT count(*) FROM sqlite_master WHERE type = 'index' AND name = ?1",
				[index],
				|row| row.get(0),
			)
			.expect("query index presence");
		assert_eq!(count, 1, "index `{index}` should exist after migration");
	}
}

#[test]
fn preview_tokens_roundtrip() {
	let conn = migrated_db();
	conn.execute(
		"INSERT INTO agent_os_preview_tokens (token, port, created_at, expires_at) \
		 VALUES (?1, ?2, ?3, ?4)",
		params!["tok-1", 8080_i64, 1_000_i64, 2_000_i64],
	)
	.expect("insert preview token");

	let (port, expires): (i64, i64) = conn
		.query_row(
			"SELECT port, expires_at FROM agent_os_preview_tokens WHERE token = ?1",
			["tok-1"],
			|row| Ok((row.get(0)?, row.get(1)?)),
		)
		.expect("read preview token");
	assert_eq!(port, 8080);
	assert_eq!(expires, 2_000);

	conn.execute(
		"DELETE FROM agent_os_preview_tokens WHERE token = ?1",
		["tok-1"],
	)
	.expect("delete preview token");
	let remaining: i64 = conn
		.query_row("SELECT count(*) FROM agent_os_preview_tokens", [], |r| {
			r.get(0)
		})
		.unwrap();
	assert_eq!(remaining, 0);
}

#[test]
fn sessions_and_events_roundtrip() {
	let conn = migrated_db();
	conn.execute(
		"INSERT INTO agent_os_sessions (session_id, agent_type, capabilities, agent_info, created_at) \
		 VALUES (?1, ?2, ?3, NULL, ?4)",
		params!["sess-1", "claude", "{}", 1_234_i64],
	)
	.expect("insert session");
	conn.execute(
		"INSERT INTO agent_os_session_events (session_id, seq, event, created_at) \
		 VALUES (?1, ?2, ?3, ?4)",
		params![
			"sess-1",
			0_i64,
			"{\"method\":\"session/update\"}",
			1_235_i64
		],
	)
	.expect("insert session event");

	let (agent_type, created_at): (String, i64) = conn
		.query_row(
			"SELECT agent_type, created_at FROM agent_os_sessions WHERE session_id = ?1",
			["sess-1"],
			|row| Ok((row.get(0)?, row.get(1)?)),
		)
		.expect("read session");
	assert_eq!(agent_type, "claude");
	assert_eq!(created_at, 1_234);

	let event: String = conn
		.query_row(
			"SELECT event FROM agent_os_session_events WHERE session_id = ?1 ORDER BY seq",
			["sess-1"],
			|row| row.get(0),
		)
		.expect("read session event");
	assert_eq!(event, "{\"method\":\"session/update\"}");
}

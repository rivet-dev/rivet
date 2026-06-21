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

/// L1: exercises the EXACT atomic `seq` allocation SQL used by
/// `persistence::insert_session_event` (which can't be unit-tested directly
/// since it takes a live actor `Ctx`). Proves: the first event for a session
/// gets `seq` 0 (`MAX` over the empty set -> SQL NULL -> `COALESCE(..,0)`);
/// subsequent events increment; and per-session counters are independent. The
/// allocation is computed INSIDE the INSERT (a sub-SELECT), which is what makes
/// the concurrent capture-task-vs-prompt-action path race-safe — SQLite
/// serializes writers, so two inserts cannot read the same `MAX` and duplicate.
/// A duplicate or gap here is a regression.
#[test]
fn session_event_seq_allocation_is_atomic_and_per_session() {
	let conn = migrated_db();
	for sid in ["a", "b"] {
		conn.execute(
			"INSERT INTO agent_os_sessions (session_id, agent_type, capabilities, created_at) \
			 VALUES (?1, 'pi', '{}', 0)",
			[sid],
		)
		.expect("insert session");
	}

	// The exact statement from `insert_session_event` (session_id bound twice).
	let insert = |sid: &str, event: &str| {
		conn.execute(
			"INSERT INTO agent_os_session_events (session_id, seq, event, created_at) \
			 SELECT ?1, \
			        COALESCE((SELECT MAX(seq) + 1 FROM agent_os_session_events WHERE session_id = ?1), 0), \
			        ?2, 0",
			params![sid, event],
		)
		.expect("insert event");
	};

	// Interleave the two sessions to mimic concurrent capture across sessions.
	insert("a", "a0");
	insert("b", "b0");
	insert("a", "a1");
	insert("a", "a2");
	insert("b", "b1");

	let seqs = |sid: &str| -> Vec<i64> {
		let mut stmt = conn
			.prepare("SELECT seq FROM agent_os_session_events WHERE session_id = ?1 ORDER BY seq")
			.unwrap();
		stmt.query_map([sid], |row| row.get(0))
			.unwrap()
			.map(|r| r.unwrap())
			.collect()
	};

	// Dense, gap-free, per-session sequences starting at 0.
	assert_eq!(seqs("a"), vec![0, 1, 2], "session a seqs");
	assert_eq!(seqs("b"), vec![0, 1], "session b seqs");

	// Insertion order preserved by seq ordering.
	let a_events: Vec<String> = {
		let mut stmt = conn
			.prepare(
				"SELECT event FROM agent_os_session_events WHERE session_id = 'a' ORDER BY seq",
			)
			.unwrap();
		stmt.query_map([], |row| row.get(0))
			.unwrap()
			.map(|r| r.unwrap())
			.collect()
	};
	assert_eq!(a_events, vec!["a0", "a1", "a2"]);
}

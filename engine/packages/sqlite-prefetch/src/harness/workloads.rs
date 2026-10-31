//! The golden workload suite: synthetic generators that run real SQLite over the two agent
//! storage formats in the repo (the pi agent's durable harness and the kitchen-sink agent
//! stress actor), rivetkit-core's wake, and the benchmark tables. Each workload builds its data,
//! reopens the database cold, and measures what follows. Sizes are scaled down from the load
//! scenarios so the suite runs in seconds; the access shapes are the same.
use std::sync::Arc;

use super::server::Strategy;
use super::store::MemStore;
use super::vfs::Event;
use super::{Harness, Report};

/// One workload.
pub struct Workload {
	pub name: &'static str,
	/// Builds the data.
	pub setup: fn(&mut Harness),
	/// The measured part, which starts with the database reopened cold.
	pub measure: fn(&mut Harness),
	/// Agentic workloads gate every revision; the rest need a written reason to regress.
	pub agentic: bool,
}

/// Every workload, agentic first.
pub fn all() -> Vec<Workload> {
	vec![
		Workload {
			name: "a1_forward_reads",
			setup: a1_setup,
			measure: a1_measure,
			agentic: true,
		},
		Workload {
			name: "a2_backward_context",
			setup: a2_setup,
			measure: a2_measure,
			agentic: true,
		},
		Workload {
			name: "a3_large_tool_rows",
			setup: a3_setup,
			measure: a3_measure,
			agentic: true,
		},
		Workload {
			name: "a3_strided_chain",
			setup: a3_strided_setup,
			measure: a3_strided_measure,
			agentic: true,
		},
		Workload {
			name: "a4_agent_loop",
			setup: a2_setup,
			measure: a4_measure,
			agentic: true,
		},
		Workload {
			name: "a5_heavy_inserts",
			setup: a5_setup,
			measure: a5_measure,
			agentic: true,
		},
		Workload {
			name: "a6_durable_stream",
			setup: a6_setup,
			measure: a6_measure,
			agentic: true,
		},
		Workload {
			name: "a7_wake",
			setup: a7_setup,
			measure: a7_measure,
			agentic: true,
		},
		Workload {
			name: "a8_point_lookups",
			setup: a7_setup,
			measure: a8_measure,
			agentic: true,
		},
		Workload {
			name: "b1_chat_log",
			setup: b1_setup,
			measure: b1_measure,
			agentic: false,
		},
		Workload {
			name: "b2_bench_rows",
			setup: b2_setup,
			measure: b2_measure,
			agentic: false,
		},
		Workload {
			name: "b3_low_write",
			setup: b3_setup,
			measure: b3_measure,
			agentic: false,
		},
		Workload {
			name: "b4_workflow_kv",
			setup: b4_setup,
			measure: b4_measure,
			agentic: false,
		},
	]
}

/// Runs a workload under a strategy. The oracle needs the reads SQLite makes, which do not
/// depend on the strategy, so it runs the workload once without prefetch first.
pub fn run(workload: &Workload, strategy: Strategy) -> Report {
	let future = match strategy {
		Strategy::Oracle => {
			let (_, events) = run_with(workload, Strategy::None, Vec::new());
			events
				.into_iter()
				.filter_map(|event| match event {
					Event::Read(access) => Some(access.pgno),
					Event::Commit { .. } | Event::Reopen => None,
				})
				.collect()
		}
		Strategy::Library
		| Strategy::LibraryWithout(_)
		| Strategy::None
		| Strategy::Window
		| Strategy::Kernel
		| Strategy::MainV2
		| Strategy::V3Adaptive => Vec::new(),
	};
	run_with(workload, strategy, future).0
}

fn run_with(workload: &Workload, strategy: Strategy, future: Vec<u32>) -> (Report, Vec<Event>) {
	let store = Arc::new(MemStore::default());
	let mut harness = Harness::with_store(strategy, store, future);
	(workload.setup)(&mut harness);
	harness.begin();
	harness.reopen();
	(workload.measure)(&mut harness);
	let report = harness.report(workload.name, strategy);
	if strategy.is_library() {
		harness.leak_check(workload.name);
	}
	let events = harness.events();
	(report, events)
}

fn blob(bytes: usize) -> String {
	format!("zeroblob({bytes})")
}

/// Text of `bytes` bytes that varies by row, so pages are not all alike.
fn text(bytes: usize, seed: &str) -> String {
	format!("substr(printf('%.{bytes}c', 'x') || {seed}, 1, {bytes})")
}

// A1: forward conversation reads. The stress actor's `thread_events` resume and catch-up, and
// the pi scheduler's task scan.
fn a1_setup(h: &mut Harness) {
	h.exec("CREATE TABLE thread_events (seq INTEGER PRIMARY KEY, event_type TEXT, payload TEXT)");
	h.exec(&format!(
		"WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 3000)
		INSERT INTO thread_events SELECT i, 'delta', {} FROM n",
		text(1_000, "i")
	));
	h.exec(
		"CREATE TABLE tasks (id INTEGER PRIMARY KEY, status TEXT, body TEXT);
		CREATE INDEX tasks_status ON tasks (status, id);",
	);
	h.exec(&format!(
		"WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 1000)
		INSERT INTO tasks SELECT i, CASE i % 4 WHEN 0 THEN 'pending' WHEN 1 THEN 'running' WHEN 2 THEN 'done' ELSE 'failed' END, {} FROM n",
		text(300, "i")
	));
}

fn a1_measure(h: &mut Harness) {
	h.query(
		"SELECT * FROM (SELECT * FROM thread_events ORDER BY seq DESC LIMIT 256) ORDER BY seq ASC",
	);
	h.mark("wake");
	for offset in [500, 1_200, 2_000] {
		h.query(&format!(
			"SELECT * FROM thread_events WHERE seq > {offset} ORDER BY seq ASC LIMIT 256"
		));
	}
	for status in ["pending", "running", "done", "failed"] {
		h.query(&format!(
			"SELECT * FROM tasks WHERE id > 0 AND status = '{status}' ORDER BY id ASC LIMIT 257"
		));
	}
	h.mark("steady");
	h.query("SELECT * FROM thread_events WHERE seq > 0 ORDER BY seq ASC");
	h.mark("full");
}

// A2: backward conversation reads. The pi context build: newest entries first, pages of 257
// from the tail back.
fn a2_setup(h: &mut Harness) {
	h.exec(
		"CREATE TABLE entries (id INTEGER PRIMARY KEY, conversation_id TEXT, head INTEGER, commit_seq INTEGER, record TEXT);
		CREATE INDEX entries_conversation ON entries (conversation_id, id DESC);
		CREATE INDEX entries_head ON entries (head) WHERE head IS NOT NULL;
		CREATE TABLE durable_metadata (k INTEGER PRIMARY KEY, next_id INTEGER, next_seq INTEGER);
		INSERT INTO durable_metadata VALUES (1, 0, 0);
		CREATE TABLE document_revisions (document_id TEXT, seq INTEGER, kind TEXT, content TEXT, PRIMARY KEY (document_id, seq));",
	);
	// Mostly assistant entries of about 2.5 KB, every tenth a tool result of up to 50 KB.
	h.exec(&format!(
		"WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 1500)
		INSERT INTO entries SELECT i, 'c1', CASE WHEN i % 500 = 0 THEN i END, i,
			CASE WHEN i % 10 = 0 THEN {} ELSE {} END FROM n",
		text(50_000, "i"),
		text(2_500, "i")
	));
}

fn a2_context_build(h: &mut Harness) {
	h.query("SELECT id, record FROM entries WHERE conversation_id = 'c1' ORDER BY id DESC LIMIT 2");
	h.query(
		"SELECT id FROM entries WHERE head IS NOT NULL AND id <= 1500 ORDER BY id DESC LIMIT 1",
	);
	let mut high = 1_500;
	while high > 1_000 {
		h.query(&format!(
			"SELECT id, record FROM entries WHERE conversation_id = 'c1' AND id >= 1000 AND id <= {high} ORDER BY id DESC LIMIT 257"
		));
		high -= 257;
	}
}

fn a2_measure(h: &mut Harness) {
	a2_context_build(h);
	h.mark("wake");
	a2_context_build(h);
	h.mark("repeat");
}

// A3: large tool calls and results, Cycle 145's huge rows, and deletes that walk the chain.
fn a3_setup(h: &mut Harness) {
	h.exec("CREATE TABLE tool_calls (id INTEGER PRIMARY KEY, args TEXT, result BLOB)");
	for (id, bytes) in [
		(1, 13 * 4_092),
		(2, 1 << 20),
		(3, 4 << 20),
		(4, 10 << 20),
		(5, 2_700),
	] {
		h.exec(&format!(
			"INSERT INTO tool_calls VALUES ({id}, 'args', {})",
			blob(bytes)
		));
	}
}

fn a3_measure(h: &mut Harness) {
	for id in 1..=5 {
		h.query(&format!(
			"SELECT length(result), result FROM tool_calls WHERE id = {id}"
		));
		h.mark(&format!("row{id}"));
	}
	h.exec("DELETE FROM tool_calls WHERE id = 4");
	h.mark("delete");
	h.exec(&format!(
		"INSERT INTO tool_calls VALUES (6, 'args', {})",
		blob(2 << 20)
	));
	h.query("SELECT length(result), result FROM tool_calls WHERE id = 6");
	h.mark("reinsert");
}

// A3 strided: two huge rows written in interleaved steps, so each chain's pages alternate.
fn a3_strided_setup(h: &mut Harness) {
	h.exec("CREATE TABLE blobs (id INTEGER PRIMARY KEY, data BLOB)");
	h.exec("INSERT INTO blobs VALUES (1, zeroblob(4096)), (2, zeroblob(4096))");
	h.exec("PRAGMA cache_size = 64");
	for size in (1..=8).map(|step| step * 256 * 1024) {
		h.exec(&format!(
			"BEGIN; UPDATE blobs SET data = zeroblob({size}) WHERE id = 1; UPDATE blobs SET data = zeroblob({size}) WHERE id = 2; COMMIT;"
		));
	}
}

fn a3_strided_measure(h: &mut Harness) {
	h.query("SELECT data FROM blobs WHERE id = 1");
	h.mark("first");
	h.query("SELECT data FROM blobs WHERE id = 2");
	h.mark("second");
}

// A4: the agent loop: a turn of partial commits, the turn-end transaction, the next context
// build, then sleep and wake.
fn a4_measure(h: &mut Harness) {
	a4_measure_with_timing(h, 0, 0);
}

/// The same four turns and SQL, with explicit partial-commit and between-turn delays.
/// Historical goldens pass zero; acceptance can exercise recency phases and sent TTLs.
pub fn a4_measure_with_timing(h: &mut Harness, partial_commit_ms: u64, think_ms: u64) {
	for turn in 0..4 {
		for partial in 0..12 {
			h.exec(&format!(
				"BEGIN;
				SELECT next_id FROM durable_metadata WHERE k = 1;
				INSERT OR REPLACE INTO document_revisions VALUES ('pi.live', {partial}, 'delta', {});
				UPDATE durable_metadata SET next_seq = next_seq + 1 WHERE k = 1;
				COMMIT;",
				text(200, "'p'")
			));
			h.advance_clock(partial_commit_ms);
		}
		let id = 1_501 + turn;
		h.exec(&format!(
			"BEGIN;
			INSERT INTO entries VALUES ({id}, 'c1', NULL, {id}, {});
			DELETE FROM document_revisions WHERE document_id = 'pi.live';
			INSERT INTO document_revisions VALUES ('pi.live', 0, 'base', {});
			COMMIT;",
			text(30_000, "'t'"),
			text(500, "'b'")
		));
		h.query(
			"SELECT id, record FROM entries WHERE conversation_id = 'c1' ORDER BY id DESC LIMIT 257",
		);
		h.mark(match turn {
			0 => "cold_turn",
			1 => "warm_turn",
			2 => "wake_turn",
			_ => "warm_after_wake",
		});
		h.advance_clock(think_ms);
		if turn == 1 {
			h.reopen();
		}
	}
}

// A5: heavy inserts: 50-row transactions, the preload's 16 KiB messages, token inserts, and
// the read-back.
fn a5_setup(h: &mut Harness) {
	h.exec(
		"CREATE TABLE messages (message_id INTEGER PRIMARY KEY, role TEXT, content TEXT, created_at INTEGER);
		CREATE INDEX messages_created ON messages (created_at DESC);
		CREATE TABLE tokens (id INTEGER PRIMARY KEY, token TEXT);",
	);
}

fn a5_measure(h: &mut Harness) {
	for batch in 0..20 {
		let start = batch * 50;
		h.exec(&format!(
			"WITH RECURSIVE n(i) AS (SELECT {start} UNION ALL SELECT i + 1 FROM n WHERE i < {start} + 49)
			INSERT INTO messages SELECT i, 'assistant', {}, i FROM n",
			text(16 * 1_024, "i")
		));
	}
	for token in 0..100 {
		h.exec(&format!("INSERT INTO tokens VALUES ({token}, 'tok')"));
	}
	h.mark("inserts");
	h.reopen();
	h.query("SELECT * FROM messages ORDER BY created_at DESC LIMIT 50");
	h.mark("read_back");
}

// A6: a durable stream: an append-only log, a tailing reader, catch-up after sleep.
fn a6_setup(h: &mut Harness) {
	h.exec("CREATE TABLE stream (seq INTEGER PRIMARY KEY, delta TEXT)");
	h.exec(&format!(
		"WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 4000)
		INSERT INTO stream SELECT i, {} FROM n",
		text(600, "i")
	));
}

fn a6_measure(h: &mut Harness) {
	h.query("SELECT * FROM stream WHERE seq > 1000 ORDER BY seq ASC LIMIT 256");
	h.mark("catch_up");
	let mut seq = 4_000;
	for _ in 0..20 {
		seq += 1;
		h.exec(&format!(
			"INSERT INTO stream VALUES ({seq}, {})",
			text(600, "'d'")
		));
		h.query(&format!(
			"SELECT * FROM stream WHERE seq > {} ORDER BY seq ASC LIMIT 256",
			seq - 1
		));
	}
	h.mark("tail");
	h.query("SELECT * FROM stream WHERE seq > 1256 ORDER BY seq ASC");
	h.mark("long_catch_up");
}

// A7: wake: rivetkit-core's sequence, the agent's migrations, then its first query.
fn a7_setup(h: &mut Harness) {
	h.exec(&format!(
		"CREATE TABLE _rivet_meta (key TEXT PRIMARY KEY, value BLOB);
		INSERT INTO _rivet_meta VALUES ('schema', 'v1');
		CREATE TABLE _rivet_actor_state (id INTEGER PRIMARY KEY, state BLOB);
		INSERT INTO _rivet_actor_state VALUES (1, {});
		CREATE TABLE _rivet_conns (id TEXT PRIMARY KEY, state BLOB);
		CREATE TABLE _rivet_alarms (id INTEGER PRIMARY KEY, at INTEGER);
		CREATE TABLE _rivet_queue (id INTEGER PRIMARY KEY, name TEXT, body BLOB);",
		blob(200 * 1_024)
	));
	a2_setup(h);
	h.exec(
		"CREATE TABLE tasks (id INTEGER PRIMARY KEY, status TEXT, body TEXT);
		CREATE INDEX tasks_status ON tasks (status, id);
		WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 400)
		INSERT INTO tasks SELECT i, 'pending', 'task' FROM n;",
	);
	// One earlier wake, so the early list holds a wake's pages.
	h.reopen();
	a7_wake_sequence(h);
	h.exec("UPDATE _rivet_meta SET value = 'woke' WHERE key = 'schema'");
}

fn a7_wake_sequence(h: &mut Harness) {
	h.query("SELECT name FROM sqlite_schema WHERE type = 'table'");
	h.query("SELECT value FROM _rivet_meta WHERE key = 'schema'");
	h.query("SELECT state FROM _rivet_actor_state WHERE id = 1");
	h.query("SELECT * FROM _rivet_conns");
	h.query("SELECT * FROM _rivet_alarms ORDER BY at");
	h.query("SELECT * FROM _rivet_queue ORDER BY id LIMIT 32");
	for table in ["entries", "durable_metadata", "document_revisions", "tasks"] {
		h.query(&format!(
			"SELECT EXISTS (SELECT 1 FROM sqlite_schema WHERE name = '{table}')"
		));
	}
	h.query("SELECT next_id, next_seq FROM durable_metadata WHERE k = 1");
	h.query("SELECT * FROM tasks WHERE id > 0 AND status = 'pending' ORDER BY id ASC LIMIT 257");
}

fn a7_measure(h: &mut Harness) {
	a7_wake_sequence(h);
	h.mark("core_and_ddl");
	h.query(
		"SELECT id, record FROM entries WHERE conversation_id = 'c1' ORDER BY id DESC LIMIT 257",
	);
	h.mark("first_query");
}

// A8: point lookups of agent state after wake, with the state blob rewritten every save.
fn a8_measure(h: &mut Harness) {
	a7_wake_sequence(h);
	h.mark("wake");
	for round in 0..20 {
		let id = 1 + (round * 73) % 1_500;
		h.query("SELECT next_id, next_seq FROM durable_metadata WHERE k = 1");
		h.query(&format!("SELECT * FROM tasks WHERE id = {}", 1 + round * 7));
		h.query(&format!("SELECT record FROM entries WHERE id = {id}"));
		if round % 5 == 4 {
			h.exec(&format!(
				"UPDATE _rivet_actor_state SET state = {} WHERE id = 1",
				blob(200 * 1_024)
			));
		}
	}
	h.mark("lookups");
}

// B1: the realworld bench's chat log.
fn b1_setup(h: &mut Harness) {
	h.exec(
		"CREATE TABLE rw_chat_log (id INTEGER PRIMARY KEY, thread_id INTEGER, seq INTEGER, created_at INTEGER, content TEXT);
		CREATE INDEX rw_chat_thread ON rw_chat_log (thread_id, seq DESC);",
	);
	h.exec(&format!(
		"WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 1500)
		INSERT INTO rw_chat_log SELECT i, i % 3, i, i, {} FROM n",
		text(4_096, "i")
	));
}

fn b1_measure(h: &mut Harness) {
	h.query("SELECT id, substr(content, 1, 128) FROM rw_chat_log WHERE thread_id = 1 ORDER BY seq DESC LIMIT 100");
	h.query("SELECT COUNT(*), SUM(length(content)) FROM rw_chat_log WHERE thread_id = 2");
	h.mark("reads");
}

// B2: the ladder's bench rows, scaled to 16 MiB.
fn b2_setup(h: &mut Harness) {
	h.exec("CREATE TABLE bench_rows (id INTEGER PRIMARY KEY, payload BLOB)");
	h.exec(&format!(
		"WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 4000)
		INSERT INTO bench_rows SELECT i, {} FROM n",
		blob(4_096)
	));
}

fn b2_measure(h: &mut Harness) {
	for round in 0..200 {
		h.query(&format!(
			"SELECT payload FROM bench_rows WHERE id = {}",
			1 + (round * 919) % 4_000
		));
	}
	h.mark("point");
	for round in 0..50 {
		let start = 1 + round * 40;
		h.query(&format!(
			"SELECT payload FROM bench_rows WHERE id BETWEEN {start} AND {}",
			start + 15
		));
	}
	h.mark("range16");
	h.query("SELECT COUNT(*), SUM(length(payload)) FROM bench_rows NOT INDEXED");
	h.mark("scan");
	for round in 0..20 {
		h.exec(&format!(
			"UPDATE bench_rows SET payload = {} WHERE id = {}",
			blob(4_096),
			1 + round * 13
		));
	}
	h.mark("updates");
}

// B3: low write and churn: a tiny table, an upsert per tick, sleep and wake.
fn b3_setup(h: &mut Harness) {
	h.exec("CREATE TABLE writes (slot INTEGER PRIMARY KEY, value TEXT)");
}

fn b3_measure(h: &mut Harness) {
	for cycle in 0..5 {
		h.query("SELECT * FROM writes ORDER BY slot LIMIT 4");
		for tick in 0..4 {
			h.exec(&format!(
				"INSERT OR REPLACE INTO writes VALUES ({}, printf('%.64c', 'v'))",
				(cycle * 4 + tick) % 16
			));
		}
		h.reopen();
	}
	h.mark("cycles");
}

// B4: workflow KV and the queue.
fn b4_setup(h: &mut Harness) {
	h.exec(
		"CREATE TABLE _rivet_wf_kv (key BLOB PRIMARY KEY, value BLOB) WITHOUT ROWID;
		CREATE TABLE _rivet_queue (id INTEGER PRIMARY KEY, name TEXT, body BLOB);
		CREATE INDEX queue_name ON _rivet_queue (name, id);",
	);
	h.exec(&format!(
		"WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 3000)
		INSERT INTO _rivet_wf_kv SELECT printf('wf/%04d/step/%04d', i / 100, i), {} FROM n",
		blob(300)
	));
	h.exec(&format!(
		"WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 1000)
		INSERT INTO _rivet_queue SELECT i, 'q' || (i % 4), {} FROM n",
		blob(200)
	));
}

fn b4_measure(h: &mut Harness) {
	for prefix in [3, 12, 25] {
		h.query(&format!(
			"SELECT * FROM _rivet_wf_kv WHERE key >= 'wf/{prefix:04}/' AND key < 'wf/{prefix:04}0' ORDER BY key"
		));
	}
	h.query("SELECT * FROM _rivet_queue ORDER BY id LIMIT 100");
	h.query("SELECT * FROM _rivet_queue WHERE name = 'q2' ORDER BY id LIMIT 100");
	for id in 1..=20 {
		h.exec(&format!("DELETE FROM _rivet_queue WHERE id = {id}"));
	}
	h.mark("ops");
}

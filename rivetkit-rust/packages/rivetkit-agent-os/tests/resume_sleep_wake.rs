//! L0 durable-SQLite prerequisite for the session-resume tests (L1/L2/L3).
//!
//! Goal: prove that the agent-os persistence schema + an
//! `agent_os_session_events` row survive a VM teardown ("Sleep") and a
//! subsequent wake, against a REAL persistent SQLite backend — without the
//! engine binary and without the agent-os sidecar.
//!
//! ## Why this does not drive the real actor `run()` loop
//!
//! The production run loop (`src/run.rs`) persists through `ctx.db_*`, which is
//! backed by `rivetkit_core::SqliteDb`. In every *constructible-from-a-test*
//! `ActorContext` the SqliteDb is `SqliteDb::default()` →
//! `SqliteBackend::Unavailable` (`ctx.sql().is_enabled() == false`), so
//! `migrate_actor`/`configure_actor_db_root` are skipped. There is **no public
//! constructor** on `ActorContext` (`new` / `new_with_kv`) that injects a live
//! `SqliteDb`; the only injecting path is `ActorContext::build`, which is
//! `pub(crate)` to `rivetkit-core` and only called by the registry with a
//! real `EnvoyHandle`. And `EnvoyHandle::sqlite_get_pages`/`sqlite_commit`
//! send `ToEnvoyMessage::SqliteRequest` over the envoy channel, which only the
//! engine (or an in-process pump, see notes at bottom) services.
//!
//! So this harness exercises the SAME storage primitive the run loop relies on
//! — the depot-backed `NativeDatabaseHandle` that core's `SqliteBackend::Local`
//! wraps — directly, running the agent-os schema and session-event SQL verbatim.
//! Closing the SQLite handle models the VM teardown on `RuntimeEvent::Sleep`;
//! reopening a fresh handle over the *same* durable depot store models the wake.
//! The depot store (RocksDB on disk) is the durable layer; it outlives the
//! handle exactly as it outlives the actor generation in production.
//!
//! Unblocking the full `run()`-loop variant requires a small production change
//! in `rivetkit-core`: a public test seam that builds an `ActorContext` with a
//! caller-supplied `SqliteDb` (e.g. `ActorContext::new_with_sqlite(...)`), plus
//! enabling the `rivetkit/sqlite-local` feature for the agent-os test build (the
//! default `rivetkit/sqlite` feature only turns on `sqlite-remote`). That change
//! is intentionally NOT made here.

use std::sync::Arc;

use depot::conveyer::Db;
use depot_client::database::NativeDatabaseHandle;
use depot_client::types::{BindParam, ColumnValue};
use depot_client_embedded::open_database_from_embedded_depot;
use gas::prelude::Id;
use rivet_pools::NodeId;
use rivetkit_agent_os::persistence::MIGRATION_SQL;
use tempfile::TempDir;

/// The exact INSERT the actor runs in `persistence::insert_session_event`
/// (atomic `MAX(seq)+1` allocation). Kept verbatim so this harness exercises the
/// real statement, not a paraphrase.
const INSERT_SESSION_EVENT_SQL: &str = "INSERT INTO agent_os_session_events (session_id, seq, event, created_at) \
	 SELECT ?, \
	        COALESCE((SELECT MAX(seq) + 1 FROM agent_os_session_events WHERE session_id = ?), 0), \
	        ?, ?";

/// A durable, in-process depot store backed by RocksDB in a temp dir. Holding
/// the `TempDir` keeps the on-disk store alive across handle close/reopen.
struct DurableDepot {
	udb: Arc<universaldb::Database>,
	bucket_id: Id,
	actor_id: String,
	generation: u64,
	_dir: TempDir,
}

impl DurableDepot {
	async fn new(actor_id: &str) -> anyhow::Result<Self> {
		let dir = tempfile::Builder::new()
			.prefix("agent-os-resume-l0-")
			.tempdir()?;
		let driver =
			universaldb::driver::RocksDbDatabaseDriver::new(dir.path().to_path_buf()).await?;
		let udb = Arc::new(universaldb::Database::new(Arc::new(driver)));
		Ok(Self {
			udb,
			bucket_id: Id::new_v1(1),
			actor_id: actor_id.to_owned(),
			generation: 1,
			_dir: dir,
		})
	}

	/// Build a fresh `NativeDatabaseHandle` over the same durable depot store.
	/// This is the same handle type `rivetkit_core::SqliteDb` opens for its
	/// local backend (`open_database_from_transport`), so the bytes path is
	/// identical to the actor's `ctx.db_*` writes.
	async fn open_handle(&self) -> anyhow::Result<NativeDatabaseHandle> {
		let db = Arc::new(Db::new(
			self.udb.clone(),
			self.bucket_id,
			self.actor_id.clone(),
			NodeId::new(),
		));
		let handle = open_database_from_embedded_depot(
			db,
			self.actor_id.clone(),
			self.generation,
			tokio::runtime::Handle::current(),
			None,
		)
		.await?;
		Ok(handle)
	}
}

async fn count_session_events(
	handle: &NativeDatabaseHandle,
	session_id: &str,
) -> anyhow::Result<i64> {
	let result = handle
		.execute(
			"SELECT COUNT(*) AS n FROM agent_os_session_events WHERE session_id = ?".to_owned(),
			Some(vec![BindParam::Text(session_id.to_owned())]),
		)
		.await?;
	let n = match result.rows.first().and_then(|row| row.first()) {
		Some(ColumnValue::Integer(n)) => *n,
		other => anyhow::bail!("unexpected COUNT(*) result: {other:?}"),
	};
	Ok(n)
}

#[tokio::test(flavor = "multi_thread")]
async fn session_event_survives_sleep_then_wake() -> anyhow::Result<()> {
	let session_id = "external-session-l0";
	let depot = DurableDepot::new("agent-os-actor-l0").await?;

	// --- Generation 1: migrate + write an agent_os_session_events row. ---
	let handle = depot.open_handle().await?;

	// Same schema the actor runs at the top of run() via migrate_actor().
	handle.exec(MIGRATION_SQL.to_owned()).await?;

	// A session row first (FK target), then the event — same shape as the
	// real persistence module.
	handle
		.execute(
			"INSERT INTO agent_os_sessions (session_id, agent_type, capabilities, agent_info, created_at) \
			 VALUES (?, ?, ?, NULL, ?)"
				.to_owned(),
			Some(vec![
				BindParam::Text(session_id.to_owned()),
				BindParam::Text("claude".to_owned()),
				BindParam::Text("{}".to_owned()),
				BindParam::Integer(1_000),
			]),
		)
		.await?;

	handle
		.execute(
			INSERT_SESSION_EVENT_SQL.to_owned(),
			Some(vec![
				BindParam::Text(session_id.to_owned()),
				BindParam::Text(session_id.to_owned()),
				BindParam::Text(r#"{"method":"session/update"}"#.to_owned()),
				BindParam::Integer(1_001),
			]),
		)
		.await?;

	assert_eq!(
		count_session_events(&handle, session_id).await?,
		1,
		"row should be visible within generation 1"
	);

	// --- Sleep: tear the SQLite handle down (models VM teardown on Sleep). ---
	handle.close().await?;
	drop(handle);

	// --- Wake: reopen a fresh handle over the SAME durable depot store. ---
	let woken = depot.open_handle().await?;
	// migrate is idempotent; the actor reruns it on every start.
	woken.exec(MIGRATION_SQL.to_owned()).await?;

	assert_eq!(
		count_session_events(&woken, session_id).await?,
		1,
		"agent_os_session_events row must survive Sleep + wake"
	);

	// The actual event payload must round-trip unchanged.
	let event = woken
		.execute(
			"SELECT event FROM agent_os_session_events WHERE session_id = ? ORDER BY seq"
				.to_owned(),
			Some(vec![BindParam::Text(session_id.to_owned())]),
		)
		.await?;
	let event_text = match event.rows.first().and_then(|row| row.first()) {
		Some(ColumnValue::Text(text)) => text.clone(),
		other => anyhow::bail!("unexpected event column: {other:?}"),
	};
	assert_eq!(event_text, r#"{"method":"session/update"}"#);

	// A second event after wake gets seq 1 (MAX(seq)+1), proving the atomic
	// allocator reads the surviving row.
	woken
		.execute(
			INSERT_SESSION_EVENT_SQL.to_owned(),
			Some(vec![
				BindParam::Text(session_id.to_owned()),
				BindParam::Text(session_id.to_owned()),
				BindParam::Text(r#"{"method":"session/update","after":"wake"}"#.to_owned()),
				BindParam::Integer(2_000),
			]),
		)
		.await?;
	assert_eq!(count_session_events(&woken, session_id).await?, 2);

	woken.close().await?;
	Ok(())
}

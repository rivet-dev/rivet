import type { RawAccess } from "@/db/config";

export async function migrateSandboxTables(db: RawAccess): Promise<void> {
	// Legacy tables from an earlier naming convention. Safe to drop because
	// they were never shipped in a release and contain no user data.
	await db.execute(`
		DROP TABLE IF EXISTS sandbox_actor_meta;
		DROP TABLE IF EXISTS sandbox_actor_sessions;
	`);

	await db.execute(`
		CREATE TABLE IF NOT EXISTS sandbox_agent_sessions (
			id TEXT PRIMARY KEY,
			created_at INTEGER NOT NULL,
			record_json TEXT NOT NULL
		);

		CREATE TABLE IF NOT EXISTS sandbox_agent_events (
			id TEXT PRIMARY KEY,
			session_id TEXT NOT NULL,
			event_index INTEGER NOT NULL,
			created_at INTEGER NOT NULL,
			connection_id TEXT NOT NULL,
			sender TEXT NOT NULL,
			payload_json TEXT NOT NULL,
			raw_payload_json TEXT,
			UNIQUE(session_id, event_index)
		);

		CREATE INDEX IF NOT EXISTS sandbox_agent_sessions_created_at_idx
			ON sandbox_agent_sessions (created_at DESC);

		CREATE INDEX IF NOT EXISTS sandbox_agent_events_session_event_index_idx
			ON sandbox_agent_events (session_id, event_index ASC);
	`);
}

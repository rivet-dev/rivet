import type { RawAccess } from "@/db/config";

export async function migrateAgentOsTables(db: RawAccess): Promise<void> {
	await db.execute(`
		CREATE TABLE IF NOT EXISTS agent_os_preview_tokens (
			token TEXT PRIMARY KEY,
			port INTEGER NOT NULL,
			created_at INTEGER NOT NULL,
			expires_at INTEGER NOT NULL
		);

		CREATE INDEX IF NOT EXISTS idx_preview_tokens_expires_at
			ON agent_os_preview_tokens(expires_at);

		CREATE TABLE IF NOT EXISTS agent_os_fs_entries (
			path TEXT PRIMARY KEY,
			is_directory INTEGER NOT NULL DEFAULT 0,
			content BLOB,
			mode INTEGER NOT NULL DEFAULT 33188,
			uid INTEGER NOT NULL DEFAULT 0,
			gid INTEGER NOT NULL DEFAULT 0,
			size INTEGER NOT NULL DEFAULT 0,
			atime_ms INTEGER NOT NULL,
			mtime_ms INTEGER NOT NULL,
			ctime_ms INTEGER NOT NULL,
			birthtime_ms INTEGER NOT NULL,
			symlink_target TEXT,
			nlink INTEGER NOT NULL DEFAULT 1
		);

		CREATE INDEX IF NOT EXISTS idx_fs_entries_parent
			ON agent_os_fs_entries(path);

		CREATE TABLE IF NOT EXISTS agent_os_sessions (
			session_id TEXT PRIMARY KEY,
			agent_type TEXT NOT NULL,
			capabilities TEXT NOT NULL,
			agent_info TEXT,
			created_at INTEGER NOT NULL
		);

		CREATE TABLE IF NOT EXISTS agent_os_session_events (
			id INTEGER PRIMARY KEY AUTOINCREMENT,
			session_id TEXT NOT NULL,
			seq INTEGER NOT NULL,
			event TEXT NOT NULL,
			created_at INTEGER NOT NULL,
			FOREIGN KEY (session_id) REFERENCES agent_os_sessions(session_id) ON DELETE CASCADE
		);

		CREATE INDEX IF NOT EXISTS idx_session_events_session_seq
			ON agent_os_session_events(session_id, seq);
	`);
}

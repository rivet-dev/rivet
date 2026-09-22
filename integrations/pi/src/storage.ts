import type {
	FileEntry,
	SessionEntry,
	SessionHeader,
	SettingsManager,
} from "@earendil-works/pi-coding-agent";
import type { RawAccess } from "rivetkit/db";
import { migrations } from "rivetkit/unstable/migrations";

export type PiDatabase = Pick<RawAccess, "execute">;

export type PiSettings = ReturnType<SettingsManager["getGlobalSettings"]>;

/** A Pi session as stored in the actor's SQLite database. */
export interface StoredPiSession {
	sessionId: string;
	cwd: string;
	header: SessionHeader;
	entries: SessionEntry[];
	settings: PiSettings | undefined;
}

/**
 * The sandbox a Pi actor's tools run in. It is stored on its own row, written
 * as soon as the provider creates it, so a failed first start never loses it.
 */
export interface StoredSandbox {
	provider: string;
	id: string;
}

/**
 * One row per Pi session entry, inserted as Pi appends them. The rows are Pi's
 * own JSON, so `SessionManager.inMemory(cwd, { id }, [header, ...entries])`
 * restores the session and Pi migrates old entry versions itself.
 */
export const migratePiTables = migrations({
	tableName: "pi_schema_version",
	migrations: [
		{
			version: 1,
			sql: `
				CREATE TABLE pi_session (
					singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
					session_id TEXT NOT NULL CHECK (length(session_id) > 0),
					cwd TEXT NOT NULL CHECK (substr(cwd, 1, 1) = '/'),
					header_json TEXT NOT NULL CHECK (json_valid(header_json)),
					settings_json TEXT CHECK (
						settings_json IS NULL OR json_valid(settings_json)
					),
					created_at INTEGER NOT NULL,
					updated_at INTEGER NOT NULL
				) STRICT;

				CREATE TABLE pi_sandbox (
					singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
					provider TEXT NOT NULL,
					sandbox_id TEXT NOT NULL,
					created_at INTEGER NOT NULL
				) STRICT;

				CREATE TABLE pi_entry (
					seq INTEGER PRIMARY KEY,
					entry_id TEXT NOT NULL UNIQUE,
					entry_json TEXT NOT NULL CHECK (json_valid(entry_json))
				) STRICT;
			`,
		},
	],
});

export async function loadPiSession(
	db: PiDatabase,
): Promise<StoredPiSession | undefined> {
	const rows = await db.execute<{
		session_id: string;
		cwd: string;
		header_json: string;
		settings_json: string | null;
	}>(
		`SELECT session_id, cwd, header_json, settings_json
		 FROM pi_session
		 WHERE singleton = 1`,
	);
	const row = rows[0];
	if (!row) return undefined;

	const entryRows = await db.execute<{ entry_json: string }>(
		`SELECT entry_json FROM pi_entry ORDER BY seq`,
	);
	return {
		sessionId: row.session_id,
		cwd: row.cwd,
		header: JSON.parse(row.header_json) as SessionHeader,
		entries: entryRows.map(
			(entry) => JSON.parse(entry.entry_json) as SessionEntry,
		),
		settings:
			row.settings_json === null
				? undefined
				: (JSON.parse(row.settings_json) as PiSettings),
	};
}

export async function createPiSession(
	db: PiDatabase,
	session: {
		header: SessionHeader;
		cwd: string;
		settings: PiSettings;
	},
): Promise<void> {
	const now = Date.now();
	await db.execute(
		`INSERT INTO pi_session (
			singleton, session_id, cwd, header_json, settings_json, created_at, updated_at
		 ) VALUES (1, ?, ?, ?, ?, ?, ?)`,
		session.header.id,
		session.cwd,
		JSON.stringify(session.header),
		JSON.stringify(session.settings),
		now,
		now,
	);
}

export async function loadPiSandbox(
	db: PiDatabase,
): Promise<StoredSandbox | undefined> {
	const rows = await db.execute<{ provider: string; sandbox_id: string }>(
		`SELECT provider, sandbox_id FROM pi_sandbox WHERE singleton = 1`,
	);
	const row = rows[0];
	return row && { provider: row.provider, id: row.sandbox_id };
}

export async function savePiSandbox(
	db: PiDatabase,
	sandbox: StoredSandbox,
): Promise<void> {
	await db.execute(
		`INSERT INTO pi_sandbox (singleton, provider, sandbox_id, created_at) VALUES (1, ?, ?, ?)
		 ON CONFLICT (singleton) DO UPDATE SET
			provider = excluded.provider,
			sandbox_id = excluded.sandbox_id,
			created_at = excluded.created_at`,
		sandbox.provider,
		sandbox.id,
		Date.now(),
	);
}

export async function appendPiEntry(
	db: PiDatabase,
	entry: SessionEntry,
): Promise<void> {
	await db.execute(
		`INSERT OR IGNORE INTO pi_entry (entry_id, entry_json) VALUES (?, ?)`,
		entry.id,
		JSON.stringify(entry),
	);
}

export async function savePiSettings(
	db: PiDatabase,
	settings: PiSettings,
): Promise<void> {
	await db.execute(
		`UPDATE pi_session SET settings_json = ?, updated_at = ? WHERE singleton = 1`,
		JSON.stringify(settings),
		Date.now(),
	);
}

/** Builds the entry list `SessionManager.inMemory` expects: header first. */
export function toFileEntries(stored: StoredPiSession): FileEntry[] {
	return [stored.header, ...stored.entries];
}

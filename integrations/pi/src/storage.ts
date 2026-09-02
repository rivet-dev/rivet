import type {
	SessionEntry,
	SessionHeader,
	SessionManager,
	SettingsManager,
} from "@earendil-works/pi-coding-agent";
import type { RawAccess } from "rivetkit/db";
import { migrations } from "rivetkit/unstable/migrations";
import type { SandboxBinding } from "@rivet-dev/sandbox-adapter";

export interface StoredPiSession {
	sessionId: string;
	cwd: string;
	transcript: string;
	sandboxBinding?: SandboxBinding;
	settings?: ReturnType<SettingsManager["getGlobalSettings"]>;
}

export const migratePiActorTables = migrations({
	tableName: "rivet_pi_schema_version",
	migrations: [
		{
			version: 1,
			sql: `
				CREATE TABLE rivet_pi_session (
					singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
					session_id TEXT NOT NULL CHECK (length(session_id) > 0),
					cwd TEXT NOT NULL CHECK (substr(cwd, 1, 1) = '/'),
					transcript TEXT NOT NULL CHECK (length(transcript) > 0),
					sandbox_binding_json TEXT CHECK (
						sandbox_binding_json IS NULL OR json_valid(sandbox_binding_json)
					),
					settings_json TEXT CHECK (
						settings_json IS NULL OR json_valid(settings_json)
					),
					created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
					updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= created_at_ms)
				) STRICT;
			`,
		},
	],
});

export async function loadPiSession(
	database: Pick<RawAccess, "execute">,
): Promise<StoredPiSession | undefined> {
	const rows = await database.execute<{
		session_id: string;
		cwd: string;
		transcript: string;
		sandbox_binding_json: string | null;
		settings_json: string | null;
	}>(
		`SELECT session_id, cwd, transcript, sandbox_binding_json, settings_json
		 FROM rivet_pi_session
		 WHERE singleton = 1`,
	);
	const row = rows[0];
	if (!row) return undefined;
	return {
		sessionId: row.session_id,
		cwd: row.cwd,
		transcript: row.transcript,
		sandboxBinding:
			row.sandbox_binding_json === null
				? undefined
				: (JSON.parse(row.sandbox_binding_json) as SandboxBinding),
		settings:
			row.settings_json === null
				? undefined
				: (JSON.parse(row.settings_json) as ReturnType<
						SettingsManager["getGlobalSettings"]
					>),
	};
}

export async function savePiSession(
	database: Pick<RawAccess, "execute">,
	session: StoredPiSession,
): Promise<void> {
	const now = Date.now();
	await database.execute(
		`INSERT INTO rivet_pi_session (
			singleton, session_id, cwd, transcript, sandbox_binding_json, settings_json,
			created_at_ms, updated_at_ms
		 ) VALUES (1, ?, ?, ?, ?, ?, ?, ?)
		 ON CONFLICT(singleton) DO UPDATE SET
			session_id = excluded.session_id,
			cwd = excluded.cwd,
			transcript = excluded.transcript,
			sandbox_binding_json = excluded.sandbox_binding_json,
			settings_json = excluded.settings_json,
			updated_at_ms = excluded.updated_at_ms`,
		session.sessionId,
		session.cwd,
		session.transcript,
		session.sandboxBinding === undefined
			? null
			: JSON.stringify(session.sandboxBinding),
		session.settings === undefined ? null : JSON.stringify(session.settings),
		now,
		now,
	);
}

export function serializeSession(manager: SessionManager): string {
	const header = manager.getHeader();
	if (!header) {
		throw new Error("Pi did not provide a session header to persist");
	}
	return serializeTranscript(header, manager.getEntries());
}

export function serializeTranscript(
	header: SessionHeader,
	entries: readonly SessionEntry[],
): string {
	return `${[header, ...entries].map((entry) => JSON.stringify(entry)).join("\n")}\n`;
}

import { actor, setup } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

export type UserSessionPreferences = {
	theme: "light" | "dark";
	language: "en" | "es" | "fr";
};

export type UserSessionActivity = {
	page: string;
	timestamp: number;
};

export type UserSessionState = {
	region: string;
	preferences: UserSessionPreferences;
	recentActivity: UserSessionActivity[];
	lastLoginAt: number;
};

interface UserSessionInput {
	region: string;
}

const MAX_ACTIVITY = 6;

export const userSession = actor({
	// Session data is saved in the actor's SQLite database, which lives in the
	// actor's region: https://rivet.dev/docs/actors/sqlite
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS session (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					region TEXT NOT NULL,
					theme TEXT NOT NULL,
					language TEXT NOT NULL,
					last_login_at INTEGER NOT NULL
				)
			`);
			await db.execute(`
				CREATE TABLE IF NOT EXISTS activity (
					seq INTEGER PRIMARY KEY AUTOINCREMENT,
					page TEXT NOT NULL,
					timestamp INTEGER NOT NULL
				)
			`);
		},
	}),

	// Save the session with a region-specific input parameter once, when the
	// actor is created: https://rivet.dev/docs/actors/input
	onCreate: async (c, input: UserSessionInput) => {
		await c.db.execute(
			"INSERT INTO session (id, region, theme, language, last_login_at) VALUES (1, ?, 'light', 'en', ?)",
			input.region,
			Date.now(),
		);
	},

	actions: {
		// Read session data from SQLite.
		getSession: (c) => readSession(c.db),

		// Update only the preferences that were passed and save them to SQLite.
		updatePreferences: async (
			c,
			preferences: Partial<UserSessionPreferences>,
		) => {
			return await c.db.transaction(
				async (tx) => {
					await tx.execute(
						"UPDATE session SET theme = COALESCE(?, theme), language = COALESCE(?, language) WHERE id = 1",
						preferences.theme || null,
						preferences.language || null,
					);
					return await readSession(tx);
				},
				{ name: "update-preferences" },
			);
		},

		// Log activity and keep a short history of recent pages.
		logActivity: async (c, entry: { page: string; isLogin?: boolean }) => {
			const timestamp = Date.now();
			return await c.db.transaction(
				async (tx) => {
					await tx.execute(
						"INSERT INTO activity (page, timestamp) VALUES (?, ?)",
						entry.page,
						timestamp,
					);
					await tx.execute(
						"DELETE FROM activity WHERE seq NOT IN (SELECT seq FROM activity ORDER BY seq DESC LIMIT ?)",
						MAX_ACTIVITY,
					);
					if (entry.isLogin) {
						await tx.execute(
							"UPDATE session SET last_login_at = ? WHERE id = 1",
							timestamp,
						);
					}
					return await readSession(tx);
				},
				{ name: "log-activity" },
			);
		},

		// Return the actor's region so the UI can show data locality.
		getRegion: async (c) => {
			const [row] = await c.db.execute<{ region: string }>(
				"SELECT region FROM session WHERE id = 1",
			);
			return row.region;
		},
	},
});

async function readSession(db: RawAccess): Promise<UserSessionState> {
	const [row] = await db.execute<{
		region: string;
		theme: UserSessionPreferences["theme"];
		language: UserSessionPreferences["language"];
		lastLoginAt: number;
	}>(
		"SELECT region, theme, language, last_login_at AS lastLoginAt FROM session WHERE id = 1",
	);
	const recentActivity = await db.execute<UserSessionActivity>(
		"SELECT page, timestamp FROM activity ORDER BY seq DESC",
	);
	return {
		region: row.region,
		preferences: { theme: row.theme, language: row.language },
		recentActivity,
		lastLoginAt: row.lastLoginAt,
	};
}

// Register actors and start the server: https://rivet.dev/docs/setup
export const registry = setup({
	use: { userSession },
});

registry.start();

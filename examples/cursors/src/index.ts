import { actor, event, setup } from "rivetkit";
import { db } from "rivetkit/db";

export interface CursorPosition {
	userId: string;
	x: number;
	y: number;
	timestamp: number;
}

export type TextLabel = {
	id: string;
	userId: string;
	text: string;
	x: number;
	y: number;
	timestamp: number;
};

// docs:start room
export const cursorRoom = actor({
	// Text labels belong to the room, so they are saved in SQLite.
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS text_labels (
					id TEXT PRIMARY KEY,
					user_id TEXT NOT NULL,
					text TEXT NOT NULL,
					x REAL NOT NULL,
					y REAL NOT NULL,
					timestamp INTEGER NOT NULL
				)
			`);
		},
	}),
	events: {
		cursorMoved: event<CursorPosition>(),
		cursorRemoved: event<CursorPosition>(),
		textUpdated: event<TextLabel>(),
		textRemoved: event<string>(),
	},

	// Cursors belong to live connections, so they stay in memory keyed by connection ID.
	createVars: () => ({
		cursors: new Map<string, CursorPosition>(),
	}),
	// docs:end room

	actions: {
		// docs:start cursor
		// Update cursor position
		updateCursor: (c, userId: string, x: number, y: number) => {
			const cursor: CursorPosition = {
				userId,
				x,
				y,
				timestamp: Date.now(),
			};
			c.vars.cursors.set(c.conn.id, cursor);
			c.broadcast("cursorMoved", cursor);
			return cursor;
		},
		// docs:end cursor

		// docs:start labels
		// Update text on the canvas (creates or updates)
		updateText: async (
			c,
			id: string,
			userId: string,
			text: string,
			x: number,
			y: number,
		) => {
			const textLabel: TextLabel = {
				id,
				userId,
				text,
				x,
				y,
				timestamp: Date.now(),
			};

			// Insert a new text label or replace the existing one with the same ID
			await c.db.execute(
				"INSERT OR REPLACE INTO text_labels (id, user_id, text, x, y, timestamp) VALUES (?, ?, ?, ?, ?, ?)",
				textLabel.id,
				textLabel.userId,
				textLabel.text,
				textLabel.x,
				textLabel.y,
				textLabel.timestamp,
			);

			c.broadcast("textUpdated", textLabel);
			return textLabel;
		},

		// Remove text from the canvas
		removeText: async (c, id: string) => {
			await c.db.execute("DELETE FROM text_labels WHERE id = ?", id);
			c.broadcast("textRemoved", id);
		},
		// docs:end labels

		// docs:start presence
		// Get all room state (cursors and text labels)
		getRoomState: async (c) => {
			const cursors: Record<string, CursorPosition> = {};
			for (const cursor of c.vars.cursors.values()) {
				cursors[cursor.userId] = cursor;
			}
			const textLabels = (await c.db.execute(
				"SELECT id, user_id AS userId, text, x, y, timestamp FROM text_labels ORDER BY timestamp",
			)) as TextLabel[];
			return {
				cursors,
				textLabels,
			};
		},
		// docs:end presence
	},

	// docs:start presence
	onDisconnect: (c, conn) => {
		const cursor = c.vars.cursors.get(conn.id);
		c.vars.cursors.delete(conn.id);
		if (cursor) {
			c.broadcast("cursorRemoved", cursor);
		}
	},
	// docs:end presence
});

// docs:start registry
// Register actors and start the server: https://rivet.dev/docs/setup
export const registry = setup({
	use: { cursorRoom },
});

registry.start();
// docs:end registry

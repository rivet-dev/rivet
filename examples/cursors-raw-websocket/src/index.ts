import {
	actor,
	type RivetMessageEvent,
	setup,
	type UniversalWebSocket,
} from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

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

interface Vars {
	websockets: Map<
		string,
		{ socket: UniversalWebSocket; cursor: CursorPosition | null }
	>;
}

/** Reads every text label saved in the room, oldest first. */
async function loadTextLabels(db: RawAccess): Promise<TextLabel[]> {
	return (await db.execute(
		"SELECT id, user_id AS userId, text, x, y, timestamp FROM text_labels ORDER BY timestamp",
	)) as TextLabel[];
}

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

	createVars: (): Vars => {
		return {
			websockets: new Map(),
		};
	},

	actions: {
		// Get or create the actor (for frontend to resolve actor ID)
		getOrCreate: () => {
			return { status: "ok" };
		},

		// Get all room state (cursors and text labels)
		getRoomState: async (c) => {
			const cursors: Record<string, CursorPosition> = {};
			for (const [sessionId, { cursor }] of c.vars.websockets.entries()) {
				if (cursor) {
					cursors[sessionId] = cursor;
				}
			}
			return {
				cursors,
				textLabels: await loadTextLabels(c.db),
			};
		},
	},

	// Handle WebSocket connections
	onWebSocket: async (c, websocket: UniversalWebSocket) => {
		if (!c.request) {
			websocket.close(1008, "no request");
			return;
		}

		const url = new URL(c.request.url);
		const sessionId = url.searchParams.get("sessionId");

		if (!sessionId) {
			websocket.close(1008, "Missing sessionId");
			return;
		}

		console.log(
			`websocket connected: sessionId=${sessionId}, actorId=${c.actorId}`,
		);

		// Store the websocket
		c.vars.websockets.set(sessionId, { socket: websocket, cursor: null });

		// Send the current cursors and text labels to the new connection
		const textLabels = await loadTextLabels(c.db);
		const cursors: Record<string, CursorPosition> = {};
		for (const [id, { cursor }] of c.vars.websockets.entries()) {
			if (cursor) {
				cursors[id] = cursor;
			}
		}
		websocket.send(
			JSON.stringify({
				type: "init",
				data: {
					cursors,
					textLabels,
				},
			}),
		);

		// Handle incoming messages
		websocket.addEventListener("message", async (event: RivetMessageEvent) => {
			try {
				const message = JSON.parse(event.data as string);

				switch (message.type) {
					case "updateCursor": {
						const { userId, x, y } = message.data;
						const cursor: CursorPosition = {
							userId,
							x,
							y,
							timestamp: Date.now(),
						};

						// Update the cursor for this session
						const session = c.vars.websockets.get(sessionId);
						if (session) {
							session.cursor = cursor;
						}

						// Broadcast to all connections (including sender)
						for (const { socket } of c.vars.websockets.values()) {
							socket.send(
								JSON.stringify({
									type: "cursorMoved",
									data: cursor,
								}),
							);
						}
						break;
					}

					case "updateText": {
						const { id, userId, text, x, y } = message.data;
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

						// Broadcast to all connections
						for (const { socket } of c.vars.websockets.values()) {
							socket.send(
								JSON.stringify({
									type: "textUpdated",
									data: textLabel,
								}),
							);
						}
						break;
					}

					case "removeText": {
						const { id } = message.data;
						await c.db.execute("DELETE FROM text_labels WHERE id = ?", id);

						// Broadcast to all connections
						for (const { socket } of c.vars.websockets.values()) {
							socket.send(
								JSON.stringify({
									type: "textRemoved",
									data: id,
								}),
							);
						}
						break;
					}
				}
			} catch (error) {
				console.error("error handling websocket message:", error);
			}
		});

		// Handle connection close
		websocket.addEventListener("close", () => {
			console.log(`websocket disconnected: sessionId=${sessionId}`);
			const session = c.vars.websockets.get(sessionId);
			if (session?.cursor) {
				// Broadcast cursor removal to all other connections
				for (const [id, { socket }] of c.vars.websockets.entries()) {
					if (id !== sessionId) {
						socket.send(
							JSON.stringify({
								type: "cursorRemoved",
								data: session.cursor,
							}),
						);
					}
				}
			}
			c.vars.websockets.delete(sessionId);
		});
	},
});

// Register actors and start the server: https://rivet.dev/docs/setup
export const registry = setup({
	use: { cursorRoom },
});

registry.start();

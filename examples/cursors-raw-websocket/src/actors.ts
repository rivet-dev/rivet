import { actor, setup, type UniversalWebSocket } from "rivetkit";

export interface CursorPosition {
	userId: string;
	x: number;
	y: number;
	timestamp: number;
}

export interface TextLabel {
	id: string;
	userId: string;
	text: string;
	x: number;
	y: number;
	timestamp: number;
}

interface Vars {
	websockets: Map<
		string,
		{ socket: UniversalWebSocket; cursor: CursorPosition | null }
	>;
}

export const cursorRoom = actor({
	state: {
		textLabels: [] as TextLabel[],
	},

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
		getRoomState: (c) => {
			const cursors: Record<string, CursorPosition> = {};
			for (const [sessionId, { cursor }] of c.vars.websockets.entries()) {
				if (cursor) {
					cursors[sessionId] = cursor;
				}
			}
			return {
				cursors,
				textLabels: c.state.textLabels,
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

		// Send initial state to the new connection
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
					textLabels: c.state.textLabels,
				},
			}),
		);

		// Handle incoming messages
		websocket.addEventListener("message", (event) => {
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

						// Find and update existing text label or add new one
						const existingIndex = c.state.textLabels.findIndex(
							(label) => label.id === id,
						);
						if (existingIndex >= 0) {
							c.state.textLabels[existingIndex] = textLabel;
						} else {
							c.state.textLabels.push(textLabel);
						}

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
						c.state.textLabels = c.state.textLabels.filter(
							(label) => label.id !== id,
						);

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

// Register actors for use: https://rivet.dev/docs/setup
export const registry = setup({
	use: { cursorRoom },
});

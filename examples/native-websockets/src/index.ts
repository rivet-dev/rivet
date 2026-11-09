import {
	actor,
	type RivetMessageEvent,
	setup,
	type UniversalWebSocket,
} from "rivetkit";

interface Cursor {
	userId: string;
	x: number;
	y: number;
	timestamp: number;
}

interface Vars {
	websockets: Map<string, UniversalWebSocket>;
	cursors: Record<string, Cursor>;
}

export const cursorRoom = actor({
	// Websockets and cursor positions only matter while users are connected,
	// so both live in vars, which are never persisted.
	createVars: (): Vars => {
		return {
			websockets: new Map(),
			cursors: {},
		};
	},

	actions: {
		// Get or create the actor (for frontend to resolve actor ID)
		getOrCreate: () => {
			return { status: "ok" };
		},
	},

	// Handle WebSocket connections
	onWebSocket: async (c, websocket: UniversalWebSocket) => {
		// Extract userId from query parameters
		if (!c.request) {
			websocket.close(1008, "Missing request");
			return;
		}
		const url = new URL(c.request.url);
		const userId = url.searchParams.get("userId");

		// Validate userId exists
		if (!userId) {
			websocket.close(1008, "Missing userId query parameter");
			return;
		}

		console.log(
			`websocket connected: userId=${userId}, actorId=${c.actorId}`,
		);

		// Store websocket in vars (non-persistent)
		c.vars.websockets.set(userId, websocket);

		// Send the current cursors to the new connection
		websocket.send(
			JSON.stringify({
				type: "init",
				data: {
					cursors: c.vars.cursors,
				},
			}),
		);

		// Handle incoming messages
		websocket.addEventListener("message", (event: RivetMessageEvent) => {
			try {
				const message = JSON.parse(event.data as string);

				switch (message.type) {
					case "updateCursor": {
						const { x, y } = message.data;

						// Update the cursor position in memory
						c.vars.cursors[userId] = {
							userId,
							x,
							y,
							timestamp: Date.now(),
						};

						// Broadcast to all websockets
						for (const ws of c.vars.websockets.values()) {
							ws.send(
								JSON.stringify({
									type: "cursorUpdate",
									data: c.vars.cursors[userId],
								}),
							);
						}
						break;
					}

					case "getCursors": {
						// Send current cursor state to requesting client
						websocket.send(
							JSON.stringify({
								type: "cursorsState",
								data: {
									cursors: c.vars.cursors,
								},
							}),
						);
						break;
					}
				}
			} catch (error) {
				console.error("error handling websocket message:", error);
			}
		});

		// Handle connection close
		websocket.addEventListener("close", () => {
			console.log(`websocket disconnected: userId=${userId}`);
			// Clean up websocket from map
			c.vars.websockets.delete(userId);
		});
	},
});

// Register actors for use
export const registry = setup({
	use: { cursorRoom },
});

registry.start();

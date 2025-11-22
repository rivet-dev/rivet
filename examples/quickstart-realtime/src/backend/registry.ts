import { actor, setup } from "rivetkit";

export interface CursorPosition {
	userId: string;
	x: number;
	y: number;
}

export const cursorRoom = actor({
	// No persistent state needed for this simple example
	state: {},

	// Connection state stores each user's cursor position
	connState: {
		cursor: null as CursorPosition | null,
	},

	actions: {
		// Update cursor position and broadcast to all connected clients
		updateCursor: (c, userId: string, x: number, y: number) => {
			const cursor: CursorPosition = {
				userId,
				x,
				y,
			};
			// Store cursor in connection state
			c.conn.state.cursor = cursor;
			// Broadcast cursor update to all clients
			c.broadcast("cursorMoved", cursor);
			return cursor;
		},

		// Get all active cursors from all connections
		getCursors: (c) => {
			const cursors: CursorPosition[] = [];
			// Iterate through all connections and collect cursor data
			for (const conn of c.conns.values()) {
				if (conn.state.cursor) {
					cursors.push(conn.state.cursor);
				}
			}
			return cursors;
		},
	},
});

// Register actors for use: https://rivet.dev/docs/setup
export const registry = setup({
	use: { cursorRoom },
});

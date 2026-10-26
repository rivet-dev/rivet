import { actor, event, setup } from "rivetkit";

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

// docs:start room
export const cursorRoom = actor({
	state: {
		textLabels: [] as TextLabel[],
	},
	events: {
		cursorMoved: event<CursorPosition>(),
		cursorRemoved: event<CursorPosition>(),
		textUpdated: event<TextLabel>(),
		textRemoved: event<string>(),
	},

	connState: {
		cursor: null as CursorPosition | null,
	},
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
			c.conn.state.cursor = cursor;
			c.broadcast("cursorMoved", cursor);
			return cursor;
		},
		// docs:end cursor

		// docs:start labels
		// Update text on the canvas (creates or updates)
		updateText: (
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

			// Find and update existing text label or add new one
			const existingIndex = c.state.textLabels.findIndex(
				(label) => label.id === id,
			);
			if (existingIndex >= 0) {
				c.state.textLabels[existingIndex] = textLabel;
			} else {
				c.state.textLabels.push(textLabel);
			}

			c.broadcast("textUpdated", textLabel);
			return textLabel;
		},

		// Remove text from the canvas
		removeText: (c, id: string) => {
			c.state.textLabels = c.state.textLabels.filter(
				(label) => label.id !== id,
			);
			c.broadcast("textRemoved", id);
		},
		// docs:end labels

		// docs:start presence
		// Get all room state (cursors and text labels)
		getRoomState: (c) => {
			const cursors: Record<string, CursorPosition> = {};
			for (const conn of c.conns.values()) {
				if (conn.state.cursor) {
					cursors[conn.state.cursor.userId] = conn.state.cursor;
				}
			}
			return {
				cursors,
				textLabels: c.state.textLabels,
			};
		},
		// docs:end presence
	},

	// docs:start presence
	onDisconnect: (c, conn) => {
		if (conn.state.cursor) {
			c.broadcast("cursorRemoved", conn.state.cursor);
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

import { actor, setup } from "rivetkit";

export type Message = { sender: string; text: string; timestamp: number };

export const chatRoom = actor({
	options: {
		sleepTimeout: 2000,
	},
	// Persistent state that survives restarts
	state: {
		messages: [] as Message[],
	},

	actions: {
		// Callable functions from clients
		sendMessage: (c, sender: string, text: string) => {
			const message = { sender, text, timestamp: Date.now() };
			// State changes are automatically persisted
			c.state.messages.push(message);
			// Send events to all connected clients
			c.broadcast("newMessage", message);
			return message;
		},

		getHistory: (c) => c.state.messages,
	},
});

// Register actors for use
export const registry = setup({
	use: { chatRoom },
});

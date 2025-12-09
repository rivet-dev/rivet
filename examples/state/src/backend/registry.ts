import { actor, setup } from "rivetkit";

export type Message = {
	id: string;
	sender: string;
	text: string;
	timestamp: number;
};

export const chatRoom = actor({
	// Persistent state that survives restarts: https://rivet.dev/docs/actors/state
	state: {
		messages: [] as Message[],
	},

	actions: {
		// Callable functions from clients: https://rivet.dev/docs/actors/actions
		sendMessage: (c, sender: string, text: string) => {
			const message: Message = {
				id: crypto.randomUUID(),
				sender,
				text,
				timestamp: Date.now(),
			};
			// State changes are automatically persisted
			c.state.messages.push(message);
			// Send events to all connected clients: https://rivet.dev/docs/actors/events
			c.broadcast("newMessage", message);
			return message;
		},

		// Returns all messages for initial state loading
		getMessages: (c) => c.state.messages,

		// Clears all messages and notifies clients
		clearMessages: (c) => {
			c.state.messages = [];
			c.broadcast("messagesCleared");
			return { success: true };
		},
	},
});

// Register actors for use: https://rivet.dev/docs/setup
export const registry = setup({
	use: { chatRoom },
});

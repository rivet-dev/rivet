import { actor, setup, event } from "rivetkit";

export type Message = { sender: string; text: string; timestamp: number };

export const chatRoom = actor({
	state: {
		messages: [] as Message[],
	},
	events: {
		newMessage: event<Message>(),
	},

	actions: {
		sendMessage: (c, sender: string, text: string) => {
			const message = { sender, text, timestamp: Date.now() };
			c.state.messages.push(message);
			c.broadcast("newMessage", message);
			return message;
		},

		getHistory: (c) => c.state.messages,
	},
});

export const registry = setup({
	use: { chatRoom },
});

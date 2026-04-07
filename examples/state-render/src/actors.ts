import { actor, setup, event } from "rivetkit";

export type Message = {
	id: string;
	sender: string;
	text: string;
	timestamp: number;
};

export const chatRoom = actor({
	state: {
		messages: [] as Message[],
	},
	events: {
		newMessage: event<Message>(),
		messagesCleared: event<[]>(),
	},

	actions: {
		sendMessage: (c, sender: string, text: string) => {
			const message: Message = {
				id: crypto.randomUUID(),
				sender,
				text,
				timestamp: Date.now(),
			};
			c.state.messages.push(message);
			c.broadcast("newMessage", message);
			return message;
		},

		getMessages: (c) => c.state.messages,

		clearMessages: (c) => {
			c.state.messages = [];
			c.broadcast("messagesCleared");
			return { success: true };
		},
	},
});

export const registry = setup({
	use: { chatRoom },
});

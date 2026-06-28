import { actor, event } from "rivetkit";

const chatRoom = actor({
	state: { messages: [] as string[] },
	events: {
		newMessage: event<{ text: string }>(),
	},
	actions: {
		sendMessage: (c, text: string) => {
			// Broadcast to ALL connected clients
			c.broadcast("newMessage", { text });
		},
	},
});

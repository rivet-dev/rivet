import { actor, setup } from "rivetkit";

interface State {
	messages: Message[];
}

interface Message {
	id: string;
	userId: string;
	username: string;
	message: string;
	timestamp: number;
}

export const chatRoom = actor({
	state: {
		messages: [],
	} as State,
	actions: {
		sendMessage: (c, message: string) => {
			// TODO: Add back using onBeforeConnect
			// // Access Better Auth with c.conn.auth
			// const newMessage = {
			// 	id: crypto.randomUUID(),
			// 	userId: c.conn.auth.user.id,
			// 	username: c.conn.auth.user.name,
			// 	message,
			// 	timestamp: Date.now(),
			// };
			//
			// c.state.messages.push(newMessage);
			// c.broadcast("newMessage", newMessage);
			//
			// return newMessage;
		},
		getMessages: (c) => {
			return c.state.messages;
		},
	},
});

export const registry = setup({
	use: { chatRoom },
});

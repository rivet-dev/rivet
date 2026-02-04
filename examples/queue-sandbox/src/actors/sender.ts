import { actor } from "rivetkit";

export interface ReceivedMessage {
	name: string;
	body: unknown;
	receivedAt: number;
}

export const sender = actor({
	state: {
		messages: [] as ReceivedMessage[],
	},
	actions: {
		getMessages(c): ReceivedMessage[] {
			return c.state.messages;
		},
		async receiveOne(c) {
			const msg = await c.queue.next("task", { timeout: 100 });
			if (msg) {
				const received: ReceivedMessage = {
					name: msg.name,
					body: msg.body,
					receivedAt: Date.now(),
				};
				c.state.messages.push(received);
				c.broadcast("messageReceived", c.state.messages);
				return received;
			}
			return null;
		},
		clearMessages(c) {
			c.state.messages = [];
			c.broadcast("messageReceived", c.state.messages);
		},
	},
});

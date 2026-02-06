import { actor } from "rivetkit";

export interface QueueMessage {
	name: string;
	body: unknown;
}

export const multiQueue = actor({
	state: {
		messages: [] as QueueMessage[],
	},
	actions: {
		async receiveFromQueues(c, names: string[], count: number) {
			const msgs = await c.queue.next(names, { count, timeout: 100 });
			if (msgs && msgs.length > 0) {
				for (const msg of msgs) {
					c.state.messages.push({ name: msg.name, body: msg.body });
				}
				c.broadcast("messagesReceived", c.state.messages);
			}
			return msgs ?? [];
		},
		getMessages(c): QueueMessage[] {
			return c.state.messages;
		},
		clearMessages(c) {
			c.state.messages = [];
			c.broadcast("messagesReceived", c.state.messages);
		},
	},
});

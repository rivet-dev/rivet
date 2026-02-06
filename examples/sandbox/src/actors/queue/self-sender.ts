import { actor } from "rivetkit";

export interface SelfSenderState {
	sentCount: number;
	receivedCount: number;
	messages: unknown[];
}

export const selfSender = actor({
	state: {
		sentCount: 0,
		receivedCount: 0,
		messages: [] as unknown[],
	},
	actions: {
		async receiveFromSelf(c) {
			const msg = await c.queue.next("self", { timeout: 100 });
			if (msg) {
				c.state.receivedCount += 1;
				c.state.messages.push(msg.body);
				c.broadcast("received", {
					receivedCount: c.state.receivedCount,
					message: msg.body,
				});
				return msg.body;
			}
			return null;
		},
		getState(c): SelfSenderState {
			return {
				sentCount: c.state.sentCount,
				receivedCount: c.state.receivedCount,
				messages: c.state.messages,
			};
		},
		clearMessages(c) {
			c.state.sentCount = 0;
			c.state.receivedCount = 0;
			c.state.messages = [];
			c.broadcast("sent", { sentCount: 0 });
			c.broadcast("received", { receivedCount: 0, message: null });
		},
		incrementSentCount(c) {
			c.state.sentCount += 1;
			c.broadcast("sent", { sentCount: c.state.sentCount });
		},
	},
});

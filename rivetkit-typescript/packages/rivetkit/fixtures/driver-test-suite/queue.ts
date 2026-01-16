import { actor } from "rivetkit";
import type { registry } from "./registry";

export const queueActor = actor({
	state: {},
	actions: {
		receiveOne: async (
			c,
			name: string,
			opts?: { count?: number; timeout?: number },
		) => {
			const message = await c.queue.next(name, opts);
			if (!message) {
				return null;
			}
			return { name: message.name, body: message.body };
		},
		receiveMany: async (
			c,
			names: string[],
			opts?: { count?: number; timeout?: number },
		) => {
			const messages = await c.queue.next(names, opts);
			return (messages ?? []).map(
				(message: { name: string; body: unknown }) => ({
					name: message.name,
					body: message.body,
				}),
			);
		},
		receiveRequest: async (
			c,
			request: {
				name: string | string[];
				count?: number;
				timeout?: number;
			},
		) => {
			const messages = await c.queue.next(request);
			return (messages ?? []).map(
				(message: { name: string; body: unknown }) => ({
					name: message.name,
					body: message.body,
				}),
			);
		},
		sendToSelf: async (c, name: string, body: unknown) => {
			const client = c.client<typeof registry>();
			const handle = client.queueActor.getForId(c.actorId);
			await handle.queue[name].send(body);
			return true;
		},
		waitForAbort: async (c) => {
			setTimeout(() => {
				c.destroy();
			}, 10);
			await c.queue.next("abort", { timeout: 10_000 });
			return true;
		},
	},
});

export const queueLimitedActor = actor({
	state: {},
	actions: {},
	options: {
		maxQueueSize: 1,
		maxQueueMessageSize: 64,
	},
});

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
		receiveAndComplete: async (c, name: string) => {
			const message = await c.queue.next(name, { wait: true });
			if (!message) {
				return null;
			}
			await message.complete({ echo: message.body });
			return { name: message.name, body: message.body };
		},
		receiveAndCompleteTwice: async (c, name: string) => {
			const message = await c.queue.next(name, { wait: true });
			if (!message) {
				return null;
			}
			await message.complete({ ok: true });
			try {
				await message.complete({ ok: false });
				return { ok: false };
			} catch (error) {
				const actorError = error as { group?: string; code?: string };
				return { group: actorError.group, code: actorError.code };
			}
		},
		receiveWithoutWaitComplete: async (c, name: string) => {
			const message = await c.queue.next(name);
			if (!message) {
				return null;
			}
			try {
				await message.complete();
				return { ok: false };
			} catch (error) {
				const actorError = error as { group?: string; code?: string };
				return { group: actorError.group, code: actorError.code };
			}
		},
		receiveWhilePending: async (c, name: string) => {
			const message = await c.queue.next(name, { wait: true });
			if (!message) {
				return null;
			}
			let errorPayload: { group?: string; code?: string } | undefined;
			try {
				await c.queue.next(name);
			} catch (error) {
				const actorError = error as { group?: string; code?: string };
				errorPayload = {
					group: actorError.group,
					code: actorError.code,
				};
			}
			await message.complete({ ok: true });
			return errorPayload ?? { ok: false };
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

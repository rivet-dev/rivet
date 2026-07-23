// @ts-nocheck
import { actor, queue } from "rivetkit";
import type { registry } from "./registry-static";

const queueSchemas = {
	greeting: queue<{ hello: string }>(),
	self: queue<{ value: number }>(),
	a: queue<number>(),
	b: queue<number>(),
	c: queue<number>(),
	one: queue<string>(),
	two: queue<string>(),
	missing: queue<unknown>(),
	abort: queue<unknown>(),
	tasks: queue<{ value: number }>(),
	timeout: queue<{ value: number }>(),
	nowait: queue<{ value: string }>(),
	twice: queue<{ value: string }>(),
	handled: queue<{ value: number }>({
		onMessage: async (c, message) => {
			c.state.handled = (c.state.handled ?? 0) + message.body.value;
		},
	}),
	retrying: queue<{ value: number }>({
		retry: {
			maxAttempts: 3,
			backoff: { initialMs: 1, factor: 1, maxMs: 1, jitter: false },
		},
		onMessage: async (c) => {
			c.state.retryAttempts = (c.state.retryAttempts ?? 0) + 1;
			if (c.state.retryAttempts < 3) throw new Error("retry me");
		},
	}),
	timedHandler: queue<{ value: number }>({
		timeout: 25,
		retry: {
			maxAttempts: 2,
			backoff: { initialMs: 1, factor: 1, maxMs: 1, jitter: false },
		},
		onMessage: async (c, _message, { signal }) => {
			c.state.timeoutAttempts = (c.state.timeoutAttempts ?? 0) + 1;
			if (c.state.timeoutAttempts === 1) {
				await new Promise<void>(() => {
					signal.addEventListener(
						"abort",
						() => {
							c.state.timeoutAborted = true;
						},
						{ once: true },
					);
				});
			}
		},
	}),
	dead: queue<{ value: number }>({
		retry: {
			maxAttempts: 2,
			backoff: { initialMs: 1, factor: 1, maxMs: 1, jitter: false },
		},
		onMessage: async () => {
			throw new Error("always fails");
		},
		onDeadLetter: async (c, message) => {
			c.state.deadLettered = message.id;
		},
	}),
} as const;

type QueueName = keyof typeof queueSchemas;

export const queueActor = actor({
	state: {
		handled: 0,
		retryAttempts: 0,
		timeoutAttempts: 0,
		timeoutAborted: false,
		deadLettered: undefined as string | undefined,
	},
	queues: queueSchemas,
	actions: {
		receiveOne: async (c, name: QueueName, opts?: { timeout?: number }) => {
			const message = await c.queue.next({
				names: [name],
				timeout: opts?.timeout,
			});
			if (!message) {
				return null;
			}
			return { name: message.name, body: message.body };
		},
		receiveMany: async (
			c,
			names: QueueName[],
			opts?: { count?: number; timeout?: number },
		) => {
			const messages = await c.queue.nextBatch({
				names,
				count: opts?.count,
				timeout: opts?.timeout,
			});
			return messages.map((message) => ({
				name: message.name,
				body: message.body,
			}));
		},
		receiveRequest: async (
			c,
			request: {
				names?: QueueName[];
				count?: number;
				timeout?: number;
			},
		) => {
			const messages = await c.queue.nextBatch(request);
			return messages.map((message) => ({
				name: message.name,
				body: message.body,
			}));
		},
		tryReceiveMany: async (
			c,
			request: {
				names?: QueueName[];
				count?: number;
			},
		) => {
			const messages = await c.queue.tryNextBatch(request);
			return messages.map((message) => ({
				name: message.name,
				body: message.body,
			}));
		},
		receiveWithIterator: async (c, name: QueueName) => {
			for await (const message of c.queue.iter({ names: [name] })) {
				return { name: message.name, body: message.body };
			}
			return null;
		},
		receiveWithAsyncIterator: async (c) => {
			for await (const message of c.queue.iter()) {
				return { name: message.name, body: message.body };
			}
			return null;
		},
		sendToSelf: async (c, name: QueueName, body: unknown) => {
			const client = c.client<typeof registry>();
			const handle = client.queueActor.getForId(c.actorId);
			await handle.send(name, body);
			return true;
		},
		waitForAbort: async (c) => {
			setTimeout(() => {
				c.destroy();
			}, 10);
			await c.queue.next({ names: ["abort"], timeout: 10_000 });
			return true;
		},
		waitForSignalAbort: async (c) => {
			const controller = new AbortController();
			controller.abort();
			try {
				await c.queue.next({
					names: ["abort"],
					timeout: 10_000,
					signal: controller.signal,
				});
				return { ok: false };
			} catch (error) {
				const actorError = error as { group?: string; code?: string };
				return { group: actorError.group, code: actorError.code };
			}
		},
		waitForActorAbortWithSignal: async (c) => {
			const controller = new AbortController();
			setTimeout(() => {
				c.destroy();
			}, 10);
			try {
				await c.queue.next({
					names: ["abort"],
					timeout: 10_000,
					signal: controller.signal,
				});
				return { ok: false };
			} catch (error) {
				const actorError = error as { group?: string; code?: string };
				return { group: actorError.group, code: actorError.code };
			}
		},
		iterWithSignalAbort: async (c) => {
			const controller = new AbortController();
			controller.abort();
			try {
				for await (const _message of c.queue.iter({
					names: ["abort"],
					signal: controller.signal,
				})) {
					return { ok: false };
				}
				return { ok: true };
			} catch (error) {
				const actorError = error as { group?: string; code?: string };
				if (
					actorError.group === "actor" &&
					actorError.code === "aborted"
				) {
					return { ok: true };
				}
				throw error;
			}
		},
		getQueueState: (c) => c.state,
	},
});

export const queueLimitedActor = actor({
	state: {},
	queues: {
		message: queue<number>(),
		oversize: queue<string>(),
	},
	actions: {},
	options: {
		maxQueueSize: 1,
		maxQueueMessageSize: 64,
	},
});

export const MANY_QUEUE_NAMES = Array.from(
	{ length: 32 },
	(_, i) => `cmd.${i}` as const,
);

const manyQueueSchemas = Object.fromEntries(
	MANY_QUEUE_NAMES.map((name) => [
		name,
		queue<{ index: number }>({
			onMessage: async (c, message) => {
				c.state.started = true;
				c.state.processed.push(message.name);
			},
		}),
	]),
);

export const manyQueueChildActor = actor({
	queues: manyQueueSchemas,
	actions: {
		ping: (c) => ({ label: c.state.label, pong: true }),
		getSnapshot: (c) => c.state,
	},
	createState: (_c, label: string) => ({
		label,
		started: false,
		processed: [] as string[],
	}),
});

export const manyQueueActionParentActor = actor({
	state: {
		spawned: [] as string[],
	},
	actions: {
		spawnChild: async (c, key: string) => {
			const client = c.client<typeof registry>();
			await client.manyQueueChildActor.getOrCreate([key], {
				createWithInput: key,
			});
			c.state.spawned.push(key);
			return { key };
		},
		getSpawned: (c) => c.state.spawned,
	},
});

export const manyQueueRunParentActor = actor({
	state: {
		spawned: [] as string[],
	},
	queues: {
		spawn: queue<{ key: string }>({
			onMessage: async (c, msg) => {
				const client = c.client<typeof registry>();
				await client.manyQueueChildActor.getOrCreate([msg.body.key], {
					createWithInput: msg.body.key,
				});
				c.state.spawned.push(msg.body.key);
			},
		}),
	},
	actions: {
		queueSpawn: async (c, key: string) => {
			await c.queue.send("spawn", { key });
			return { queued: true };
		},
		getSpawned: (c) => c.state.spawned,
	},
});

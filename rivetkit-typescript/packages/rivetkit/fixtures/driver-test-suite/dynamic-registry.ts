import { actor, setup, UserError } from "rivetkit";
import { dynamicActor } from "rivetkit/dynamic";

export const DYNAMIC_SOURCE = `
import { actor } from "rivetkit";

const SLEEP_TIMEOUT = 200;

export default actor({
	state: {
		count: 0,
		wakeCount: 0,
		sleepCount: 0,
		alarmCount: 0,
	},
	onWake: (c) => {
		c.state.wakeCount += 1;
	},
	onSleep: (c) => {
		c.state.sleepCount += 1;
	},
	onRequest: async (_c, request) => {
		return new Response(
			JSON.stringify({
				method: request.method,
				token: request.headers.get("x-dynamic-auth"),
			}),
			{
				headers: {
					"content-type": "application/json",
				},
			},
		);
	},
	onWebSocket: (c, websocket) => {
		websocket.send(
			JSON.stringify({
				type: "welcome",
				wakeCount: c.state.wakeCount,
			}),
		);

		websocket.addEventListener("message", (event) => {
			const data = event.data;
			if (typeof data === "string") {
				try {
					const parsed = JSON.parse(data);
					if (parsed.type === "ping") {
						websocket.send(JSON.stringify({ type: "pong" }));
						return;
					}
					if (parsed.type === "stats") {
						websocket.send(
							JSON.stringify({
								type: "stats",
								count: c.state.count,
								wakeCount: c.state.wakeCount,
								sleepCount: c.state.sleepCount,
								alarmCount: c.state.alarmCount,
							}),
						);
						return;
					}
				} catch {}
				websocket.send(data);
				return;
			}

			websocket.send(data);
		});
	},
	actions: {
		increment: (c, amount = 1) => {
			c.state.count += amount;
			return c.state.count;
		},
		getState: (c) => {
			return {
				count: c.state.count,
				wakeCount: c.state.wakeCount,
				sleepCount: c.state.sleepCount,
				alarmCount: c.state.alarmCount,
			};
		},
			getSourceCodeLength: async (c) => {
				const source = (await c
					.client()
					.sourceCode.getOrCreate(["dynamic-source"])
					.getCode());
				return source.length;
			},
		putText: async (c, key, value) => {
			await c.kv.put(key, value);
			return true;
		},
		getText: async (c, key) => {
			return await c.kv.get(key);
		},
		listText: async (c, prefix) => {
			const values = await c.kv.list(prefix, { keyType: "text" });
			return values.map(([key, value]) => ({ key, value }));
		},
		triggerSleep: (c) => {
			c.sleep();
			return true;
		},
		scheduleAlarm: async (c, duration) => {
			await c.schedule.after(duration, "onAlarm");
			return true;
		},
		onAlarm: (c) => {
			c.state.alarmCount += 1;
			return c.state.alarmCount;
		},
	},
	options: {
		sleepTimeout: SLEEP_TIMEOUT,
	},
});
`;

const sourceCode = actor({
	actions: {
		getCode: () => DYNAMIC_SOURCE,
	},
});

const dynamicFromUrl = dynamicActor({
	load: async () => {
		const sourceUrl = process.env.RIVETKIT_DYNAMIC_TEST_SOURCE_URL;
		if (!sourceUrl) {
			throw new Error(
				"missing RIVETKIT_DYNAMIC_TEST_SOURCE_URL for dynamic actor URL loader",
			);
		}

		const response = await fetch(sourceUrl);
		if (!response.ok) {
			throw new Error(
				`dynamic actor URL loader failed with status ${response.status}`,
			);
		}

		return {
			source: await response.text(),
			sourceFormat: "esm-js" as const,
			nodeProcess: {
				memoryLimit: 256,
				cpuTimeLimitMs: 10_000,
			},
		};
	},
});

const dynamicFromActor = dynamicActor({
	load: async (c) => {
		const source = (await c
			.client<any>()
			.sourceCode.getOrCreate(["dynamic-source"])
			.getCode()) as string;
		return {
			source,
			sourceFormat: "esm-js" as const,
			nodeProcess: {
				memoryLimit: 256,
				cpuTimeLimitMs: 10_000,
			},
		};
	},
});

const dynamicWithAuth = dynamicActor({
	load: async (c) => {
		const source = (await c
			.client<any>()
			.sourceCode.getOrCreate(["dynamic-source"])
			.getCode()) as string;
		return {
			source,
			sourceFormat: "esm-js" as const,
			nodeProcess: {
				memoryLimit: 256,
				cpuTimeLimitMs: 10_000,
			},
		};
	},
	auth: (c, params: unknown) => {
		const authHeader = c.request?.headers.get("x-dynamic-auth");
		const authToken =
			typeof params === "object" &&
			params !== null &&
			"token" in params &&
			typeof (params as { token?: unknown }).token === "string"
				? (params as { token: string }).token
				: undefined;
		if (authHeader === "allow" || authToken === "allow") {
			return;
		}
		throw new UserError("auth required", {
			code: "unauthorized",
			metadata: {
				hasRequest: c.request !== undefined,
			},
		});
	},
});

const dynamicLoaderThrows = dynamicActor({
	load: async () => {
		throw new Error("dynamic.loader_failed_for_test");
	},
});

const dynamicInvalidSource = dynamicActor({
	load: async () => {
		return {
			source: "export default 42;",
			sourceFormat: "esm-js" as const,
		};
	},
});

export const registry = setup({
	use: {
		sourceCode,
		dynamicFromUrl,
		dynamicFromActor,
		dynamicWithAuth,
		dynamicLoaderThrows,
		dynamicInvalidSource,
	},
});

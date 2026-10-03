import {
	type AddressInfo,
	connect,
	createServer,
	type Server,
	type Socket,
} from "node:net";
import { serve } from "@hono/node-server";
import { describe, expect, test } from "vitest";
import { actor, setup } from "@/mod";
import { NapiCoreRuntime } from "@/registry/napi-runtime";
import type { ActorContextHandle } from "@/registry/runtime";
import { createClient } from "../../src/client/mod";
import { getOrStartSharedEngine, TOKEN } from "./shared-harness";

interface HookEvent {
	hook: "createVars" | "onSleep" | "onSleepDone" | "clearRuntimeState";
	boot: string | undefined;
}

// The registry runs in this process, so hooks and runtime cleanup report straight to the test.
const hookEvents: HookEvent[] = [];

// Holds one generation's `onSleep` open so the next generation starts while it is still shutting
// down, which is the overlap seen in production.
let heldSleep: { boot: string; released: Promise<void> } | undefined;

const lostVarsActor = actor({
	state: {},
	createVars: () => {
		const boot = crypto.randomUUID();
		hookEvents.push({ hook: "createVars", boot });
		return { boot };
	},
	onSleep: async (c) => {
		hookEvents.push({ hook: "onSleep", boot: c.vars.boot });
		if (heldSleep?.boot === c.vars.boot) {
			await heldSleep.released;
		}
		hookEvents.push({ hook: "onSleepDone", boot: c.vars.boot });
	},
	actions: {
		readBoot: (c) => c.vars.boot,
		hasVars: (c) => c.vars !== undefined,
	},
	options: {
		sleepGracePeriod: 60_000,
	},
});

/**
 * Records every native runtime-state clear with the boot id of the vars it clears. This is the
 * cleanup step that used to wipe the next generation's state.
 */
function observeRuntimeStateClears(): () => void {
	const original = NapiCoreRuntime.prototype.actorClearRuntimeState;
	NapiCoreRuntime.prototype.actorClearRuntimeState = function (
		this: NapiCoreRuntime,
		ctx: ActorContextHandle,
	) {
		const runtimeState = this.actorRuntimeState(ctx) as {
			vars?: { boot?: string };
		};
		hookEvents.push({
			hook: "clearRuntimeState",
			boot: runtimeState.vars?.boot,
		});
		return original.call(this, ctx);
	};
	return () => {
		NapiCoreRuntime.prototype.actorClearRuntimeState = original;
	};
}

function hasEvent(hook: HookEvent["hook"], boot: string): boolean {
	return hookEvents.some(
		(event) => event.hook === hook && event.boot === boot,
	);
}

/**
 * TCP proxy in front of the serverless runner. The engine holds one long-lived `/start` request
 * per actor generation through it, and `resetEngineConnections` resets those sockets mid-stream
 * the way a load balancer does. The runner's envoy WebSocket dials the engine directly, so it is
 * unaffected.
 */
class StartRequestProxy {
	readonly #server: Server;
	readonly #engineSockets = new Set<Socket>();

	private constructor(server: Server) {
		this.#server = server;
	}

	static async start(upstreamPort: number): Promise<StartRequestProxy> {
		const server = createServer();
		const proxy = new StartRequestProxy(server);
		server.on("connection", (engineSocket) => {
			proxy.#engineSockets.add(engineSocket);
			engineSocket.once("close", () =>
				proxy.#engineSockets.delete(engineSocket),
			);

			const runnerSocket = connect(upstreamPort, "127.0.0.1");
			engineSocket.pipe(runnerSocket);
			runnerSocket.pipe(engineSocket);
			engineSocket.on("error", () => runnerSocket.destroy());
			runnerSocket.on("error", () => engineSocket.destroy());
			engineSocket.once("close", () => runnerSocket.destroy());
			runnerSocket.once("close", () => engineSocket.destroy());
		});
		await new Promise<void>((resolve) =>
			server.listen(0, "127.0.0.1", resolve),
		);
		return proxy;
	}

	get port(): number {
		return (this.#server.address() as AddressInfo).port;
	}

	/** Returns how many in-flight `/start` connections were reset. */
	resetEngineConnections(): number {
		const sockets = [...this.#engineSockets];
		for (const socket of sockets) {
			socket.resetAndDestroy();
		}
		return sockets.length;
	}

	async close(): Promise<void> {
		for (const socket of this.#engineSockets) {
			socket.destroy();
		}
		await new Promise<void>((resolve) =>
			this.#server.close(() => resolve()),
		);
	}
}

async function apiFetch(
	endpoint: string,
	path: string,
	init: RequestInit = {},
): Promise<Response> {
	const response = await fetch(`${endpoint}${path}`, {
		...init,
		headers: {
			Authorization: `Bearer ${TOKEN}`,
			"Content-Type": "application/json",
			...init.headers,
		},
	});
	if (!response.ok) {
		throw new Error(
			`${init.method ?? "GET"} ${path} failed: ${response.status} ${await response.text()}`,
		);
	}
	return response;
}

async function upsertServerlessRunnerConfig(input: {
	endpoint: string;
	namespace: string;
	poolName: string;
	serverlessUrl: string;
}): Promise<void> {
	const datacenters = (await (
		await apiFetch(
			input.endpoint,
			`/datacenters?namespace=${encodeURIComponent(input.namespace)}`,
		)
	).json()) as { datacenters: Array<{ name: string }> };
	const datacenter = datacenters.datacenters[0]?.name;
	if (!datacenter) {
		throw new Error("engine returned no datacenters");
	}

	await apiFetch(
		input.endpoint,
		`/runner-configs/${encodeURIComponent(input.poolName)}?namespace=${encodeURIComponent(input.namespace)}`,
		{
			method: "PUT",
			body: JSON.stringify({
				datacenters: {
					[datacenter]: {
						serverless: {
							url: input.serverlessUrl,
							headers: { "x-rivet-token": TOKEN },
							// Keep the planned lifespan drain out of the test window.
							request_lifespan: 3600,
							drain_grace_period: 5,
							metadata_poll_interval: 1000,
							max_runners: 10,
							min_runners: 0,
							runners_margin: 0,
							slots_per_runner: 10,
						},
					},
				},
			}),
		},
	);
}

async function waitFor<T>(
	description: string,
	timeoutMs: number,
	check: () => Promise<T | undefined>,
): Promise<T> {
	const deadline = Date.now() + timeoutMs;
	let lastError: unknown;
	while (Date.now() < deadline) {
		try {
			const value = await check();
			if (value !== undefined) {
				return value;
			}
		} catch (error) {
			lastError = error;
		}
		await new Promise((resolve) => setTimeout(resolve, 250));
	}
	throw new Error(
		`timed out waiting for ${description}: ${String(lastError)}`,
	);
}

async function withTimeout<T>(
	promise: Promise<T>,
	timeoutMs: number,
): Promise<T> {
	let timer: ReturnType<typeof setTimeout> | undefined;
	try {
		return await Promise.race([
			promise,
			new Promise<never>((_, reject) => {
				timer = setTimeout(
					() => reject(new Error(`timed out after ${timeoutMs}ms`)),
					timeoutMs,
				);
			}),
		]);
	} finally {
		clearTimeout(timer);
	}
}

describe("Serverless Lost generation", () => {
	test("a Lost generation's cleanup does not clear the next generation's state", async () => {
		const engine = await getOrStartSharedEngine();
		const namespace = `serverless-lost-${crypto.randomUUID()}`;
		const poolName = `serverless-lost-${crypto.randomUUID()}`;
		await apiFetch(engine.endpoint, "/namespaces", {
			method: "POST",
			body: JSON.stringify({ name: namespace, display_name: namespace }),
		});

		const registry = setup({
			use: { lostVarsActor },
			noWelcome: true,
			endpoint: engine.endpoint,
			token: TOKEN,
			namespace,
			envoy: { poolName },
			// The test host has no Services binary, and Services failing to start shuts the
			// registry down before `/start` is served.
			startServices: false,
		});
		let runner: ReturnType<typeof serve> | undefined;
		const runnerPort = await new Promise<number>((resolve) => {
			runner = serve(
				{
					fetch: (request) => registry.handler(request),
					hostname: "127.0.0.1",
					port: 0,
					// Hono's global Request override breaks POST bodies under vitest, so `/start`
					// would fail before reaching the registry.
					overrideGlobalObjects: false,
				},
				(info) => resolve(info.port),
			);
		});
		const proxy = await StartRequestProxy.start(runnerPort);
		const stopObservingClears = observeRuntimeStateClears();

		try {
			await upsertServerlessRunnerConfig({
				endpoint: engine.endpoint,
				namespace,
				poolName,
				serverlessUrl: `http://127.0.0.1:${proxy.port}/api/rivet`,
			});

			const client = createClient<typeof registry>({
				endpoint: engine.endpoint,
				namespace,
				token: TOKEN,
				poolName,
			});
			const handle = client.lostVarsActor.getOrCreate([
				`lost-${crypto.randomUUID()}`,
			]);

			// The runner config needs a metadata poll before the engine dispatches starts.
			const firstBoot = await waitFor("first generation", 30_000, () =>
				handle.readBoot(),
			);

			let releaseOldSleep = () => {};
			heldSleep = {
				boot: firstBoot,
				released: new Promise<void>((resolve) => {
					releaseOldSleep = resolve;
				}),
			};

			// Reset the engine's `/start` stream for the running generation. The engine treats
			// that as the envoy being lost, sends `StopActor { reason: Lost }` to the still
			// connected envoy, and the old generation starts its graceful sleep.
			expect(proxy.resetEngineConnections()).toBeGreaterThan(0);
			await waitFor("old generation onSleep", 30_000, async () =>
				hasEvent("onSleep", firstBoot) ? true : undefined,
			);

			// Waking the actor starts the next generation on the same envoy while the old one is
			// still inside `onSleep`.
			const secondBoot = await waitFor(
				"second generation",
				30_000,
				async () => {
					const boot = await handle.readBoot();
					return boot === firstBoot ? undefined : boot;
				},
			);
			expect(hasEvent("onSleepDone", firstBoot)).toBe(false);

			// Let the old generation finish and wait for the runtime-state clear its cleanup runs.
			const eventsBeforeRelease = hookEvents.length;
			releaseOldSleep();
			const oldGenerationClear = await waitFor(
				"old generation runtime cleanup",
				30_000,
				async () =>
					hookEvents
						.slice(eventsBeforeRelease)
						.find((event) => event.hook === "clearRuntimeState"),
			);

			// The old generation's cleanup must clear its own state, not the new generation's.
			expect(oldGenerationClear.boot).toBe(firstBoot);
			await expect(withTimeout(handle.hasVars(), 10_000)).resolves.toBe(
				true,
			);
			await expect(withTimeout(handle.readBoot(), 10_000)).resolves.toBe(
				secondBoot,
			);
		} finally {
			stopObservingClears();
			heldSleep = undefined;
			await proxy.close();
			await new Promise<void>((resolve) => {
				if (runner) {
					runner.close(() => resolve());
				} else {
					resolve();
				}
			});
		}
	}, 90_000);
});

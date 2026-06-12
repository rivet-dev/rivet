import { randomUUID } from "node:crypto";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import getPort from "get-port";
import { describe, expect, test } from "vitest";
import {
	buildPlatformSqliteCounterRegistrySource,
	createPlatformServerlessRunner,
	createPlatformSqliteCounterClient,
	createTempPlatformApp,
	getOrStartPlatformTestEngine,
	type LoggedChild,
	linkWorkspacePackage,
	PLATFORM_TEST_TOKEN,
	releasePlatformTestEngine,
	spawnPinnedPnpmDlx,
	type TempPlatformApp,
	waitForHttpOk,
} from "./shared-platform-harness";

const WRANGLER_VERSION = "4.87.0";
const TEST_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(TEST_DIR, "../../../../..");

function writeCloudflareWorkerApp(
	app: TempPlatformApp,
	{
		endpoint,
		namespace,
		runnerName,
		token,
	}: {
		endpoint: string;
		namespace: string;
		runnerName: string;
		token: string;
	},
) {
	linkWorkspacePackage(
		app,
		"rivetkit",
		resolve(REPO_ROOT, "rivetkit-typescript/packages/rivetkit"),
	);
	linkWorkspacePackage(
		app,
		"@rivetkit/rivetkit-wasm",
		resolve(REPO_ROOT, "rivetkit-typescript/packages/rivetkit-wasm"),
	);

	app.writeFile(
		"package.json",
		JSON.stringify(
			{
				type: "module",
				dependencies: {
					"@rivetkit/rivetkit-wasm": "workspace:*",
					rivetkit: "workspace:*",
				},
			},
			null,
			2,
		),
	);
	app.writeFile(
		"wrangler.toml",
		`
name = "rivetkit-cloudflare-platform-smoke"
main = "src/index.ts"
compatibility_date = "2025-04-01"
compatibility_flags = ["nodejs_compat"]

[vars]
RIVET_ENDPOINT = "${endpoint}"
RIVET_NAMESPACE = "${namespace}"
RIVET_POOL = "${runnerName}"
RIVET_TOKEN = "${token}"
`,
	);
	app.writeFile(
		"src/registry.ts",
		buildPlatformSqliteCounterRegistrySource("cloudflare-module-import"),
	);
	app.writeFile(
		"src/index.ts",
		`
import { createRegistry } from "./registry";

interface Env {
	RIVET_ENDPOINT: string;
	RIVET_NAMESPACE: string;
	RIVET_POOL: string;
	RIVET_TOKEN: string;
}

type WebSocketProtocolInput = string | string[] | undefined;

class FetchWebSocket {
	static readonly CONNECTING = 0;
	static readonly OPEN = 1;
	static readonly CLOSING = 2;
	static readonly CLOSED = 3;

	binaryType: BinaryType = "arraybuffer";
	onopen: ((event: Event) => void) | null = null;
	onmessage: ((event: MessageEvent) => void) | null = null;
	onclose: ((event: CloseEvent) => void) | null = null;
	onerror: ((event: Event) => void) | null = null;
	readyState = FetchWebSocket.CONNECTING;
	#socket: WebSocket | undefined;
	#pending: Array<string | ArrayBuffer | ArrayBufferView> = [];

	constructor(url: string, protocols?: WebSocketProtocolInput) {
		void this.#connect(url, protocols);
	}

	async #connect(url: string, protocols?: WebSocketProtocolInput) {
		try {
			const protocolList = Array.isArray(protocols)
				? protocols
				: protocols
					? [protocols]
					: [];
			const headers = new Headers({ Upgrade: "websocket" });
			if (protocolList.length > 0) {
				headers.set("Sec-WebSocket-Protocol", protocolList.join(", "));
			}
			const response = await fetch(
				url.replace(/^ws:/, "http:").replace(/^wss:/, "https:"),
				{ headers },
			);
			const socket = response.webSocket;
			if (!socket) {
				throw new Error(
					\`websocket upgrade failed with status \${response.status}\`,
				);
			}

			socket.accept();
			socket.binaryType = this.binaryType;
			this.#socket = socket;
			this.readyState = FetchWebSocket.OPEN;
			socket.addEventListener("message", (event) => {
				this.onmessage?.(event);
			});
			socket.addEventListener("close", (event) => {
				this.readyState = FetchWebSocket.CLOSED;
				this.onclose?.(event);
			});
			socket.addEventListener("error", (event) => {
				this.onerror?.(event);
			});
			this.onopen?.(new Event("open"));
			for (const data of this.#pending.splice(0)) {
				socket.send(data);
			}
		} catch (error) {
			console.error("rivetkit cloudflare websocket shim failed", error);
			this.readyState = FetchWebSocket.CLOSED;
			this.onerror?.(error instanceof Event ? error : new Event("error"));
			this.onclose?.(new CloseEvent("close", { code: 1006 }));
		}
	}

	send(data: string | ArrayBuffer | ArrayBufferView) {
		if (this.readyState === FetchWebSocket.CONNECTING) {
			this.#pending.push(data);
			return;
		}
		this.#socket?.send(data);
	}

	close(code?: number, reason?: string) {
		this.readyState = FetchWebSocket.CLOSING;
		this.#socket?.close(code, reason);
	}
}

(globalThis as unknown as { WebSocket: typeof WebSocket }).WebSocket =
	FetchWebSocket as unknown as typeof WebSocket;

let registry: ReturnType<typeof createRegistry> | undefined;

function getRegistry(env: Env) {
	registry ??= createRegistry({
		endpoint: env.RIVET_ENDPOINT,
		namespace: env.RIVET_NAMESPACE,
		token: env.RIVET_TOKEN,
		runnerName: env.RIVET_POOL,
	});

	return registry;
}

export default {
	async fetch(request: Request, env: Env): Promise<Response> {
		if (new URL(request.url).pathname === "/health") {
			return new Response("ok");
		}

		return await getRegistry(env).handler(request);
	},
};
`,
	);
}

async function waitForRunnerMetadata(url: string) {
	const deadline = Date.now() + 15_000;
	let bodyText = "";

	while (Date.now() < deadline) {
		const response = await fetch(url);
		bodyText = await response.text();
		if (response.ok) {
			const body = JSON.parse(bodyText) as {
				envoy?: { version?: number } | null;
				envoyProtocolVersion?: number | null;
			};
			if (body.envoy?.version && body.envoyProtocolVersion != null) {
				return;
			}
		}
		await new Promise((resolveWait) => setTimeout(resolveWait, 250));
	}

	throw new Error(
		`serverless metadata did not expose envoy metadata: ${bodyText}`,
	);
}

async function waitForRunnerConfigReady({
	endpoint,
	namespace,
	runnerName,
	token,
}: {
	endpoint: string;
	namespace: string;
	runnerName: string;
	token: string;
}) {
	const deadline = Date.now() + 15_000;
	let bodyText = "";

	while (Date.now() < deadline) {
		const response = await fetch(
			`${endpoint}/runner-configs?namespace=${encodeURIComponent(namespace)}&runner_name=${encodeURIComponent(runnerName)}`,
			{
				headers: {
					Authorization: `Bearer ${token}`,
				},
			},
		);
		bodyText = await response.text();
		if (response.ok) {
			const body = JSON.parse(bodyText) as {
				runner_configs?: Record<
					string,
					{
						datacenters?: Record<
							string,
							{
								protocol_version?: number | null;
								serverless?: unknown;
							}
						>;
					}
				>;
			};
			const config = body.runner_configs?.[runnerName];
			const datacenters = Object.values(config?.datacenters ?? {});
			if (
				datacenters.length > 0 &&
				datacenters.every(
					(datacenter) => datacenter.protocol_version != null,
				)
			) {
				return;
			}
		}
		await new Promise((resolveWait) => setTimeout(resolveWait, 250));
	}

	throw new Error(`serverless runner config was not ready: ${bodyText}`);
}

async function waitForWorkerStartRequest(worker: LoggedChild) {
	const deadline = Date.now() + 75_000;

	while (Date.now() < deadline) {
		if (
			worker.getOutput().includes("GET /start") ||
			worker.getOutput().includes("GET /api/rivet/start")
		) {
			return;
		}
		await new Promise((resolveWait) => setTimeout(resolveWait, 250));
	}

	throw new Error(
		`timed out waiting for Cloudflare Worker start request:\n${worker.getOutput()}`,
	);
}

function isColdStartCapacityError(error: unknown): boolean {
	const message = error instanceof Error ? error.message : String(error);
	const code =
		error && typeof error === "object" && "code" in error
			? String((error as { code: unknown }).code)
			: "";
	return (
		code === "service_unavailable" ||
		code === "actor_wake_retries_exceeded" ||
		message.includes("actor_ready_timeout") ||
		message.includes("actor_wake_retries_exceeded") ||
		message.includes("no_capacity") ||
		message.includes("request_timeout") ||
		message.includes("service_unavailable")
	);
}

async function runAfterColdStart<T>(
	worker: LoggedChild,
	run: () => Promise<T>,
): Promise<T> {
	const firstRequest = run().then(
		(value) => ({ ok: true as const, value }),
		(error: unknown) => ({ ok: false as const, error }),
	);
	await Promise.race([
		firstRequest,
		waitForWorkerStartRequest(worker).then(() => undefined),
	]);
	const firstResult = await firstRequest;
	if (firstResult.ok) {
		return firstResult.value;
	}
	if (!isColdStartCapacityError(firstResult.error)) {
		throw firstResult.error;
	}
	return await run();
}

async function delay(ms: number) {
	await new Promise((resolve) => setTimeout(resolve, ms));
}

describe("Cloudflare Workers wasm platform smoke", () => {
	test("runs the shared SQLite counter registry through local workerd", async () => {
		const engine = await getOrStartPlatformTestEngine();
		let app: TempPlatformApp | undefined;
		let worker: LoggedChild | undefined;

		try {
			const port = await getPort();
			const workerOrigin = `http://127.0.0.1:${port}`;
			const serverlessUrl = `${workerOrigin}/api/rivet`;
			const namespace = `cf-${randomUUID()}`;
			const runnerName = `cf-${randomUUID()}`;

			app = createTempPlatformApp({}, "rivetkit-cloudflare-");
			writeCloudflareWorkerApp(app, {
				endpoint: engine.endpoint.replace("127.0.0.1", "localhost"),
				namespace,
				runnerName,
				token: PLATFORM_TEST_TOKEN,
			});
			worker = spawnPinnedPnpmDlx({
				label: "wrangler",
				packageName: "wrangler",
				packageVersion: WRANGLER_VERSION,
				args: [
					"dev",
					"--local",
					"--ip",
					"127.0.0.1",
					"--port",
					String(port),
					"--inspector-port",
					"0",
				],
				options: {
					cwd: app.path,
					env: {
						...process.env,
						NO_COLOR: "1",
					},
				},
			});
			await waitForHttpOk({
				url: `${workerOrigin}/health`,
				child: worker.child,
				getOutput: worker.getOutput,
				timeoutMs: 60_000,
			});
			const runner = await createPlatformServerlessRunner({
				engine,
				namespace,
				runnerName,
				serverlessUrl,
				minRunners: 1,
				runnersMargin: 1,
			});
			await waitForRunnerMetadata(`${serverlessUrl}/metadata`);
			await waitForRunnerConfigReady({
				endpoint: engine.endpoint,
				namespace,
				runnerName,
				token: PLATFORM_TEST_TOKEN,
			});
			const actorKey = `counter-${randomUUID()}`;

			const client = createPlatformSqliteCounterClient(runner);
			const actor = client.sqliteCounter.getOrCreate([actorKey]);

			expect(
				await runAfterColdStart(worker, () => actor.increment(2)),
			).toBe(2);
			expect(await actor.increment(3)).toBe(5);
			expect(await actor.getCount()).toBe(5);

			const beforeSleep = await actor.getLifecycleCounts();
			expect(beforeSleep.wakeCount).toBeGreaterThanOrEqual(1);
			await actor.triggerSleep();
			await delay(500);

			expect(
				await runAfterColdStart(worker, () => actor.getCount()),
			).toBe(5);
			const afterWake = await actor.getLifecycleCounts();
			expect(afterWake.wakeCount).toBeGreaterThanOrEqual(
				beforeSleep.wakeCount + 1,
			);

			const parallelActors = [1, 2, 3].map((amount) =>
				client.sqliteCounter.getOrCreate([
					`parallel-${amount}-${randomUUID()}`,
				]),
			);
			await expect(
				Promise.all(
					parallelActors.map((parallelActor, index) =>
						parallelActor.increment(index + 1),
					),
				),
			).resolves.toEqual([1, 2, 3]);
			await expect(
				Promise.all(
					parallelActors.map((parallelActor) =>
						parallelActor.getCount(),
					),
				),
			).resolves.toEqual([1, 2, 3]);
		} finally {
			await worker?.stop();
			app?.cleanup();
			await releasePlatformTestEngine();
		}
	}, 120_000);
});

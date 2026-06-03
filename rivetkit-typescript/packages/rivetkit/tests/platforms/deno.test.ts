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
	spawnLoggedChild,
	type TempPlatformApp,
	waitForHttpOk,
} from "./shared-platform-harness";

const TEST_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(TEST_DIR, "../../../../..");

function writeDenoApp(
	app: TempPlatformApp,
	{
		endpoint,
		namespace,
		port,
		runnerName,
		token,
	}: {
		endpoint: string;
		namespace: string;
		port: number;
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
			},
			null,
			2,
		),
	);
	app.writeFile(
		"src/registry.ts",
		buildPlatformSqliteCounterRegistrySource("deno-read-file"),
	);
	app.writeFile(
		"src/index.ts",
		`
import { createRegistry } from "./registry.ts";

const registry = createRegistry({
	endpoint: "${endpoint}",
	namespace: "${namespace}",
	token: "${token}",
	runnerName: "${runnerName}",
});

Deno.serve(
	{
		hostname: "127.0.0.1",
		port: ${port},
		onListen: () => {
			console.log("deno platform app listening");
		},
	},
	async (request) => {
		const pathname = new URL(request.url).pathname;
		console.log(\`\${request.method} \${pathname}\`);
		if (pathname === "/health") {
			return new Response("ok");
		}

		return await registry.handler(request);
	},
);
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

async function waitForDenoStartRequest(deno: LoggedChild) {
	const deadline = Date.now() + 75_000;

	while (Date.now() < deadline) {
		if (
			deno.getOutput().includes("GET /start") ||
			deno.getOutput().includes("GET /api/rivet/start")
		) {
			return;
		}
		await new Promise((resolveWait) => setTimeout(resolveWait, 250));
	}

	throw new Error(
		`timed out waiting for Deno start request:\n${deno.getOutput()}`,
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
	deno: LoggedChild,
	run: () => Promise<T>,
): Promise<T> {
	const firstRequest = run().then(
		(value) => ({ ok: true as const, value }),
		(error: unknown) => ({ ok: false as const, error }),
	);
	await Promise.race([
		firstRequest,
		waitForDenoStartRequest(deno).then(() => undefined),
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

describe("Deno wasm platform smoke", () => {
	test("runs the shared SQLite counter registry through local Deno", async () => {
		const engine = await getOrStartPlatformTestEngine();
		let app: TempPlatformApp | undefined;
		let deno: LoggedChild | undefined;

		try {
			const port = await getPort();
			const denoOrigin = `http://127.0.0.1:${port}`;
			const serverlessUrl = `${denoOrigin}/api/rivet`;
			const namespace = `deno-${randomUUID()}`;
			const runnerName = `deno-${randomUUID()}`;

			app = createTempPlatformApp({}, "rivetkit-deno-");
			writeDenoApp(app, {
				endpoint: engine.endpoint,
				namespace,
				port,
				runnerName,
				token: PLATFORM_TEST_TOKEN,
			});
			deno = spawnLoggedChild({
				label: "deno",
				command: "deno",
				args: [
					"run",
					"--allow-env",
					"--allow-net",
					"--allow-read",
					"--allow-sys",
					"--node-modules-dir=manual",
					"--no-lock",
					"src/index.ts",
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
				url: `${denoOrigin}/health`,
				child: deno.child,
				getOutput: deno.getOutput,
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
				await runAfterColdStart(deno, () => actor.increment(2)),
			).toBe(2);
			expect(await actor.increment(3)).toBe(5);
			expect(await actor.getCount()).toBe(5);

			const beforeSleep = await actor.getLifecycleCounts();
			expect(beforeSleep.wakeCount).toBeGreaterThanOrEqual(1);
			await actor.triggerSleep();
			await delay(500);

			expect(await runAfterColdStart(deno, () => actor.getCount())).toBe(
				5,
			);
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
			await deno?.stop();
			app?.cleanup();
			await releasePlatformTestEngine();
		}
	}, 120_000);
});

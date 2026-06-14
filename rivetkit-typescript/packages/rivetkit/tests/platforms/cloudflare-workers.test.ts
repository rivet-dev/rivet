import { randomUUID } from "node:crypto";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import getPort from "get-port";
import { describe, expect, test } from "vitest";
import {
	buildPlatformSqliteCounterActorSource,
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
	linkWorkspacePackage(
		app,
		"@rivetkit/cloudflare-workers",
		resolve(REPO_ROOT, "rivetkit-typescript/packages/cloudflare-workers"),
	);

	app.writeFile(
		"package.json",
		JSON.stringify(
			{
				type: "module",
				dependencies: {
					"@rivetkit/cloudflare-workers": "workspace:*",
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
	app.writeFile("src/actor.ts", buildPlatformSqliteCounterActorSource());
	app.writeFile(
		"src/index.ts",
		`
import { createHandler, setup } from "@rivetkit/cloudflare-workers";
import { sqliteCounter } from "./actor";

const registry = setup({ use: { sqliteCounter }, sqlite: "remote" });

export default createHandler(registry, {
	fetch: (request) => {
		if (new URL(request.url).pathname === "/health") {
			return new Response("ok");
		}
		return new Response("not found", { status: 404 });
	},
});
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

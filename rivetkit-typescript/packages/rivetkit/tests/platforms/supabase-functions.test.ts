import { randomUUID } from "node:crypto";
import {
	cpSync,
	existsSync,
	mkdirSync,
	mkdtempSync,
	readFileSync,
	realpathSync,
	rmSync,
	writeFileSync,
} from "node:fs";
import { networkInterfaces, tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { getEnginePath } from "@rivetkit/engine-cli";
import getPort from "get-port";
import { describe, expect, test } from "vitest";
import {
	buildPlatformSqliteCounterRegistrySource,
	createPlatformServerlessRunner,
	createPlatformSqliteCounterClient,
	createTempPlatformApp,
	type LoggedChild,
	linkWorkspacePackage,
	PLATFORM_TEST_TOKEN,
	spawnLoggedChild,
	spawnPinnedPnpmDlx,
	type TempPlatformApp,
	waitForHttpOk,
} from "./shared-platform-harness";

const SUPABASE_VERSION = "2.95.4";
const TEST_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(TEST_DIR, "../../../../..");
const REPO_ENGINE_BINARY = resolve(REPO_ROOT, "target/debug/rivet-engine");
const RIVETKIT_PACKAGE_DIR = resolve(
	REPO_ROOT,
	"rivetkit-typescript/packages/rivetkit",
);

interface SupabaseTestEngine {
	endpoint: string;
	publicEndpoint: string;
	pid: number;
	dbRoot: string;
	process: LoggedChild;
	stop(): Promise<void>;
}

function resolveEngineBinaryPath(): string {
	if (existsSync(REPO_ENGINE_BINARY)) {
		return REPO_ENGINE_BINARY;
	}

	return getEnginePath();
}

function resolveDockerHost(): string {
	const docker0Address = networkInterfaces().docker0?.find(
		(address) => address.family === "IPv4",
	)?.address;

	return docker0Address ?? "host.docker.internal";
}

async function startSupabaseTestEngine(): Promise<SupabaseTestEngine> {
	const host = "127.0.0.1";
	const guardPort = await getPort({ host });
	const apiPeerPort = await getPort({
		host,
		exclude: [guardPort],
	});
	const metricsPort = await getPort({
		host,
		exclude: [guardPort, apiPeerPort],
	});
	const endpoint = `http://${host}:${guardPort}`;
	const publicEndpoint = `http://${resolveDockerHost()}:${guardPort}`;
	const dbRoot = mkdtempSync(join(tmpdir(), "rivetkit-supabase-engine-"));
	const configPath = join(dbRoot, "config.json");
	writeFileSync(
		configPath,
		JSON.stringify({
			topology: {
				datacenter_label: 1,
				datacenters: {
					default: {
						datacenter_label: 1,
						is_leader: true,
						public_url: publicEndpoint,
						peer_url: `http://${host}:${apiPeerPort}`,
					},
				},
			},
		}),
	);

	const engineProcess = spawnLoggedChild({
		label: "supabase-engine",
		command: resolveEngineBinaryPath(),
		args: ["start", "--config", configPath],
		options: {
			env: {
				...process.env,
				RIVET__GUARD__HOST: "0.0.0.0",
				RIVET__GUARD__PORT: guardPort.toString(),
				RIVET__API_PEER__HOST: host,
				RIVET__API_PEER__PORT: apiPeerPort.toString(),
				RIVET__METRICS__HOST: host,
				RIVET__METRICS__PORT: metricsPort.toString(),
				RIVET__FILE_SYSTEM__PATH: join(dbRoot, "db"),
			},
		},
	});
	await waitForHttpOk({
		url: `${endpoint}/health`,
		child: engineProcess.child,
		getOutput: engineProcess.getOutput,
		timeoutMs: 90_000,
	});

	if (engineProcess.child.pid === undefined) {
		await engineProcess.stop();
		rmSync(dbRoot, { force: true, recursive: true });
		throw new Error("Supabase test engine started without a pid");
	}

	return {
		endpoint,
		publicEndpoint,
		pid: engineProcess.child.pid,
		dbRoot,
		process: engineProcess,
		stop: async () => {
			await engineProcess.stop();
			rmSync(dbRoot, { force: true, recursive: true });
		},
	};
}

function packagePathParts(packageName: string): string[] {
	return packageName.split("/");
}

function resolvePackageSource(
	packageName: string,
	fromDir = resolve(RIVETKIT_PACKAGE_DIR, "node_modules"),
): string {
	if (packageName === "rivetkit") {
		return RIVETKIT_PACKAGE_DIR;
	}

	const packagePath = resolve(fromDir, ...packagePathParts(packageName));
	if (existsSync(packagePath)) {
		return realpathSync(packagePath);
	}

	const parentPath = resolve(fromDir, "..", ...packagePathParts(packageName));
	if (existsSync(parentPath)) {
		return realpathSync(parentPath);
	}

	throw new Error(`unable to resolve package ${packageName} from ${fromDir}`);
}

function copyPackageTree(
	destinationNodeModules: string,
	packageName: string,
	seen: Set<string>,
	fromDir?: string,
	includeDependencies = true,
) {
	if (seen.has(packageName)) return;
	seen.add(packageName);

	const source = resolvePackageSource(packageName, fromDir);
	const destination = resolve(
		destinationNodeModules,
		...packagePathParts(packageName),
	);
	mkdirSync(dirname(destination), { recursive: true });
	rmSync(destination, { force: true, recursive: true });
	cpSync(source, destination, {
		dereference: true,
		filter: (path) =>
			!path.startsWith(resolve(source, "node_modules")) &&
			!path.includes("/.git/") &&
			!path.endsWith(".map"),
		recursive: true,
	});

	const packageJson = JSON.parse(
		readFileSync(resolve(source, "package.json"), "utf8"),
	) as { dependencies?: Record<string, string> };
	if (!includeDependencies) return;

	for (const dependency of Object.keys(packageJson.dependencies ?? {})) {
		copyPackageTree(destinationNodeModules, dependency, seen, source);
	}
}

function copySupabaseFunctionPackages(app: TempPlatformApp) {
	const destinationNodeModules = resolve(
		app.path,
		"supabase/functions/rivet/node_modules",
	);
	mkdirSync(destinationNodeModules, { recursive: true });

	const seen = new Set<string>();
	for (const packageName of [
		"@rivetkit/rivetkit-wasm",
		"@rivetkit/virtual-websocket",
		"@rivetkit/bare-ts",
		"cbor-x",
		"hono",
		"invariant",
		"p-retry",
		"pino",
		"rivetkit",
		"vbare",
		"zod",
	]) {
		copyPackageTree(
			destinationNodeModules,
			packageName,
			seen,
			undefined,
			packageName !== "rivetkit",
		);
	}
}

function writeSupabaseFunctionApp(
	app: TempPlatformApp,
	{
		apiPort,
		dbPort,
		endpoint,
		publicEndpoint,
		namespace,
		runnerName,
		token,
	}: {
		apiPort: number;
		dbPort: number;
		endpoint: string;
		publicEndpoint: string;
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
		"supabase/config.toml",
		`
project_id = "rivetkit-platform-${randomUUID()}"

[api]
port = ${apiPort}
schemas = ["public"]
extra_search_path = ["public", "extensions"]
max_rows = 1000

[db]
port = ${dbPort}
shadow_port = ${dbPort + 1}
major_version = 15

[studio]
port = ${dbPort + 2}

[inbucket]
port = ${dbPort + 3}

[edge_runtime]
policy = "per_worker"
`,
	);
	app.writeFile(
		"supabase/functions/rivet/registry.ts",
		buildPlatformSqliteCounterRegistrySource("deno-read-file"),
	);
	app.writeFile(
		"supabase/functions/rivet/index.ts",
		`
import { createRegistry } from "./registry.ts";

const SERVERLESS_BASE_PATH = "/rivet/api/rivet";

const registry = createRegistry({
	endpoint: "${endpoint}",
	namespace: "${namespace}",
	token: "${token}",
	runnerName: "${runnerName}",
	serverless: {
		basePath: SERVERLESS_BASE_PATH,
		publicEndpoint: "${publicEndpoint}",
	},
});

Deno.serve(async (request) => {
	const pathname = new URL(request.url).pathname;
	console.log(\`\${request.method} \${pathname}\`);
	if (pathname.endsWith("/health")) {
		return new Response("ok");
	}

	return await registry.handler(request);
});
`,
	);
	app.writeFile(
		"supabase/functions/rivet/package.json",
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
	copySupabaseFunctionPackages(app);
}

function waitForChildExit(
	child: LoggedChild,
	timeoutMs: number,
): Promise<void> {
	return new Promise((resolveWait, rejectWait) => {
		const timeout = setTimeout(() => {
			void child.stop("SIGKILL", 1_000).finally(() => {
				rejectWait(
					new Error(
						`platform command timed out:\n${child.getOutput()}`,
					),
				);
			});
		}, timeoutMs);

		child.child.once("exit", (code, signal) => {
			clearTimeout(timeout);
			if (code === 0) {
				resolveWait();
				return;
			}

			rejectWait(
				new Error(
					`platform command failed with code ${code ?? signal}:\n${child.getOutput()}`,
				),
			);
		});
	});
}

async function runSupabaseCli(
	label: string,
	app: TempPlatformApp,
	args: string[],
	timeoutMs: number,
) {
	const command = spawnPinnedPnpmDlx({
		label,
		packageName: "supabase",
		packageVersion: SUPABASE_VERSION,
		args,
		options: {
			cwd: app.path,
			env: {
				...process.env,
				NO_COLOR: "1",
			},
		},
	});
	await waitForChildExit(command, timeoutMs);
}

async function waitForRunnerMetadata(url: string, platform: LoggedChild) {
	const deadline = Date.now() + 30_000;
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
		`serverless metadata did not expose envoy metadata: ${bodyText}\n${platform.getOutput()}`,
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
	const deadline = Date.now() + 30_000;
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

async function waitForSupabaseStartRequest(supabase: LoggedChild) {
	const deadline = Date.now() + 75_000;

	while (Date.now() < deadline) {
		if (
			supabase.getOutput().includes("GET /start") ||
			supabase.getOutput().includes("GET /rivet/api/rivet/start") ||
			supabase.getOutput().includes("POST /rivet/api/rivet/start") ||
			supabase
				.getOutput()
				.includes("GET /functions/v1/rivet/api/rivet/start") ||
			supabase
				.getOutput()
				.includes("POST /functions/v1/rivet/api/rivet/start")
		) {
			return;
		}
		await new Promise((resolveWait) => setTimeout(resolveWait, 250));
	}

	throw new Error(
		`timed out waiting for Supabase Functions start request:\n${supabase.getOutput()}`,
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
	supabase: LoggedChild,
	run: () => Promise<T>,
): Promise<T> {
	const deadline = Date.now() + 30_000;
	const firstRequest = run().then(
		(value) => ({ ok: true as const, value }),
		(error: unknown) => ({ ok: false as const, error }),
	);
	await Promise.race([
		firstRequest,
		waitForSupabaseStartRequest(supabase).then(() => undefined),
	]);
	const firstResult = await firstRequest;
	if (firstResult.ok) {
		return firstResult.value;
	}
	if (!isColdStartCapacityError(firstResult.error)) {
		throw firstResult.error;
	}

	let lastError = firstResult.error;
	while (Date.now() < deadline) {
		await delay(500);
		try {
			return await run();
		} catch (error) {
			if (!isColdStartCapacityError(error)) {
				throw error;
			}
			lastError = error;
		}
	}

	throw lastError;
}

async function delay(ms: number) {
	await new Promise((resolve) => setTimeout(resolve, ms));
}

describe("Supabase Functions wasm platform smoke", () => {
	test("runs the shared SQLite counter registry through local Supabase Functions", async () => {
		const engine = await startSupabaseTestEngine();
		let app: TempPlatformApp | undefined;
		let supabase: LoggedChild | undefined;

		try {
			const apiPort = await getPort();
			const dbPort = await getPort();
			const supabaseOrigin = `http://127.0.0.1:${apiPort}`;
			const serverlessBasePath = "/functions/v1/rivet/api/rivet";
			const serverlessUrl = `${supabaseOrigin}${serverlessBasePath}`;
			const namespace = `supabase-${randomUUID()}`;
			const runnerName = `supabase-${randomUUID()}`;

			app = createTempPlatformApp({}, "rivetkit-supabase-");
			writeSupabaseFunctionApp(app, {
				apiPort,
				dbPort,
				endpoint: engine.publicEndpoint,
				publicEndpoint: engine.endpoint,
				namespace,
				runnerName,
				token: PLATFORM_TEST_TOKEN,
			});
			await runSupabaseCli(
				"supabase-start",
				app,
				[
					"start",
					"-x",
					[
						"gotrue",
						"realtime",
						"storage-api",
						"imgproxy",
						"mailpit",
						"postgrest",
						"postgres-meta",
						"studio",
						"edge-runtime",
						"logflare",
						"vector",
						"supavisor",
					].join(","),
					"--ignore-health-check",
				],
				180_000,
			);
			supabase = spawnPinnedPnpmDlx({
				label: "supabase-functions",
				packageName: "supabase",
				packageVersion: SUPABASE_VERSION,
				args: ["functions", "serve", "--no-verify-jwt"],
				options: {
					cwd: app.path,
					env: {
						...process.env,
						NO_COLOR: "1",
					},
				},
			});
			await waitForHttpOk({
				url: `${supabaseOrigin}/functions/v1/rivet/health`,
				child: supabase.child,
				getOutput: supabase.getOutput,
				timeoutMs: 90_000,
			});
			const runner = await createPlatformServerlessRunner({
				engine,
				namespace,
				runnerName,
				serverlessUrl,
			});
			await waitForRunnerMetadata(`${serverlessUrl}/metadata`, supabase);
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
				await runAfterColdStart(supabase, () => actor.increment(2)),
			).toBe(2);
			expect(await actor.increment(3)).toBe(5);
			expect(await actor.getCount()).toBe(5);

			const beforeSleep = await actor.getLifecycleCounts();
			expect(beforeSleep.wakeCount).toBeGreaterThanOrEqual(1);
			await delay(1_500);

			expect(
				await runAfterColdStart(supabase, () => actor.getCount()),
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
			await supabase?.stop();
			if (app) {
				try {
					await runSupabaseCli(
						"supabase-stop",
						app,
						["stop", "--no-backup"],
						60_000,
					);
				} catch {}
			}
			app?.cleanup();
			await engine.stop();
		}
	}, 240_000);
});

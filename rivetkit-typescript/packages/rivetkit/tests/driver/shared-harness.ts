import { type ChildProcess, spawn } from "node:child_process";
import { createHash } from "node:crypto";
import {
	existsSync,
	mkdirSync,
	mkdtempSync,
	readFileSync,
	rmSync,
	statSync,
	unlinkSync,
	writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { getEnginePath } from "@rivetkit/engine-cli";
import getPort from "get-port";
import type { DriverRegistryVariant } from "../driver-registry-variants";
import type { DriverDeployOutput, DriverTestConfig } from "./shared-types";

const DRIVER_TEST_DIR = dirname(fileURLToPath(import.meta.url));
const TEST_DIR = join(DRIVER_TEST_DIR, "..");
const FIXTURE_PATH = join(TEST_DIR, "fixtures", "driver-test-suite-runtime.ts");
const REPO_ENGINE_BINARY = join(
	TEST_DIR,
	"../../../../target/debug/rivet-engine",
);
export const TOKEN = "dev";
const TIMING_ENABLED = process.env.RIVETKIT_DRIVER_TEST_TIMING === "1";
const ENGINE_STATE_ID = createHash("sha256")
	.update(TEST_DIR)
	.digest("hex")
	.slice(0, 16);
const ENGINE_START_LOCK_DIR = join(
	tmpdir(),
	`rivetkit-driver-engine-${ENGINE_STATE_ID}.lock`,
);
const ENGINE_STATE_PATH = join(
	tmpdir(),
	`rivetkit-driver-engine-${ENGINE_STATE_ID}.json`,
);
const ENGINE_START_LOCK_STALE_MS = 120_000;

interface RuntimeLogs {
	stdout: string;
	stderr: string;
}

export interface SharedEngine {
	endpoint: string;
	pid: number;
	dbRoot: string;
}

export interface NativeDriverTestConfigOptions {
	variant: DriverRegistryVariant;
	encoding: NonNullable<DriverTestConfig["encoding"]>;
	useRealTimers?: boolean;
	skip?: DriverTestConfig["skip"];
	features?: DriverTestConfig["features"];
}

interface SharedEngineState extends SharedEngine {
	refs: number;
}

let sharedEnginePromise: Promise<SharedEngine> | undefined;
let sharedEngineRefAcquired = false;

function childOutput(logs: RuntimeLogs): string {
	return [logs.stdout, logs.stderr].filter(Boolean).join("\n");
}

function timing(
	label: string,
	startedAt: number,
	fields: Record<string, string> = {},
) {
	if (!TIMING_ENABLED) {
		return;
	}

	const fieldText = Object.entries(fields)
		.map(([key, value]) => `${key}=${value}`)
		.join(" ");
	console.log(
		`DRIVER_TIMING ${label} ms=${Math.round(performance.now() - startedAt)}${fieldText ? ` ${fieldText}` : ""}`,
	);
}

function resolveEngineBinaryPath(): string {
	if (existsSync(REPO_ENGINE_BINARY)) {
		return REPO_ENGINE_BINARY;
	}

	return getEnginePath();
}

async function acquireEngineStartLock(): Promise<() => void> {
	const startedAt = performance.now();

	while (true) {
		try {
			mkdirSync(ENGINE_START_LOCK_DIR);
			timing("engine.start_lock", startedAt);
			return () => {
				rmSync(ENGINE_START_LOCK_DIR, { force: true, recursive: true });
			};
		} catch (error) {
			const code = (error as NodeJS.ErrnoException).code;
			if (code !== "EEXIST") {
				throw error;
			}

			try {
				const stat = statSync(ENGINE_START_LOCK_DIR);
				if (Date.now() - stat.mtimeMs > ENGINE_START_LOCK_STALE_MS) {
					rmSync(ENGINE_START_LOCK_DIR, { force: true, recursive: true });
					continue;
				}
			} catch {}

			await new Promise((resolve) => setTimeout(resolve, 50));
		}
	}
}

async function waitForEngineHealth(
	child: ChildProcess,
	logs: RuntimeLogs,
	endpoint: string,
	timeoutMs: number,
): Promise<void> {
	const deadline = Date.now() + timeoutMs;

	while (Date.now() < deadline) {
		if (child.exitCode !== null) {
			throw new Error(
				`shared engine exited before health check passed:\n${childOutput(logs)}`,
			);
		}

		try {
			const response = await fetch(`${endpoint}/health`);
			if (response.ok) {
				return;
			}
		} catch {}

		await new Promise((resolve) => setTimeout(resolve, 500));
	}

	throw new Error(
		`timed out waiting for shared engine health:\n${childOutput(logs)}`,
	);
}

async function waitForEnvoy(
	child: ChildProcess,
	logs: RuntimeLogs,
	endpoint: string,
	namespace: string,
	poolName: string,
	timeoutMs: number,
): Promise<void> {
	const deadline = Date.now() + timeoutMs;

	while (Date.now() < deadline) {
		if (child.exitCode !== null) {
			throw new Error(
				`native runtime exited before envoy registration:\n${childOutput(logs)}`,
			);
		}

		const response = await fetch(
			`${endpoint}/envoys?namespace=${encodeURIComponent(namespace)}&name=${encodeURIComponent(poolName)}`,
			{
				headers: {
					Authorization: `Bearer ${TOKEN}`,
				},
			},
		);

		if (response.ok) {
			const body = (await response.json()) as {
				envoys: Array<{ envoy_key: string }>;
			};

			if (body.envoys.length > 0) {
				return;
			}
		}

		await new Promise((resolve) => setTimeout(resolve, 500));
	}

	throw new Error(
		`timed out waiting for envoy registration in pool ${poolName}\n${childOutput(logs)}`,
	);
}

async function upsertNormalRunnerConfig(
	logs: RuntimeLogs,
	endpoint: string,
	namespace: string,
	poolName: string,
): Promise<void> {
	const datacentersStartedAt = performance.now();
	const datacentersResponse = await fetch(
		`${endpoint}/datacenters?namespace=${encodeURIComponent(namespace)}`,
		{
			headers: {
				Authorization: `Bearer ${TOKEN}`,
			},
		},
	);

	if (!datacentersResponse.ok) {
		throw new Error(
			`failed to list datacenters: ${datacentersResponse.status} ${await datacentersResponse.text()}\n${childOutput(logs)}`,
		);
	}

	const datacentersBody = (await datacentersResponse.json()) as {
		datacenters: Array<{ name: string }>;
	};
	const datacenter = datacentersBody.datacenters[0]?.name;

	if (!datacenter) {
		throw new Error(`engine returned no datacenters\n${childOutput(logs)}`);
	}
	timing("runner_config.datacenters", datacentersStartedAt, { namespace });

	const deadline = Date.now() + 30_000;

	while (Date.now() < deadline) {
		const upsertStartedAt = performance.now();
		const response = await fetch(
			`${endpoint}/runner-configs/${encodeURIComponent(poolName)}?namespace=${encodeURIComponent(namespace)}`,
			{
				method: "PUT",
				headers: {
					Authorization: `Bearer ${TOKEN}`,
					"Content-Type": "application/json",
				},
				body: JSON.stringify({
					datacenters: {
						[datacenter]: {
							normal: {},
						},
					},
				}),
			},
		);

		if (response.ok) {
			timing("runner_config.upsert", upsertStartedAt, {
				namespace,
				poolName,
			});
			return;
		}

		const responseBody = await response.text();
		if (
			(response.status === 400 &&
				responseBody.includes('"group":"namespace"') &&
				responseBody.includes('"code":"not_found"')) ||
			(response.status === 500 &&
				responseBody.includes('"group":"core"') &&
				responseBody.includes('"code":"internal_error"'))
		) {
			await new Promise((resolve) => setTimeout(resolve, 500));
			continue;
		}

		throw new Error(
			`failed to upsert runner config ${poolName}: ${response.status} ${responseBody}\n${childOutput(logs)}`,
		);
	}

	throw new Error(
		`timed out waiting to upsert runner config ${poolName}\n${childOutput(logs)}`,
	);
}

async function createNamespace(endpoint: string, namespace: string): Promise<void> {
	const startedAt = performance.now();
	const response = await fetch(`${endpoint}/namespaces`, {
		method: "POST",
		headers: {
			Authorization: `Bearer ${TOKEN}`,
			"Content-Type": "application/json",
		},
		body: JSON.stringify({
			name: namespace,
			display_name: `Driver test ${namespace}`,
		}),
	});

	if (!response.ok) {
		throw new Error(
			`failed to create namespace ${namespace}: ${response.status} ${await response.text()}`,
		);
	}
	timing("namespace.create", startedAt, { namespace });
}

function readSharedEngineState(): SharedEngineState | undefined {
	try {
		return JSON.parse(readFileSync(ENGINE_STATE_PATH, "utf8"));
	} catch {
		return undefined;
	}
}

function writeSharedEngineState(state: SharedEngineState): void {
	writeFileSync(ENGINE_STATE_PATH, JSON.stringify(state), "utf8");
}

function removeSharedEngineState(): void {
	try {
		unlinkSync(ENGINE_STATE_PATH);
	} catch {}
}

function isPidRunning(pid: number): boolean {
	try {
		process.kill(pid, 0);
		return true;
	} catch {
		return false;
	}
}

async function isEngineHealthy(endpoint: string): Promise<boolean> {
	try {
		const response = await fetch(`${endpoint}/health`);
		return response.ok;
	} catch {
		return false;
	}
}

async function stopProcess(
	child: ChildProcess,
	signal: NodeJS.Signals,
	timeoutMs: number,
): Promise<void> {
	if (child.exitCode !== null) {
		return;
	}

	child.kill(signal);

	await new Promise<void>((resolve) => {
		const timeout = setTimeout(() => {
			if (child.exitCode === null) {
				child.kill("SIGKILL");
			}
		}, timeoutMs);

		child.once("exit", () => {
			clearTimeout(timeout);
			resolve();
		});
	});
}

async function stopPid(pid: number, timeoutMs: number): Promise<void> {
	if (!isPidRunning(pid)) {
		return;
	}

	process.kill(pid, "SIGTERM");

	const deadline = Date.now() + timeoutMs;
	while (Date.now() < deadline) {
		if (!isPidRunning(pid)) {
			return;
		}
		await new Promise((resolve) => setTimeout(resolve, 100));
	}

	if (isPidRunning(pid)) {
		process.kill(pid, "SIGKILL");
	}
}

async function spawnSharedEngine(): Promise<SharedEngine> {
	const startedAt = performance.now();
	const portStartedAt = performance.now();
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
	const dbRoot = mkdtempSync(join(tmpdir(), "rivetkit-driver-engine-"));
	timing("engine.allocate", portStartedAt, { endpoint });

	const spawnStartedAt = performance.now();
	const logs: RuntimeLogs = { stdout: "", stderr: "" };
	const engine = spawn(resolveEngineBinaryPath(), ["start"], {
		env: {
			...process.env,
			RIVET__GUARD__HOST: host,
			RIVET__GUARD__PORT: guardPort.toString(),
			RIVET__API_PEER__HOST: host,
			RIVET__API_PEER__PORT: apiPeerPort.toString(),
			RIVET__METRICS__HOST: host,
			RIVET__METRICS__PORT: metricsPort.toString(),
			RIVET__FILE_SYSTEM__PATH: join(dbRoot, "db"),
		},
		stdio: ["ignore", "pipe", "pipe"],
	});
	timing("engine.spawn", spawnStartedAt, { endpoint });

	engine.stdout?.on("data", (chunk) => {
		const text = chunk.toString();
		logs.stdout += text;
		if (process.env.DRIVER_ENGINE_LOGS === "1") {
			process.stderr.write(`[ENG.OUT] ${text}`);
		}
	});
	engine.stderr?.on("data", (chunk) => {
		const text = chunk.toString();
		logs.stderr += text;
		if (process.env.DRIVER_ENGINE_LOGS === "1") {
			process.stderr.write(`[ENG.ERR] ${text}`);
		}
	});

	try {
		const healthStartedAt = performance.now();
		await waitForEngineHealth(engine, logs, endpoint, 90_000);
		timing("engine.health", healthStartedAt, { endpoint });
	} catch (error) {
		await stopRuntime(engine);
		rmSync(dbRoot, { force: true, recursive: true });
		throw error;
	}

	if (engine.pid === undefined) {
		await stopRuntime(engine);
		rmSync(dbRoot, { force: true, recursive: true });
		throw new Error("shared engine started without a pid");
	}

	const sharedEngine = {
		endpoint,
		pid: engine.pid,
		dbRoot,
	};
	timing("engine.start_total", startedAt, { endpoint });
	return sharedEngine;
}

export async function getOrStartSharedEngine(): Promise<SharedEngine> {
	if (sharedEnginePromise) {
		return sharedEnginePromise;
	}

	sharedEnginePromise = (async () => {
		const releaseStartLock = await acquireEngineStartLock();
		try {
			const existing = readSharedEngineState();
			if (
				existing &&
				isPidRunning(existing.pid) &&
				(await isEngineHealthy(existing.endpoint))
			) {
				const state = { ...existing, refs: existing.refs + 1 };
				writeSharedEngineState(state);
				sharedEngineRefAcquired = true;
				timing("engine.reuse", performance.now(), {
					endpoint: existing.endpoint,
				});
				return {
					endpoint: existing.endpoint,
					pid: existing.pid,
					dbRoot: existing.dbRoot,
				};
			}

			if (existing) {
				await stopPid(existing.pid, 5_000);
				rmSync(existing.dbRoot, { force: true, recursive: true });
				removeSharedEngineState();
			}

			const engine = await spawnSharedEngine();
			writeSharedEngineState({ ...engine, refs: 1 });
			sharedEngineRefAcquired = true;
			return engine;
		} catch (error) {
			sharedEnginePromise = undefined;
			throw error;
		} finally {
			releaseStartLock();
		}
	})();

	return sharedEnginePromise;
}

export async function releaseSharedEngine(): Promise<void> {
	if (!sharedEngineRefAcquired) {
		return;
	}
	sharedEngineRefAcquired = false;
	sharedEnginePromise = undefined;

	const releaseStartLock = await acquireEngineStartLock();
	const startedAt = performance.now();
	try {
		const state = readSharedEngineState();
		if (!state) {
			return;
		}

		const refs = Math.max(0, state.refs - 1);
		if (refs > 0) {
			writeSharedEngineState({ ...state, refs });
			return;
		}

		await stopPid(state.pid, 5_000);
		rmSync(state.dbRoot, { force: true, recursive: true });
		removeSharedEngineState();
		timing("engine.stop", startedAt, { endpoint: state.endpoint });
	} finally {
		releaseStartLock();
	}
}

async function stopRuntime(child: ChildProcess): Promise<void> {
	const startedAt = performance.now();
	await stopProcess(child, "SIGTERM", 1_000);
	timing("runtime.stop", startedAt);
}

export async function startNativeDriverRuntime(
	variant: DriverRegistryVariant,
	engine: SharedEngine,
): Promise<DriverDeployOutput> {
	const startedAt = performance.now();
	const endpoint = engine.endpoint;
	const namespace = `driver-${crypto.randomUUID()}`;
	const poolName = `driver-suite-${crypto.randomUUID()}`;
	const logs: RuntimeLogs = { stdout: "", stderr: "" };

	await createNamespace(endpoint, namespace);
	await upsertNormalRunnerConfig(logs, endpoint, namespace, poolName);

	const spawnStartedAt = performance.now();
	const runtime = spawn(process.execPath, ["--import", "tsx", FIXTURE_PATH], {
		cwd: dirname(TEST_DIR),
		env: {
			...process.env,
			RIVET_TOKEN: TOKEN,
			RIVET_NAMESPACE: namespace,
			RIVETKIT_DRIVER_REGISTRY_PATH: variant.registryPath,
			RIVETKIT_TEST_ENDPOINT: endpoint,
			RIVETKIT_TEST_POOL_NAME: poolName,
		},
		stdio: ["ignore", "pipe", "pipe"],
	});
	timing("runtime.spawn", spawnStartedAt, { namespace, poolName });

	runtime.stdout?.on("data", (chunk) => {
		const text = chunk.toString();
		logs.stdout += text;
		if (process.env.DRIVER_RUNTIME_LOGS === "1") {
			process.stderr.write(`[RT.OUT] ${text}`);
		}
	});
	runtime.stderr?.on("data", (chunk) => {
		const text = chunk.toString();
		logs.stderr += text;
		if (process.env.DRIVER_RUNTIME_LOGS === "1") {
			process.stderr.write(`[RT.ERR] ${text}`);
		}
	});

	try {
		const envoyStartedAt = performance.now();
		await waitForEnvoy(runtime, logs, endpoint, namespace, poolName, 30_000);
		timing("runtime.envoy", envoyStartedAt, { namespace, poolName });
	} catch (error) {
		await stopRuntime(runtime);
		throw error;
	}
	timing("runtime.start_total", startedAt, { namespace, poolName });

	return {
		endpoint,
		namespace,
		runnerName: poolName,
		cleanup: async () => {
			await stopRuntime(runtime);
		},
	};
}

export function createNativeDriverTestConfig(
	options: NativeDriverTestConfigOptions,
): DriverTestConfig {
	return {
		encoding: options.encoding,
		skip: options.skip,
		features: {
			hibernatableWebSocketProtocol: true,
			...options.features,
		},
		useRealTimers: options.useRealTimers ?? true,
		start: async () => {
			const engine = await getOrStartSharedEngine();
			return startNativeDriverRuntime(options.variant, engine);
		},
	};
}

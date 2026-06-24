import { type ChildProcess, execFile, spawn } from "node:child_process";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import getPort from "get-port";
import { afterEach, describe, expect, test } from "vitest";
import { createClient } from "../src/client/mod";

const TEST_DIR = dirname(fileURLToPath(import.meta.url));
const FIXTURE_PATH = join(
	TEST_DIR,
	"fixtures",
	"native-plugin-runtime-server.ts",
);
const R6_ROOT = resolve(TEST_DIR, "../../../..");
const RUST_WORKSPACE = join(R6_ROOT, "rivetkit-rust");
const TEST_PLUGIN_PATH = join(
	R6_ROOT,
	"target",
	"debug",
	process.platform === "darwin"
		? "librivet_actor_test_plugin.dylib"
		: process.platform === "win32"
			? "rivet_actor_test_plugin.dll"
			: "librivet_actor_test_plugin.so",
);
const NAMESPACE = "default";
const TOKEN = "dev";
const execFileAsync = promisify(execFile);
let runtimeLogs = {
	stdout: "",
	stderr: "",
};

function childOutput(): string {
	return [runtimeLogs.stdout, runtimeLogs.stderr].filter(Boolean).join("\n");
}

async function waitForHealth(
	child: ChildProcess,
	endpoint: string,
	timeoutMs: number,
): Promise<void> {
	const deadline = Date.now() + timeoutMs;

	while (Date.now() < deadline) {
		if (child.exitCode !== null) {
			throw new Error(
				`native plugin runtime exited before health check passed:\n${childOutput()}`,
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
		`timed out waiting for native plugin runtime health:\n${childOutput()}`,
	);
}

async function waitForEnvoy(
	child: ChildProcess,
	endpoint: string,
	poolName: string,
	timeoutMs: number,
): Promise<void> {
	const deadline = Date.now() + timeoutMs;

	while (Date.now() < deadline) {
		if (child.exitCode !== null) {
			throw new Error(
				`native plugin runtime exited before envoy registration:\n${childOutput()}`,
			);
		}

		const response = await fetch(
			`${endpoint}/envoys?namespace=${encodeURIComponent(NAMESPACE)}&name=${encodeURIComponent(poolName)}`,
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
		`timed out waiting for envoy registration in pool ${poolName}\n${childOutput()}`,
	);
}

async function upsertNormalRunnerConfig(
	endpoint: string,
	poolName: string,
): Promise<void> {
	const datacentersResponse = await fetch(
		`${endpoint}/datacenters?namespace=${encodeURIComponent(NAMESPACE)}`,
		{
			headers: {
				Authorization: `Bearer ${TOKEN}`,
			},
		},
	);

	if (!datacentersResponse.ok) {
		throw new Error(
			`failed to list datacenters: ${datacentersResponse.status} ${await datacentersResponse.text()}\n${childOutput()}`,
		);
	}

	const datacentersBody = (await datacentersResponse.json()) as {
		datacenters: Array<{ name: string }>;
	};
	const datacenter = datacentersBody.datacenters[0]?.name;

	if (!datacenter) {
		throw new Error(`engine returned no datacenters\n${childOutput()}`);
	}

	const response = await fetch(
		`${endpoint}/runner-configs/${encodeURIComponent(poolName)}?namespace=${encodeURIComponent(NAMESPACE)}`,
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
		return;
	}

	throw new Error(
		`failed to upsert runner config ${poolName}: ${response.status} ${await response.text()}\n${childOutput()}`,
	);
}

async function waitForActorReady<T>(
	callback: () => Promise<T>,
	timeoutMs: number,
): Promise<T> {
	const deadline = Date.now() + timeoutMs;
	let lastError: unknown;

	while (Date.now() < deadline) {
		try {
			return await callback();
		} catch (error) {
			lastError = error;
			const errorCode =
				typeof error === "object" &&
				error !== null &&
				"code" in error &&
				typeof error.code === "string"
					? error.code
					: undefined;
			if (
				!(
					(errorCode &&
						/^(no_envoys|actor_ready_timeout|actor_wake_retries_exceeded|service_unavailable)$/.test(
							errorCode,
						)) ||
					(error instanceof Error &&
						/(no_envoys|actor_ready_timeout|actor_wake_retries_exceeded|service_unavailable)/.test(
							error.message,
						))
				)
			) {
				throw error;
			}
		}

		await new Promise((resolve) => setTimeout(resolve, 500));
	}

	throw lastError instanceof Error
		? lastError
		: new Error("timed out waiting for actor to become ready");
}

async function stopRuntime(child: ChildProcess): Promise<void> {
	if (child.exitCode !== null) {
		return;
	}

	child.kill("SIGINT");

	await new Promise<void>((resolve) => {
		const timeout = setTimeout(() => {
			if (child.exitCode === null) {
				child.kill("SIGKILL");
			}
		}, 5_000);

		child.once("exit", () => {
			clearTimeout(timeout);
			resolve();
		});
	});
}

async function buildNativePluginFixture(): Promise<string> {
	await execFileAsync("cargo", ["build", "-p", "rivet-actor-test-plugin"], {
		cwd: RUST_WORKSPACE,
		env: process.env,
		maxBuffer: 1024 * 1024 * 20,
	});
	return TEST_PLUGIN_PATH;
}

describe.sequential("native plugin runtime integration", () => {
	let runtime: ChildProcess | undefined;

	afterEach(async () => {
		if (runtime) {
			await stopRuntime(runtime);
			runtime = undefined;
		}
	}, 30_000);

	test("registers a native plugin actor through the TS runtime", async () => {
		const pluginPath = await buildNativePluginFixture();
		const poolName = "native-plugin";
		const port = await getPort({ host: "127.0.0.1" });
		const endpoint = `http://127.0.0.1:${port}`;
		const configJson = JSON.stringify({
			package: "@rivetkit/native-plugin-test-shape",
			sidecar: true,
		});
		const sidecarPath = "/tmp/rivetkit-native-plugin-sidecar";
		runtimeLogs = { stdout: "", stderr: "" };
		runtime = spawn(process.execPath, ["--import", "tsx", FIXTURE_PATH], {
			cwd: dirname(TEST_DIR),
			env: {
				...process.env,
				RIVET_TOKEN: TOKEN,
				RIVET_NAMESPACE: NAMESPACE,
				RIVETKIT_TEST_ENDPOINT: endpoint,
				RIVETKIT_TEST_POOL_NAME: poolName,
				RIVETKIT_TEST_NATIVE_PLUGIN_PATH: pluginPath,
				RIVETKIT_TEST_NATIVE_PLUGIN_CONFIG_JSON: configJson,
				RIVETKIT_TEST_NATIVE_PLUGIN_SIDECAR_PATH: sidecarPath,
				RIVETKIT_STORAGE_PATH: mkdtempSync(
					join(tmpdir(), "rivetkit-native-plugin-test-"),
				),
			},
			stdio: ["ignore", "pipe", "pipe"],
		});
		runtime.stdout?.on("data", (chunk) => {
			runtimeLogs.stdout += chunk.toString();
		});
		runtime.stderr?.on("data", (chunk) => {
			runtimeLogs.stderr += chunk.toString();
		});

		await waitForHealth(runtime, endpoint, 90_000);
		await upsertNormalRunnerConfig(endpoint, poolName);
		await waitForEnvoy(runtime, endpoint, poolName, 30_000);

		const client = createClient<any>({
			endpoint,
			token: TOKEN,
			namespace: NAMESPACE,
			poolName,
			disableMetadataLookup: true,
		}) as any;

		const handle = await waitForActorReady(
			() =>
				client.nativePluginActor.create([
					`native-plugin-${crypto.randomUUID()}`,
				]),
			30_000,
		);

		expect(
			await waitForActorReady(
				() => handle.factory_config_report(),
				30_000,
			),
		).toEqual({
			configJson,
			sidecarPath,
		});
		expect(await waitForActorReady(() => handle.increment(), 30_000)).toBe(
			1,
		);
		expect(await waitForActorReady(() => handle.get(), 30_000)).toBe(1);

		await client.dispose();
	}, 120_000);
});

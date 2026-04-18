import { type ChildProcess, spawn } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import getPort from "get-port";
import { describe } from "vitest";
import { runDriverTests, type DriverDeployOutput } from "../src/driver-test-suite/mod";
import {
	getDriverRegistryVariants,
	type DriverRegistryVariant,
} from "./driver-registry-variants";

const TEST_DIR = dirname(fileURLToPath(import.meta.url));
const FIXTURE_PATH = join(TEST_DIR, "fixtures", "driver-test-suite-runtime.ts");
const NAMESPACE = "default";
const TOKEN = "dev";

interface RuntimeLogs {
	stdout: string;
	stderr: string;
}

function childOutput(logs: RuntimeLogs): string {
	return [logs.stdout, logs.stderr].filter(Boolean).join("\n");
}

async function waitForHealth(
	child: ChildProcess,
	logs: RuntimeLogs,
	endpoint: string,
	timeoutMs: number,
): Promise<void> {
	const deadline = Date.now() + timeoutMs;

	while (Date.now() < deadline) {
		if (child.exitCode !== null) {
			throw new Error(
				`native runtime exited before health check passed:\n${childOutput(logs)}`,
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
		`timed out waiting for native runtime health:\n${childOutput(logs)}`,
	);
}

async function waitForEnvoy(
	child: ChildProcess,
	logs: RuntimeLogs,
	endpoint: string,
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
		`timed out waiting for envoy registration in pool ${poolName}\n${childOutput(logs)}`,
	);
}

async function upsertNormalRunnerConfig(
	child: ChildProcess,
	logs: RuntimeLogs,
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

	const deadline = Date.now() + 30_000;

	while (Date.now() < deadline) {
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

async function startNativeDriverRuntime(
	variant: DriverRegistryVariant,
): Promise<DriverDeployOutput> {
	const port = await getPort({ host: "127.0.0.1" });
	const endpoint = `http://127.0.0.1:${port}`;
	const poolName = `driver-suite-${crypto.randomUUID()}`;
	const logs: RuntimeLogs = { stdout: "", stderr: "" };
	const runtime = spawn(process.execPath, ["--import", "tsx", FIXTURE_PATH], {
		cwd: dirname(TEST_DIR),
		env: {
			...process.env,
			RIVET_TOKEN: TOKEN,
			RIVET_NAMESPACE: NAMESPACE,
			RIVETKIT_DRIVER_REGISTRY_PATH: variant.registryPath,
			RIVETKIT_TEST_ENDPOINT: endpoint,
			RIVETKIT_TEST_POOL_NAME: poolName,
		},
		stdio: ["ignore", "pipe", "pipe"],
	});

	runtime.stdout?.on("data", (chunk) => {
		logs.stdout += chunk.toString();
	});
	runtime.stderr?.on("data", (chunk) => {
		logs.stderr += chunk.toString();
	});

	try {
		await waitForHealth(runtime, logs, endpoint, 90_000);
		await upsertNormalRunnerConfig(runtime, logs, endpoint, poolName);
		await waitForEnvoy(runtime, logs, endpoint, poolName, 30_000);
	} catch (error) {
		await stopRuntime(runtime);
		throw error;
	}

	return {
		endpoint,
		namespace: NAMESPACE,
		runnerName: poolName,
		cleanup: async () => {
			await stopRuntime(runtime);
		},
	};
}

describe.sequential("driver test suite", () => {
	for (const variant of getDriverRegistryVariants(TEST_DIR)) {
		if (variant.skip) {
			describe.skip(`${variant.name} registry`, () => {});
			continue;
		}

		describe.sequential(`${variant.name} registry`, () => {
			runDriverTests({
				useRealTimers: true,
				skip: {
					inline: true,
				},
				start: async () => startNativeDriverRuntime(variant),
			});
		});
	}
});

import { type ChildProcess, spawn } from "node:child_process";
import { mkdtemp, readdir, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { context, propagation, trace } from "@opentelemetry/api";
import { NodeTracerProvider } from "@opentelemetry/sdk-trace-node";
import getPort from "get-port";
import { afterEach, describe, expect, test, vi } from "vitest";
import { createClient } from "../src/client/mod";
import {
	type OtlpCollector,
	startOtlpCollector,
} from "./fixtures/otlp-collector";

const TEST_DIR = dirname(fileURLToPath(import.meta.url));

// Register a context manager for caller spans and baggage; no exporter is needed.
new NodeTracerProvider().register();
const testTracer = trace.getTracer("napi-runtime-integration");

/** Runs `run` with `rayId` in OpenTelemetry baggage under `rivet.ray.id`. */
function withRayBaggage<T>(rayId: string, run: () => Promise<T>): Promise<T> {
	const baggage = propagation.createBaggage({
		"rivet.ray.id": { value: rayId },
	});
	return context.with(propagation.setBaggage(context.active(), baggage), run);
}

const FIXTURE_PATH = join(TEST_DIR, "fixtures", "napi-runtime-server.ts");
const NAMESPACE = "default";
const TOKEN = "dev";
const SERVICES_POOL_NAME = "services";
let runtimeLogs = {
	stdout: "",
	stderr: "",
};
let engineEndpoint: string | undefined;
let storagePath: string | undefined;

function createIntegrationClient(endpoint: string, poolName: string) {
	return createClient<any>({
		endpoint,
		poolName,
		token: TOKEN,
		namespace: NAMESPACE,
		disableMetadataLookup: true,
	}) as any;
}

function runtimeOutput(): string {
	return [runtimeLogs.stdout, runtimeLogs.stderr].filter(Boolean).join("\n");
}

function childOutput(child: ChildProcess): string {
	void child;
	return runtimeOutput();
}

async function engineOutput(): Promise<string> {
	if (!storagePath) return "";
	const logsPath = join(
		storagePath,
		".rivetkit",
		"var",
		"logs",
		"rivet-engine",
	);
	try {
		const files = await readdir(logsPath);
		return (
			await Promise.all(
				files.map(
					async (file) =>
						`${file}:\n${await readFile(join(logsPath, file), "utf8")}`,
				),
			)
		).join("\n");
	} catch {
		return "";
	}
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
				`native runtime exited before health check passed:\n${childOutput(child)}\n${await engineOutput()}`,
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
		`timed out waiting for native runtime health:\n${childOutput(child)}\n${await engineOutput()}`,
	);
}

async function waitForActorSleep(
	endpoint: string,
	actorId: string,
	timeoutMs: number,
): Promise<void> {
	const deadline = Date.now() + timeoutMs;

	while (Date.now() < deadline) {
		const response = await fetch(
			`${endpoint}/actors?actor_ids=${encodeURIComponent(actorId)}&namespace=${encodeURIComponent(NAMESPACE)}`,
			{
				headers: {
					Authorization: `Bearer ${TOKEN}`,
				},
			},
		);
		expect(response.ok).toBe(true);

		const body = (await response.json()) as {
			actors: Array<{ sleep_ts?: number | null }>;
		};
		const actor = body.actors[0];
		if (actor?.sleep_ts) {
			return;
		}

		await new Promise((resolve) => setTimeout(resolve, 500));
	}

	throw new Error(`timed out waiting for actor ${actorId} to sleep`);
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
				`native runtime exited before envoy registration:\n${childOutput(child)}`,
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
		`timed out waiting for envoy registration in pool ${poolName}\n${childOutput(child)}`,
	);
}

function servicesPid(): number {
	const match = runtimeLogs.stdout.match(
		/Services process is ready[^\n]*\bpid=(\d+)/,
	);
	if (!match) {
		throw new Error("Services readiness log did not include a pid");
	}
	return Number(match[1]);
}

async function waitForProcessExit(
	pid: number,
	timeoutMs: number,
): Promise<void> {
	const deadline = Date.now() + timeoutMs;
	let lastState: string | undefined;
	while (Date.now() < deadline) {
		try {
			process.kill(pid, 0);
		} catch {
			return;
		}
		try {
			const stat = await readFile(`/proc/${pid}/stat`, "utf8");
			lastState = stat.slice(stat.lastIndexOf(") ") + 2).charAt(0);
			if (lastState === "Z") return;
		} catch {
			// `/proc` is Linux-specific; process.kill remains the portable check.
		}
		await new Promise((resolve) => setTimeout(resolve, 100));
	}
	throw new Error(
		`timed out waiting for process ${pid} to stop (state ${lastState ?? "unknown"})`,
	);
}

async function expectNormalRunnerConfig(
	endpoint: string,
	poolName: string,
): Promise<void> {
	const response = await fetch(
		`${endpoint}/runner-configs?namespace=${encodeURIComponent(NAMESPACE)}&runner_name=${encodeURIComponent(poolName)}`,
		{
			headers: { Authorization: `Bearer ${TOKEN}` },
		},
	);
	expect(response.ok).toBe(true);
	const body = (await response.json()) as {
		runner_configs: Record<
			string,
			{ datacenters: Record<string, { normal?: unknown }> }
		>;
	};
	const runnerConfig = body.runner_configs[poolName];
	expect(runnerConfig).toBeDefined();
	expect(
		Object.values(runnerConfig?.datacenters ?? {}).some(
			(datacenter) => datacenter.normal !== undefined,
		),
	).toBe(true);
}

async function createServicesActor(endpoint: string): Promise<string> {
	const response = await fetch(
		`${endpoint}/actors?namespace=${encodeURIComponent(NAMESPACE)}`,
		{
			method: "POST",
			headers: {
				Authorization: `Bearer ${TOKEN}`,
				"Content-Type": "application/json",
			},
			body: JSON.stringify({
				name: "services",
				key: `integration-${crypto.randomUUID()}`,
				runner_name_selector: SERVICES_POOL_NAME,
				crash_policy: "destroy",
			}),
		},
	);
	if (!response.ok) {
		throw new Error(
			`failed to create Services actor: ${response.status} ${await response.text()}`,
		);
	}
	const body = (await response.json()) as { actor: { actor_id: string } };
	return body.actor.actor_id;
}

async function waitForActorStarted(
	endpoint: string,
	actorId: string,
	timeoutMs: number,
): Promise<void> {
	const deadline = Date.now() + timeoutMs;
	while (Date.now() < deadline) {
		const response = await fetch(
			`${endpoint}/actors?actor_ids=${encodeURIComponent(actorId)}&namespace=${encodeURIComponent(NAMESPACE)}`,
			{
				headers: { Authorization: `Bearer ${TOKEN}` },
			},
		);
		if (response.ok) {
			const body = (await response.json()) as {
				actors: Array<{ start_ts?: number | null; error?: unknown }>;
			};
			const actor = body.actors[0];
			if (actor?.error) {
				throw new Error(
					`Services actor failed: ${JSON.stringify(actor.error)}`,
				);
			}
			if (actor?.start_ts) return;
		}
		await new Promise((resolve) => setTimeout(resolve, 250));
	}
	throw new Error(`timed out waiting for Services actor ${actorId}`);
}

async function upsertNormalRunnerConfig(
	child: ChildProcess,
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
			`failed to list datacenters: ${datacentersResponse.status} ${await datacentersResponse.text()}\n${childOutput(child)}`,
		);
	}

	const datacentersBody = (await datacentersResponse.json()) as {
		datacenters: Array<{ name: string }>;
	};
	const datacenter = datacentersBody.datacenters[0]?.name;

	if (!datacenter) {
		throw new Error(
			`engine returned no datacenters\n${childOutput(child)}`,
		);
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
		`failed to upsert runner config ${poolName}: ${response.status} ${await response.text()}\n${childOutput(child)}`,
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
		}, 10_000);

		child.once("exit", () => {
			clearTimeout(timeout);
			resolve();
		});
	});
}

async function stopTestEngine(): Promise<void> {
	if (!storagePath || !engineEndpoint) return;
	const stampPath = join(
		storagePath,
		".rivetkit",
		"var",
		"engine",
		"runtime.json",
	);
	try {
		const stamp = JSON.parse(await readFile(stampPath, "utf8")) as {
			pid: number;
			endpoint: string;
		};
		if (new URL(stamp.endpoint).href !== new URL(engineEndpoint).href) {
			throw new Error(
				`refusing to stop Engine for unexpected endpoint ${stamp.endpoint}`,
			);
		}
		process.kill(stamp.pid, "SIGTERM");
		const deadline = Date.now() + 5_000;
		while (Date.now() < deadline) {
			try {
				process.kill(stamp.pid, 0);
			} catch {
				return;
			}
			await new Promise((resolve) => setTimeout(resolve, 100));
		}
		process.kill(stamp.pid, "SIGKILL");
	} catch (error) {
		if (
			!(
				error instanceof Error &&
				"code" in error &&
				error.code === "ENOENT"
			)
		) {
			throw error;
		}
	}
}

/** OTLP `SpanKind.CLIENT`. */
const OTLP_SPAN_KIND_CLIENT = 3;

interface ExportedSpan {
	name: string;
	traceId: string;
	spanId: string;
	parentSpanId?: string;
	traceState?: string;
	kind?: number;
	endTimeUnixNano: bigint;
	attributes: Record<string, string | undefined>;
	links: Array<{ traceId: string; spanId: string }>;
}

/** Flattens OTLP/JSON export bodies into the spans they carry. */
function exportedSpans(exports: Buffer[]): ExportedSpan[] {
	type OtlpAttribute = {
		key: string;
		value: { stringValue?: string; intValue?: string | number };
	};
	type OtlpSpan = Omit<ExportedSpan, "attributes" | "endTimeUnixNano"> & {
		attributes?: OtlpAttribute[];
		endTimeUnixNano?: string | number;
		kind?: number;
		links?: Array<{ traceId: string; spanId: string }>;
	};
	type OtlpPayload = {
		resourceSpans?: Array<{ scopeSpans?: Array<{ spans?: OtlpSpan[] }> }>;
	};
	return exports.flatMap((body) => {
		const payload = JSON.parse(body.toString("utf8")) as OtlpPayload;
		return (payload.resourceSpans ?? []).flatMap((resource) =>
			(resource.scopeSpans ?? []).flatMap((scope) =>
				(scope.spans ?? []).map((span) => ({
					name: span.name,
					traceId: span.traceId,
					spanId: span.spanId,
					parentSpanId: span.parentSpanId || undefined,
					traceState: span.traceState,
					kind: span.kind,
					endTimeUnixNano: BigInt(span.endTimeUnixNano ?? 0),
					attributes: Object.fromEntries(
						(span.attributes ?? []).map((attribute) => [
							attribute.key,
							attribute.value.stringValue ??
								(attribute.value.intValue === undefined
									? undefined
									: String(attribute.value.intValue)),
						]),
					),
					links: (span.links ?? []).map((link) => ({
						traceId: link.traceId,
						spanId: link.spanId,
					})),
				})),
			),
		);
	});
}

/**
 * Polls until the exported spans satisfy `ready`, then returns them. Parent
 * and child spans can land in different export batches, so callers that
 * assert parentage must wait for both.
 */
async function waitForSpans(
	exports: Buffer[],
	description: string,
	ready: (spans: ExportedSpan[]) => boolean,
	timeoutMs: number,
): Promise<ExportedSpan[]> {
	const deadline = Date.now() + timeoutMs;
	while (Date.now() < deadline) {
		const spans = exportedSpans(exports);
		if (ready(spans)) {
			return spans;
		}
		await new Promise((resolve) => setTimeout(resolve, 100));
	}
	const arrived = exportedSpans(exports)
		.map((span) => `${span.name} (${span.spanId} < ${span.parentSpanId})`)
		.join(", ");
	throw new Error(
		`timed out waiting for ${description}; exported: ${arrived}`,
	);
}

function isSqliteSpan(span: ExportedSpan): boolean {
	return span.name === "rivet.sqlite.execute";
}

function isFailedSqliteSpan(span: ExportedSpan): boolean {
	return isSqliteSpan(span) && span.attributes["error.type"] !== undefined;
}

function findInvocation(
	spans: ExportedSpan[],
	actionName: string,
): ExportedSpan | undefined {
	return spans.find(
		(span) =>
			span.attributes["rivet.invocation.type"] !== undefined &&
			span.attributes["rivet.action.name"] === actionName,
	);
}

/** The `onRequest` invocation span carrying `rayId`, ignoring its children. */
function findRequestInvocation(
	spans: ExportedSpan[],
	rayId: string,
): ExportedSpan | undefined {
	return spans.find(
		(span) =>
			span.attributes["rivet.invocation.type"] === "request" &&
			span.attributes["rivet.ray.id"] === rayId,
	);
}

/**
 * The `queue.receive` span of `actorName` under `parentSpanId`, or its root
 * one when `parentSpanId` is undefined.
 */
function findQueueReceive(
	spans: ExportedSpan[],
	actorName: string,
	parentSpanId: string | undefined,
): ExportedSpan | undefined {
	return spans.find(
		(span) =>
			span.name === `${actorName}/queue.receive` &&
			span.parentSpanId === parentSpanId,
	);
}

/** The `queue.send` invocation span carrying `rayId`, ignoring its children. */
function findQueueSendInvocation(
	spans: ExportedSpan[],
	rayId: string,
): ExportedSpan | undefined {
	return spans.find(
		(span) =>
			span.attributes["rivet.invocation.type"] === "queue_send" &&
			span.attributes["rivet.ray.id"] === rayId,
	);
}

/** Polls until an invocation span has been exported for every named action. */
async function waitForInvocationSpans(
	exports: Buffer[],
	actionNames: string[],
	timeoutMs: number,
): Promise<ExportedSpan[]> {
	return waitForSpans(
		exports,
		`invocation spans: ${actionNames.join(", ")}`,
		(spans) => actionNames.every((name) => findInvocation(spans, name)),
		timeoutMs,
	);
}

async function waitForRuntimeLog(
	correlationToken: string,
	timeoutMs: number,
): Promise<string> {
	const deadline = Date.now() + timeoutMs;
	const marker = `correlation_token=${correlationToken}`;
	while (Date.now() < deadline) {
		const line = runtimeOutput()
			.split("\n")
			.find((candidate) => candidate.includes(marker));
		if (line) {
			return line;
		}
		await new Promise((resolve) => setTimeout(resolve, 100));
	}
	throw new Error(`timed out waiting for runtime log ${correlationToken}`);
}

/**
 * Starts an engine and a native runtime pointed at one OTLP endpoint, and
 * returns the pieces every telemetry test needs.
 */
async function startTracedRuntime(
	tracesEndpoint?: string,
	extraEnv: Record<string, string> = {},
): Promise<{ endpoint: string; poolName: string; child: ChildProcess }> {
	const poolName = "default";
	const port = await getPort({ host: "127.0.0.1" });
	const endpoint = `http://127.0.0.1:${port}`;
	engineEndpoint = endpoint;
	storagePath = await mkdtemp(join(tmpdir(), "rivetkit-services-"));
	runtimeLogs = { stdout: "", stderr: "" };
	const child = spawn(process.execPath, ["--import", "tsx", FIXTURE_PATH], {
		cwd: dirname(TEST_DIR),
		env: {
			...process.env,
			RIVET_TOKEN: TOKEN,
			RIVET_NAMESPACE: NAMESPACE,
			RIVET_RUN_ENGINE_HOST: "127.0.0.1",
			RIVET_RUN_ENGINE_PORT: String(port),
			RIVETKIT_TEST_ENDPOINT: endpoint,
			RIVETKIT_TEST_POOL_NAME: poolName,
			RIVETKIT_STORAGE_PATH: storagePath,
			// Export spans only for tests that read them.
			...(tracesEndpoint
				? {
						OTEL_EXPORTER_OTLP_TRACES_ENDPOINT: tracesEndpoint,
						OTEL_EXPORTER_OTLP_TRACES_PROTOCOL: "http/json",
						OTEL_TRACES_SAMPLER: "always_on",
						OTEL_BSP_SCHEDULE_DELAY: "10",
					}
				: {}),
			...extraEnv,
		},
		stdio: ["ignore", "pipe", "pipe"],
	});
	child.stdout?.on("data", (chunk) => {
		runtimeLogs.stdout += chunk.toString();
	});
	child.stderr?.on("data", (chunk) => {
		runtimeLogs.stderr += chunk.toString();
	});
	await waitForHealth(child, endpoint, 90_000);
	await upsertNormalRunnerConfig(child, endpoint, poolName);
	await waitForEnvoy(child, endpoint, poolName, 30_000);
	return { endpoint, poolName, child };
}

describe.sequential("native NAPI runtime integration", () => {
	let runtime: ChildProcess | undefined;
	let collector: OtlpCollector | undefined;

	afterEach(async () => {
		if (runtime) {
			await stopRuntime(runtime);
			runtime = undefined;
		}
		if (collector) {
			await collector.close();
			collector = undefined;
		}
		await stopTestEngine();
		if (storagePath) {
			await rm(storagePath, { recursive: true, force: true });
			storagePath = undefined;
		}
		engineEndpoint = undefined;
	}, 30_000);

	test("runs a TS actor through registry, NAPI, core, envoy, and engine", async () => {
		const { endpoint, poolName, child } = await startTracedRuntime();
		runtime = child;
		await waitForEnvoy(runtime, endpoint, SERVICES_POOL_NAME, 30_000);
		await expectNormalRunnerConfig(endpoint, SERVICES_POOL_NAME);
		const servicesActorId = await createServicesActor(endpoint);
		await waitForActorStarted(endpoint, servicesActorId, 30_000);

		const client = createIntegrationClient(endpoint, poolName);

		const actorKey = `napi-runtime-${crypto.randomUUID()}`;
		const handle = await waitForActorReady(
			() =>
				client.integrationActor.create([actorKey], {
					params: { userId: "integration-test" },
				}),
			30_000,
		);
		const actorId = await handle.resolve();

		expect(await waitForActorReady(() => handle.getCount(), 30_000)).toBe(
			0,
		);
		expect(
			await waitForActorReady(
				() => handle.validatedAction({ amount: 4 }),
				30_000,
			),
		).toBe(4);
		await expect(
			waitForActorReady(
				() => handle.validatedAction({ amount: "bad" }),
				30_000,
			),
		).rejects.toMatchObject({
			group: "actor",
			code: "validation_error",
		});
		expect(
			await waitForActorReady(
				() => handle.emitValidatedEvent({ count: 2 }),
				30_000,
			),
		).toBe(2);
		await expect(
			waitForActorReady(
				() => handle.emitValidatedEvent({ count: "bad" }),
				30_000,
			),
		).rejects.toMatchObject({
			group: "actor",
			code: "validation_error",
		});
		expect(
			await waitForActorReady(
				() => handle.enqueueValidatedJob({ id: "job-1" }),
				30_000,
			),
		).toBe("job-1");
		await expect(
			waitForActorReady(
				() => handle.enqueueValidatedJob({ id: "" }),
				30_000,
			),
		).rejects.toMatchObject({
			group: "actor",
			code: "validation_error",
		});

		expect(
			await waitForActorReady(() => handle.increment(2), 30_000),
		).toEqual({
			count: 2,
			sqliteValues: [2],
		});
		expect(await handle.snapshot()).toEqual({
			count: 2,
			kvCount: 2,
			sqliteValues: [2],
		});

		expect(await handle.goToSleep()).toEqual({ ok: true });
		await waitForActorSleep(endpoint, actorId, 30_000);

		expect(
			await waitForActorReady(
				() => handle.incrementWithoutSql(3),
				30_000,
			),
		).toEqual({
			count: 5,
		});
		expect(await handle.getCountViaClient()).toBe(5);
		expect(await handle.stateSnapshot()).toEqual({
			count: 5,
			kvCount: 5,
		});
		await expect(handle.throwTypedError()).rejects.toMatchObject({
			group: "user",
			code: "boom",
			message: "native typed error",
			metadata: {
				source: "native",
			},
		});
		await expect(handle.throwUntypedError()).rejects.toMatchObject({
			group: "rivetkit",
			code: "internal_error",
			message: "An internal error occurred",
		});

		await client.dispose();

		const processId = servicesPid();
		await stopRuntime(runtime);
		runtime = undefined;
		await waitForProcessExit(processId, 5_000);
	}, 120_000);

	test("preserves vendor trace state across actor calls and ignores invalid trace versions", async () => {
		collector = await startOtlpCollector(
			await getPort({ host: "127.0.0.1" }),
		);
		const traceExports = collector.spans();
		const { endpoint, poolName, child } = await startTracedRuntime(
			collector.endpoint,
		);
		runtime = child;
		const traceId = "1234567890abcdef1234567890abcdef";
		const parentSpanId = "1234567890abcdef";
		const traceState = "vendor=opaque-value";
		const client = createIntegrationClient(endpoint, poolName);
		const handle = await waitForActorReady(
			() =>
				client.integrationActor.create(
					[`trace-context-${crypto.randomUUID()}`],
					{ params: { userId: "integration-test" } },
				),
			30_000,
		);
		await waitForActorReady(() => handle.getCount(), 30_000);
		const actorId = await handle.resolve();
		const url = new URL(await handle.getGatewayUrl());
		url.pathname = `${url.pathname.replace(/\/$/, "")}/action/getCountViaClient`;
		// Every version calls the same actor, so each round takes the caller
		// span that no earlier round has claimed.
		const claimed = new Set<string>();
		const callChain = (spans: ExportedSpan[]) => {
			const caller = spans.find(
				(span) =>
					span.attributes["rivet.actor.id"] === actorId &&
					span.attributes["rivet.action.name"] ===
						"getCountViaClient" &&
					!claimed.has(span.spanId),
			);
			const hop =
				caller &&
				spans.find(
					(span) =>
						span.kind === OTLP_SPAN_KIND_CLIENT &&
						span.traceId === caller.traceId,
				);
			const callee =
				hop && spans.find((span) => span.parentSpanId === hop.spanId);
			return { caller, hop, callee };
		};
		for (const version of ["00", "zz", "0A"]) {
			const response = await fetch(url, {
				method: "POST",
				headers: {
					"content-type": "application/json",
					"x-rivet-encoding": "json",
					"x-rivet-token": TOKEN,
					"x-rivet-conn-params": JSON.stringify({
						userId: "integration-test",
					}),
					traceparent: `${version}-${traceId}-${parentSpanId}-01`,
					tracestate: traceState,
				},
				body: JSON.stringify({ args: [] }),
			});
			expect(response.status).toBe(200);
			await response.arrayBuffer();
			const { caller, hop, callee } = callChain(
				await waitForSpans(
					traceExports,
					"caller and callee trace contexts",
					(spans) => callChain(spans).callee !== undefined,
					10_000,
				),
			);
			claimed.add(caller?.spanId ?? "");
			if (version === "00") {
				expect(caller?.traceId).toBe(traceId);
				expect(caller?.parentSpanId).toBe(parentSpanId);
				for (const span of [caller, hop, callee])
					expect(span?.traceState).toBe(traceState);
			} else {
				expect(caller?.traceId).not.toBe(traceId);
				expect(caller?.parentSpanId).toBeUndefined();
				for (const span of [caller, hop, callee])
					expect(span?.traceState || "").toBe("");
			}
		}
		await client.dispose();
	}, 120_000);

	test("keeps overlapping invocations of one actor telemetrically isolated", async () => {
		collector = await startOtlpCollector(
			await getPort({ host: "127.0.0.1" }),
		);
		const traceExports = collector.spans();
		const { endpoint, poolName, child } = await startTracedRuntime(
			collector.endpoint,
		);
		runtime = child;

		const client = createIntegrationClient(endpoint, poolName);
		const handle = await waitForActorReady(
			() =>
				client.integrationActor.create(
					[`napi-isolation-${crypto.randomUUID()}`],
					{ params: { userId: "integration-test" } },
				),
			30_000,
		);
		await waitForActorReady(() => handle.getCount(), 30_000);

		// Use the same actor to exercise isolation between concurrent invocations.
		const okToken = crypto.randomUUID();
		const failToken = crypto.randomUUID();
		const results = Promise.allSettled([
			handle.isolationProbe(okToken, false),
			handle.isolationProbe(failToken, true),
		]);
		const [okLog, failLog] = await Promise.all([
			waitForRuntimeLog(okToken, 10_000),
			waitForRuntimeLog(failToken, 10_000),
		]);
		await handle.send("jobs", { id: okToken });
		await handle.send("jobs", { id: failToken });
		const [ok, failed] = await results;
		expect(ok.status).toBe("fulfilled");
		expect(failed.status).toBe("rejected");

		const spans = await waitForSpans(
			traceExports,
			"both isolation probe invocations and the calls each one made",
			(exported) => {
				const probes = exported.filter(
					(span) =>
						span.attributes["rivet.action.name"] ===
						"isolationProbe",
				);
				return (
					probes.length >= 2 &&
					probes.every((probe) => {
						const hop = exported.find(
							(span) =>
								span.kind === OTLP_SPAN_KIND_CLIENT &&
								span.traceId === probe.traceId,
						);
						return (
							!!hop &&
							exported.some(
								(span) => span.parentSpanId === hop.spanId,
							) &&
							exported.filter(
								(span) =>
									isSqliteSpan(span) &&
									span.parentSpanId === probe.spanId,
							).length >= 2
						);
					})
				);
			},
			20_000,
		);

		const probes = spans.filter(
			(span) => span.attributes["rivet.action.name"] === "isolationProbe",
		);
		expect(probes).toHaveLength(2);

		const rays = probes.map((probe) => probe.attributes["rivet.ray.id"]);
		expect(new Set(rays).size).toBe(2);
		expect(new Set(probes.map((probe) => probe.traceId)).size).toBe(2);
		const failedProbes = probes.filter(
			(probe) => probe.attributes["error.type"] !== undefined,
		);
		expect(failedProbes).toHaveLength(1);
		expect(failedProbes[0]?.attributes["error.type"]).toBe(
			"user.isolation_probe_failed",
		);

		for (const probe of probes) {
			const owned = spans.filter(
				(span) =>
					isSqliteSpan(span) && span.parentSpanId === probe.spanId,
			);
			expect(owned.length).toBeGreaterThanOrEqual(2);
			for (const span of owned) {
				expect(span.traceId).toBe(probe.traceId);
				expect(span.attributes["rivet.ray.id"]).toBe(
					probe.attributes["rivet.ray.id"],
				);
			}
		}

		for (const probe of probes) {
			const hop = spans.find(
				(span) =>
					span.kind === OTLP_SPAN_KIND_CLIENT &&
					span.traceId === probe.traceId,
			);
			const callee = spans.find(
				(span) =>
					span.attributes["rivet.invocation.type"] !== undefined &&
					span.attributes["rivet.action.name"] === "getCount" &&
					span.traceId === probe.traceId,
			);
			expect(hop).toBeDefined();
			expect(callee).toBeDefined();
			expect(hop?.attributes["rivet.actor.name"]).toBe(
				"integrationActor",
			);
			expect(hop?.parentSpanId).toBe(probe.spanId);
			expect(callee?.parentSpanId).toBe(hop?.spanId);
			for (const span of [hop, callee]) {
				expect(span?.attributes["rivet.ray.id"]).toBe(
					probe.attributes["rivet.ray.id"],
				);
			}
		}

		for (const line of [okLog, failLog]) {
			expect(line).toContain(`actorId=${await handle.resolve()}`);
			expect(line).toMatch(/ trace_id=[0-9a-f]{32}( |$)/);
			expect(line).toMatch(/ span_id=[0-9a-f]{16}( |$)/);
		}
		const rayOf = (line: string) =>
			/ rayId=([A-Za-z0-9_-]+)/.exec(line)?.[1];
		expect(rayOf(okLog)).toBeDefined();
		expect(rayOf(okLog)).not.toBe(rayOf(failLog));
		expect(rays).toContain(rayOf(okLog));
		expect(rays).toContain(rayOf(failLog));

		// SQLite and actor calls inherit the active application span.
		const underApp = await handle.getCountUnderApplicationSpan();
		const applicationSpans = await waitForSpans(
			traceExports,
			"SQLite, outgoing call, and callee under the application span",
			(exported) => {
				const hop = exported.find(
					(span) =>
						span.kind === OTLP_SPAN_KIND_CLIENT &&
						span.parentSpanId === underApp.spanId,
				);
				return (
					!!hop &&
					exported.some((span) => span.parentSpanId === hop.spanId) &&
					exported.some(
						(span) =>
							isSqliteSpan(span) &&
							span.parentSpanId === underApp.spanId,
					)
				);
			},
			10_000,
		);
		const appHop = applicationSpans.find(
			(span) =>
				span.kind === OTLP_SPAN_KIND_CLIENT &&
				span.parentSpanId === underApp.spanId,
		);
		expect(
			applicationSpans.find(
				(span) => span.parentSpanId === appHop?.spanId,
			)?.attributes["rivet.action.name"],
		).toBe("getCount");
		expect(
			applicationSpans.find(
				(span) =>
					isSqliteSpan(span) && span.parentSpanId === underApp.spanId,
			)?.attributes["rivet.operation.name"],
		).toBe("execute");

		await client.dispose();
	}, 120_000);

	test("carries a caller-supplied ray through requests, queue sends, and work after the reply", async () => {
		collector = await startOtlpCollector(
			await getPort({ host: "127.0.0.1" }),
		);
		const traceExports = collector.spans();
		const { endpoint, poolName, child } = await startTracedRuntime(
			collector.endpoint,
		);
		runtime = child;

		const client = createIntegrationClient(endpoint, poolName);
		const handle = await waitForActorReady(
			() =>
				client.integrationActor.create(
					[`napi-caller-ray-${crypto.randomUUID()}`],
					{ params: { userId: "integration-test" } },
				),
			30_000,
		);
		await waitForActorReady(() => handle.getCount(), 30_000);

		const callerRay = `caller-${crypto.randomUUID()}`;
		await withRayBaggage(callerRay, () => handle.getCount());
		const rayedGetCount = await waitForSpans(
			traceExports,
			"the getCount invocation carrying the caller-supplied ray",
			(exported) =>
				exported.some(
					(span) =>
						span.attributes["rivet.action.name"] === "getCount" &&
						span.attributes["rivet.ray.id"] === callerRay,
				),
			10_000,
		);
		expect(
			rayedGetCount.find(
				(span) => span.attributes["rivet.ray.id"] === callerRay,
			)?.attributes["rivet.invocation.type"],
		).toBe("action");

		// Fetch inherits the active span and baggage without explicit headers.
		const requestRay = `request-${crypto.randomUUID()}`;
		const callerSpan = testTracer.startSpan("request.handle");
		const underCallerSpan = <T>(run: () => Promise<T>) =>
			withRayBaggage(requestRay, () =>
				context.with(trace.setSpan(context.active(), callerSpan), run),
			);
		const response = await underCallerSpan(() => handle.fetch("hello"));
		expect(response.status).toBe(200);
		const requestSpans = await waitForSpans(
			traceExports,
			"the onRequest invocation and its sqlite span",
			(exported) => {
				const request = findRequestInvocation(exported, requestRay);
				return (
					request !== undefined &&
					exported.some(
						(span) =>
							isSqliteSpan(span) &&
							span.parentSpanId === request.spanId,
					)
				);
			},
			10_000,
		);
		const requestSpan = findRequestInvocation(requestSpans, requestRay);
		expect(requestSpan?.name).toBe("integrationActor/onRequest");
		expect(requestSpan?.attributes["http.response.status_code"]).toBe(
			"200",
		);
		expect(requestSpan?.traceId).toBe(callerSpan.spanContext().traceId);
		expect(requestSpan?.parentSpanId).toBe(callerSpan.spanContext().spanId);

		// Headers the caller set on the request win over the active context.
		const explicitRay = `explicit-${crypto.randomUUID()}`;
		const explicitTraceId = crypto.randomUUID().replaceAll("-", "");
		const explicitSpanId = crypto
			.randomUUID()
			.replaceAll("-", "")
			.slice(0, 16);
		const explicitResponse = await underCallerSpan(() =>
			handle.fetch("hello", {
				headers: {
					"x-rivet-ray-id": explicitRay,
					traceparent: `00-${explicitTraceId}-${explicitSpanId}-01`,
				},
			}),
		);
		callerSpan.end();
		expect(explicitResponse.status).toBe(200);
		const explicitSpans = await waitForSpans(
			traceExports,
			"the onRequest invocation under the caller's own headers",
			(exported) =>
				findRequestInvocation(exported, explicitRay) !== undefined,
			10_000,
		);
		const explicitSpan = findRequestInvocation(explicitSpans, explicitRay);
		expect(explicitSpan?.traceId).toBe(explicitTraceId);
		expect(explicitSpan?.parentSpanId).toBe(explicitSpanId);

		// Release deferred work after the reply; its invocation must remain open.
		const deferredToken = crypto.randomUUID();
		expect(await handle.insertAfterReply(deferredToken)).toBe("replied");
		await handle.send("jobs", { id: deferredToken });
		const deferredSpans = await waitForSpans(
			traceExports,
			"the insertAfterReply invocation and its deferred sqlite span",
			(exported) => {
				const invocation = findInvocation(exported, "insertAfterReply");
				return (
					invocation !== undefined &&
					exported.some(
						(span) =>
							isSqliteSpan(span) &&
							span.parentSpanId === invocation.spanId,
					)
				);
			},
			10_000,
		);
		const deferredInvocation = findInvocation(
			deferredSpans,
			"insertAfterReply",
		);
		const deferredSqlite = deferredSpans.find(
			(span) =>
				isSqliteSpan(span) &&
				span.parentSpanId === deferredInvocation?.spanId,
		);
		expect(deferredSqlite).toBeDefined();
		expect(
			deferredInvocation !== undefined &&
				deferredSqlite !== undefined &&
				deferredInvocation.endTimeUnixNano >=
					deferredSqlite.endTimeUnixNano,
		).toBe(true);

		// The action receipt links to the send and keeps the consuming action’s ray.
		const queueRay = `queue-${crypto.randomUUID()}`;
		await withRayBaggage(queueRay, () =>
			handle.send("jobs", { id: "job-42" }),
		);
		expect(await handle.consumeJob()).toEqual({ id: "job-42" });
		const queueSpans = await waitForSpans(
			traceExports,
			"the queue send invocation, the consuming action, and its receipt",
			(exported) => {
				const consumer = findInvocation(exported, "consumeJob");
				return (
					findQueueSendInvocation(exported, queueRay) !== undefined &&
					consumer !== undefined &&
					findQueueReceive(
						exported,
						"integrationActor",
						consumer.spanId,
					) !== undefined
				);
			},
			10_000,
		);
		const queueSend = findQueueSendInvocation(queueSpans, queueRay);
		expect(queueSend?.name).toBe("integrationActor/queue.send");
		expect(queueSend?.attributes).toMatchObject({
			"rivet.invocation.type": "queue_send",
			"rivet.queue.name": "jobs",
		});
		const consumer = findInvocation(queueSpans, "consumeJob");
		const receipt = findQueueReceive(
			queueSpans,
			"integrationActor",
			consumer?.spanId,
		);
		expect(receipt?.attributes).toMatchObject({
			"rivet.queue.name": "jobs",
			"rivet.ray.id": consumer?.attributes["rivet.ray.id"],
		});
		expect(receipt?.links).toEqual([
			{ traceId: queueSend?.traceId, spanId: queueSend?.spanId },
		]);

		// Without an invocation, the receipt is a root span carrying the sender’s ray.
		const runRay = `run-${crypto.randomUUID()}`;
		const runConsumer = client.runConsumerActor.getOrCreate([
			`napi-run-consumer-${crypto.randomUUID()}`,
		]);
		await withRayBaggage(runRay, () =>
			runConsumer.send("runJobs", { id: "job-run" }),
		);
		const runSpans = await waitForSpans(
			traceExports,
			"the run handler's receipt of a queue message",
			(exported) =>
				findQueueSendInvocation(exported, runRay) !== undefined &&
				findQueueReceive(exported, "runConsumerActor", undefined) !==
					undefined,
			10_000,
		);
		const runSend = findQueueSendInvocation(runSpans, runRay);
		const runReceipt = findQueueReceive(
			runSpans,
			"runConsumerActor",
			undefined,
		);
		expect(runReceipt?.attributes).toMatchObject({
			"rivet.queue.name": "runJobs",
			"rivet.ray.id": runRay,
		});
		expect(runReceipt?.links).toEqual([
			{ traceId: runSend?.traceId, spanId: runSend?.spanId },
		]);

		traceExports.length = 0;
		await expect(handle.sqliteFailure()).rejects.toMatchObject({
			code: expect.any(String),
		});
		const failureSpans = await waitForSpans(
			traceExports,
			"sqliteFailure invocation and failed sqlite spans",
			(spans) =>
				spans.some(isFailedSqliteSpan) &&
				findInvocation(spans, "sqliteFailure") !== undefined,
			10_000,
		);
		const failedSqlite = failureSpans.find(isFailedSqliteSpan);
		expect(failedSqlite?.parentSpanId).toBe(
			findInvocation(failureSpans, "sqliteFailure")?.spanId,
		);

		// Scheduled work starts a new trace linked to its origin.
		traceExports.length = 0;
		const scheduleToken = crypto.randomUUID();
		expect(await handle.scheduleTrace(scheduleToken)).toBe(scheduleToken);
		const scheduleSpans = await waitForInvocationSpans(
			traceExports,
			["scheduleTrace", "scheduledTrace"],
			15_000,
		);
		const definer = findInvocation(scheduleSpans, "scheduleTrace");
		const scheduled = findInvocation(scheduleSpans, "scheduledTrace");
		expect(scheduled?.attributes["rivet.invocation.type"]).toBe(
			"scheduled",
		);
		expect(scheduled?.attributes["rivet.ray.id"]).toBe(
			definer?.attributes["rivet.ray.id"],
		);
		// Ensure the scheduled action succeeded before checking its trace.
		expect(scheduled?.attributes["error.type"]).toBeUndefined();
		expect(scheduled?.traceId).not.toBe(definer?.traceId);
		expect(scheduled?.links).toEqual([
			{ traceId: definer?.traceId, spanId: definer?.spanId },
		]);
		await client.dispose();
	}, 120_000);

	test("keeps actor behavior intact when the trace exporter is unavailable", async () => {
		// Nothing listens on this port, so every OTLP export attempt fails.
		const unavailable = `http://127.0.0.1:${await getPort({ host: "127.0.0.1" })}/v1/traces`;
		const { endpoint, poolName, child } =
			await startTracedRuntime(unavailable);
		runtime = child;

		const client = createIntegrationClient(endpoint, poolName);
		const handle = await waitForActorReady(
			() =>
				client.integrationActor.create(
					[`napi-telemetry-failure-${crypto.randomUUID()}`],
					{ params: { userId: "integration-test" } },
				),
			30_000,
		);

		expect(await waitForActorReady(() => handle.getCount(), 30_000)).toBe(
			0,
		);
		expect(
			await waitForActorReady(
				() => handle.validatedAction({ amount: 4 }),
				30_000,
			),
		).toBe(4);

		await client.dispose();
	}, 120_000);

	test("keeps actor behavior intact when the trace exporter is slow", async () => {
		// Stall exports long enough to saturate the small queue below.
		collector = await startOtlpCollector(
			await getPort({ host: "127.0.0.1" }),
			{
				responseDelayMs: 120_000,
			},
		);
		const { endpoint, poolName, child } = await startTracedRuntime(
			collector.endpoint,
			{
				OTEL_BSP_MAX_QUEUE_SIZE: "8",
				OTEL_BSP_MAX_EXPORT_BATCH_SIZE: "4",
				// Disable Rust SDK logs so only the JS bridge can satisfy the log assertion.
				RUST_LOG: "warn,opentelemetry_sdk=off",
			},
		);
		runtime = child;

		const client = createIntegrationClient(endpoint, poolName);
		const handle = await waitForActorReady(
			() =>
				client.integrationActor.create(
					[`napi-telemetry-slow-${crypto.randomUUID()}`],
					{ params: { userId: "integration-test" } },
				),
			30_000,
		);

		const started = Date.now();
		for (let index = 1; index <= 12; index += 1) {
			expect(
				await waitForActorReady(() => handle.increment(1), 30_000),
			).toMatchObject({ count: index });
		}
		const elapsed = Date.now() - started;

		// Actions must finish before the stalled collector responds.
		expect(elapsed).toBeLessThan(60_000);

		// The processor reports dropped spans on its own export cycle, which
		// runs after the actions return, so there is nothing to await.
		await vi.waitFor(
			() => {
				expect(runtimeOutput()).toContain(
					"BatchSpanProcessor.SpanDroppingStarted",
				);
			},
			{ timeout: 15_000, interval: 250 },
		);

		expect(await waitForActorReady(() => handle.getCount(), 30_000)).toBe(
			12,
		);

		await client.dispose();
	}, 180_000);
});

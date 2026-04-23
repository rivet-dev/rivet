// Soak harness for Cloud Run kitchen-sink-staging.
//
// Goals: (1) verify correctness of unstable code, (2) validate Cloud Run
// autoscale, (3) detect memory leaks, (4) inform Cloud Run tuning.
//
// Usage:
//   pnpm tsx scripts/soak.ts --mode=churn [--duration-min=N] [--skip-revision-bump]
//   pnpm tsx scripts/soak.ts --mode=steady
//   pnpm tsx scripts/soak.ts --mode=scale
//
// The script does NOT mutate Cloud Run service config (memory, cpu, maxScale,
// containerConcurrency). Configure those once at the service level. The script
// only bumps the SOAK_RUN_ID env var to force a fresh revision so memory
// baselines are clean and metrics can be filtered by revision_name.

import { execFile } from "node:child_process";
import { createWriteStream, type WriteStream } from "node:fs";
import { tmpdir } from "node:os";
import { promisify } from "node:util";
import { randomBytes } from "node:crypto";
import { setTimeout as sleep } from "node:timers/promises";
import { performance } from "node:perf_hooks";
import { createClient, type Client } from "rivetkit/client";
import type { registry } from "../src/index.ts";

const execFileAsync = promisify(execFile);

// Hardcoded staging target. Do not repoint at production from this script.
const SERVICE = {
	name: "kitchen-sink-staging",
	region: "us-east4",
	project: "dev-projects-491221",
} as const;
const NAMESPACE = "kitchen-sink-gv34-staging-52gh";
const ENGINE_HOST = "api.staging.rivet.dev";

type Mode = "churn" | "steady" | "scale";

interface Args {
	mode: Mode;
	durationMin?: number;
	skipRevisionBump: boolean;
}

function parseArgs(argv: string[]): Args {
	let mode: Mode | undefined;
	let durationMin: number | undefined;
	let skipRevisionBump = false;

	for (let i = 0; i < argv.length; i += 1) {
		const arg = argv[i];
		if (arg.startsWith("--mode=")) {
			const value = arg.slice("--mode=".length);
			if (value !== "churn" && value !== "steady" && value !== "scale") {
				die(`invalid mode: ${value}`);
			}
			mode = value;
		} else if (arg === "--mode") {
			const value = argv[i + 1];
			i += 1;
			if (value !== "churn" && value !== "steady" && value !== "scale") {
				die(`invalid mode: ${value}`);
			}
			mode = value;
		} else if (arg.startsWith("--duration-min=")) {
			durationMin = Number(arg.slice("--duration-min=".length));
		} else if (arg === "--duration-min") {
			durationMin = Number(argv[i + 1]);
			i += 1;
		} else if (arg === "--skip-revision-bump") {
			skipRevisionBump = true;
		} else if (arg === "--help" || arg === "-h") {
			printUsage();
			process.exit(0);
		} else {
			die(`unknown arg: ${arg}`);
		}
	}

	if (!mode) {
		printUsage();
		die("--mode is required");
	}
	return { mode: mode as Mode, durationMin, skipRevisionBump };
}

function printUsage(): void {
	process.stdout.write(
		"Usage: pnpm tsx scripts/soak.ts --mode={churn|steady|scale} [--duration-min=N] [--skip-revision-bump]\n",
	);
}

function die(message: string): never {
	process.stderr.write(`[soak] error: ${message}\n`);
	process.exit(1);
}

function progress(message: string): void {
	process.stdout.write(`[soak] ${message}\n`);
}

function timestampSlug(): string {
	const d = new Date();
	const pad = (n: number) => String(n).padStart(2, "0");
	return `${d.getUTCFullYear()}${pad(d.getUTCMonth() + 1)}${pad(d.getUTCDate())}-${pad(d.getUTCHours())}${pad(d.getUTCMinutes())}${pad(d.getUTCSeconds())}`;
}

class JsonlWriter {
	private stream: WriteStream;
	private closed = false;
	constructor(public readonly path: string) {
		this.stream = createWriteStream(path, { flags: "a" });
		// Background tasks (ping intervals, ws listeners) may still fire after the
		// workload returns. Drop their writes silently rather than crash.
		this.stream.on("error", () => undefined);
	}
	write(event: Record<string, unknown>): void {
		if (this.closed) return;
		try {
			this.stream.write(
				`${JSON.stringify({ ts: new Date().toISOString(), ...event })}\n`,
			);
		} catch {
			// best-effort observability
		}
	}
	async close(): Promise<void> {
		this.closed = true;
		await new Promise<void>((resolve) => {
			this.stream.end(() => resolve());
		});
	}
}

interface DescribeService {
	spec?: {
		template?: {
			spec?: {
				containers?: Array<{
					env?: Array<{ name: string; value?: string }>;
				}>;
			};
		};
	};
	status?: {
		latestReadyRevisionName?: string;
	};
}

async function describeService(): Promise<DescribeService> {
	const { stdout } = await execFileAsync("gcloud", [
		"run",
		"services",
		"describe",
		SERVICE.name,
		`--region=${SERVICE.region}`,
		`--project=${SERVICE.project}`,
		"--format=json",
	]);
	return JSON.parse(stdout) as DescribeService;
}

async function getStagingSecretToken(): Promise<string> {
	const desc = await describeService();
	const env = desc.spec?.template?.spec?.containers?.[0]?.env ?? [];
	const entry = env.find((e) => e.name === "RIVET_ENDPOINT");
	if (!entry?.value) die("RIVET_ENDPOINT not set on Cloud Run service");
	const url = new URL(entry.value);
	if (!url.password) die("RIVET_ENDPOINT has no token (password component)");
	return url.password;
}

async function getCurrentRevision(): Promise<string> {
	const desc = await describeService();
	const rev = desc.status?.latestReadyRevisionName;
	if (!rev) die("no latestReadyRevisionName on service");
	return rev;
}

async function bumpRevision(runId: string, prevRevision: string | null): Promise<string> {
	await execFileAsync(
		"gcloud",
		[
			"run",
			"services",
			"update",
			SERVICE.name,
			`--region=${SERVICE.region}`,
			`--project=${SERVICE.project}`,
			`--update-env-vars=SOAK_RUN_ID=${runId}`,
		],
		{ maxBuffer: 32 * 1024 * 1024 },
	);
	// gcloud blocks until the operation completes, but Knative status conditions
	// may take a moment to reconcile. Poll until latestReadyRevisionName changes.
	for (let attempt = 0; attempt < 30; attempt += 1) {
		const rev = await getCurrentRevision();
		if (rev !== prevRevision) return rev;
		await sleep(2000);
	}
	throw new Error(`revision did not change from ${prevRevision} within 60s after bump`);
}

async function gcloudAccessToken(): Promise<string> {
	const { stdout } = await execFileAsync("gcloud", ["auth", "print-access-token"]);
	return stdout.trim();
}

const METRIC_TYPES = [
	"run.googleapis.com/container/memory/utilizations",
	"run.googleapis.com/container/cpu/utilizations",
	"run.googleapis.com/container/instance_count",
	"run.googleapis.com/request_count",
	"run.googleapis.com/request_latencies",
	"run.googleapis.com/container/billable_instance_time",
] as const;

interface MetricPoint {
	ts: string;
	value: number | null;
}

interface MetricSeries {
	metric: string;
	labels: Record<string, string>;
	points: MetricPoint[];
}

function pointValue(p: { value: Record<string, unknown> }): number | null {
	const v = p.value;
	if (typeof v.doubleValue === "number") return v.doubleValue;
	if (typeof v.int64Value === "string") return Number(v.int64Value);
	if (typeof v.int64Value === "number") return v.int64Value;
	if (v.distributionValue && typeof (v.distributionValue as { mean?: number }).mean === "number") {
		return (v.distributionValue as { mean: number }).mean;
	}
	return null;
}

async function fetchMetricSeries(
	metric: string,
	revision: string,
	startISO: string,
	endISO: string,
	token: string,
): Promise<MetricSeries[]> {
	let filter =
		`metric.type = "${metric}" ` +
		`AND resource.labels.service_name = "${SERVICE.name}" ` +
		`AND resource.labels.revision_name = "${revision}"`;
	// instance_count returns one series per state (active, idle). Filter to
	// active so the verdict's max reflects instances actually serving traffic.
	if (metric.endsWith("/instance_count")) {
		filter += ` AND metric.labels.state = "active"`;
	}
	const params = new URLSearchParams({
		filter,
		"interval.startTime": startISO,
		"interval.endTime": endISO,
	});
	const url = `https://monitoring.googleapis.com/v3/projects/${SERVICE.project}/timeSeries?${params}`;
	const res = await fetch(url, {
		headers: { Authorization: `Bearer ${token}` },
	});
	if (!res.ok) {
		const text = await res.text();
		throw new Error(`monitoring api ${res.status}: ${text}`);
	}
	const body = (await res.json()) as {
		timeSeries?: Array<{
			resource: { labels: Record<string, string> };
			points: Array<{
				interval: { startTime: string; endTime: string };
				value: Record<string, unknown>;
			}>;
		}>;
	};
	return (body.timeSeries ?? []).map((ts) => ({
		metric,
		labels: ts.resource.labels,
		points: ts.points
			.map((p) => ({
				ts: p.interval.endTime,
				value: pointValue(p),
			}))
			.reverse(),
	}));
}

interface LogEntry {
	timestamp: string;
	severity: string;
	textPayload?: string;
	jsonPayload?: Record<string, unknown>;
	resource?: { labels?: Record<string, string> };
}

async function fetchErrorLogs(
	revision: string,
	startISO: string,
	endISO: string,
): Promise<LogEntry[]> {
	const filter =
		`resource.type="cloud_run_revision" ` +
		`AND resource.labels.service_name="${SERVICE.name}" ` +
		`AND resource.labels.revision_name="${revision}" ` +
		`AND severity>=ERROR ` +
		`AND timestamp>="${startISO}" AND timestamp<="${endISO}"`;
	const { stdout } = await execFileAsync(
		"gcloud",
		[
			"logging",
			"read",
			filter,
			`--project=${SERVICE.project}`,
			"--limit=1000",
			"--format=json",
		],
		{ maxBuffer: 64 * 1024 * 1024 },
	);
	if (!stdout.trim()) return [];
	return JSON.parse(stdout) as LogEntry[];
}

function defaultDurationMin(mode: Mode): number {
	switch (mode) {
		case "churn":
			return 30;
		case "steady":
			return 30;
		case "scale":
			return 10;
	}
}

// Best-effort action runner. Records assertion failures to the JSONL but never
// throws — soak workloads should keep running through transient errors and let
// the post-hoc verdict count them.
async function safeCall<T>(
	jsonl: JsonlWriter,
	step: string,
	context: Record<string, unknown>,
	fn: () => Promise<T>,
): Promise<T | undefined> {
	try {
		return await fn();
	} catch (err) {
		jsonl.write({
			event: "assertion_failure",
			step,
			context,
			error: err instanceof Error ? err.message : String(err),
		});
		return undefined;
	}
}

interface Stats {
	cycles: number;
	failures: number;
}

async function runChurn(
	client: Client<typeof registry>,
	runId: string,
	jsonl: JsonlWriter,
	durationMs: number,
): Promise<Stats> {
	const WORKERS = 4;
	const startedAt = performance.now();
	const stats: Stats = { cycles: 0, failures: 0 };

	const workers = Array.from({ length: WORKERS }, async (_, workerIdx) => {
		let localCycle = 0;
		while (performance.now() - startedAt < durationMs) {
			const cycleId = `${workerIdx}-${localCycle}`;
			const choice = localCycle % 2;
			if (choice === 0) {
				// Full lifecycle through destroyActor: create + setValue + destroy.
				const key = [`soak-${runId}-destroy-${cycleId}`];
				const handle = client.destroyActor.getOrCreate(key);
				const ok = await safeCall(jsonl, "churn_destroy", { key }, async () => {
					const v = await handle.setValue(localCycle);
					if (v !== localCycle) {
						throw new Error(`setValue returned ${v}, expected ${localCycle}`);
					}
					await handle.destroy();
					return true;
				});
				if (!ok) stats.failures += 1;
			} else {
				// Sleep cycle: wake + triggerSleep on the sleep actor.
				const key = [`soak-${runId}-sleep-${cycleId}`];
				const handle = client.sleep.getOrCreate(key);
				const ok = await safeCall(jsonl, "churn_sleep", { key }, async () => {
					const counts = await handle.getCounts();
					if (typeof counts.startCount !== "number") {
						throw new Error("getCounts returned non-numeric startCount");
					}
					await handle.triggerSleep();
					return true;
				});
				if (!ok) stats.failures += 1;
			}
			stats.cycles += 1;
			localCycle += 1;
		}
	});

	const shutdown = new AbortController();
	const reporter = (async () => {
		while (performance.now() - startedAt < durationMs && !shutdown.signal.aborted) {
			try {
				await sleep(60_000, undefined, { signal: shutdown.signal });
			} catch {
				break;
			}
			const elapsedS = Math.floor((performance.now() - startedAt) / 1000);
			const totalS = Math.floor(durationMs / 1000);
			const pct = Math.floor((elapsedS / totalS) * 100);
			progress(
				`progress ${pct}% (${elapsedS}/${totalS}s) cycles=${stats.cycles} failures=${stats.failures}`,
			);
			jsonl.write({
				event: "progress",
				elapsed_s: elapsedS,
				cycles: stats.cycles,
				failures: stats.failures,
			});
		}
	})();

	await Promise.all(workers);
	shutdown.abort();
	await reporter.catch(() => undefined);
	return stats;
}

async function runSteady(
	client: Client<typeof registry>,
	runId: string,
	jsonl: JsonlWriter,
): Promise<Stats> {
	const STEPS = (process.env.SOAK_STEADY_STEPS ?? "50,100,200,400,800")
		.split(",")
		.map((s) => Number(s.trim()))
		.filter((n) => Number.isFinite(n) && n > 0);
	const STEP_HOLD_MS = Number(process.env.SOAK_STEADY_HOLD_MS ?? 5 * 60_000);
	const QUIESCE_MS = Number(process.env.SOAK_STEADY_QUIESCE_MS ?? 5 * 60_000);
	const stats: Stats = { cycles: 0, failures: 0 };
	let createdCount = 0;
	const heldKeys: { kind: "counter" | "sqlite" | "kv"; key: string[] }[] = [];

	for (const target of STEPS) {
		progress(`steady step target=${target} (currently held=${heldKeys.length})`);
		while (heldKeys.length < target) {
			const i = createdCount;
			createdCount += 1;
			const r = i % 5;
			let kind: "counter" | "sqlite" | "kv";
			if (r < 2) kind = "counter";
			else if (r < 4) kind = "sqlite";
			else kind = "kv";
			const key = [`soak-${runId}-${kind}-${i}`];
			const ok = await safeCall(jsonl, "steady_create", { kind, key }, async () => {
				if (kind === "counter") {
					await client.counter.getOrCreate(key).increment(1);
				} else if (kind === "sqlite") {
					await client.sqliteRawActor.getOrCreate(key).addTodo(`row-${i}`);
				} else {
					await client.kvActor.getOrCreate(key).putText(`k${i}`, `v${i}`);
				}
				return true;
			});
			if (ok) {
				heldKeys.push({ kind, key });
				stats.cycles += 1;
			} else {
				stats.failures += 1;
			}
		}
		jsonl.write({ event: "step_reached", target, holding: heldKeys.length });
		progress(`step ${target} reached, holding ${STEP_HOLD_MS / 1000}s before next step`);
		await sleep(STEP_HOLD_MS);
		jsonl.write({ event: "step_sample", target });
	}

	progress(`all steps done. quiesce ${QUIESCE_MS / 1000}s with no activity`);
	jsonl.write({ event: "quiesce_start", held: heldKeys.length });
	await sleep(QUIESCE_MS);
	jsonl.write({ event: "quiesce_complete" });
	return stats;
}

async function runScale(
	client: Client<typeof registry>,
	runId: string,
	jsonl: JsonlWriter,
	durationMs: number,
): Promise<Stats> {
	const N_WS = Number(process.env.SOAK_SCALE_WS_COUNT ?? 200);
	const N_RPS_TARGET = Number(process.env.SOAK_SCALE_RPS ?? 50);
	const stats: Stats = { cycles: 0, failures: 0 };

	progress(`opening ${N_WS} websockets...`);
	type WsHandle = { ws: WebSocket; key: string[]; pingTimer?: NodeJS.Timeout };
	const conns: WsHandle[] = [];
	for (let i = 0; i < N_WS; i += 1) {
		const key = [`soak-${runId}-ws-${i}`];
		const handle = client.rawWebSocketActor.getOrCreate(key);
		try {
			const ws = (await handle.webSocket()) as unknown as WebSocket;
			if (ws.readyState !== WebSocket.OPEN) {
				await new Promise<void>((resolve, reject) => {
					ws.addEventListener("open", () => resolve(), { once: true });
					ws.addEventListener("error", () => reject(new Error("ws error")), { once: true });
					ws.addEventListener("close", () => reject(new Error("ws closed before open")), { once: true });
				});
			}
			ws.addEventListener("error", (ev) => {
				jsonl.write({ event: "ws_error", key, message: String((ev as { message?: string }).message ?? "") });
				stats.failures += 1;
			});
			ws.addEventListener("close", (ev) => {
				jsonl.write({ event: "ws_close", key, code: (ev as CloseEvent).code });
			});
			conns.push({ ws, key });
			if (i % 50 === 49) progress(`opened ${i + 1}/${N_WS} websockets`);
		} catch (err) {
			jsonl.write({
				event: "ws_open_failed",
				key,
				error: err instanceof Error ? err.message : String(err),
			});
			stats.failures += 1;
		}
	}
	jsonl.write({ event: "ws_opened", count: conns.length, target: N_WS });
	progress(`websockets opened: ${conns.length}/${N_WS}. holding ${durationMs / 1000}s`);

	let pingCount = 0;
	let counterRpcCount = 0;
	let nextCounterIdx = 0;

	for (const c of conns) {
		c.pingTimer = setInterval(() => {
			if (c.ws.readyState === WebSocket.OPEN) {
				c.ws.send(JSON.stringify({ type: "ping" }));
				pingCount += 1;
			}
		}, 1000);
	}

	const startedAt = performance.now();
	const shutdown = new AbortController();
	const counterTask = (async () => {
		const intervalMs = Math.max(1, Math.floor(1000 / N_RPS_TARGET));
		while (performance.now() - startedAt < durationMs && !shutdown.signal.aborted) {
			const i = nextCounterIdx;
			nextCounterIdx += 1;
			const key = [`soak-${runId}-c-${i}`];
			const ok = await safeCall(jsonl, "scale_counter", { key }, async () => {
				await client.counter.getOrCreate(key).noop();
				return true;
			});
			if (!ok) stats.failures += 1;
			else counterRpcCount += 1;
			try {
				await sleep(intervalMs, undefined, { signal: shutdown.signal });
			} catch {
				break;
			}
		}
	})();

	const reporter = (async () => {
		while (performance.now() - startedAt < durationMs && !shutdown.signal.aborted) {
			try {
				await sleep(30_000, undefined, { signal: shutdown.signal });
			} catch {
				break;
			}
			const elapsedS = Math.floor((performance.now() - startedAt) / 1000);
			const totalS = Math.floor(durationMs / 1000);
			const open = conns.filter((c) => c.ws.readyState === WebSocket.OPEN).length;
			progress(
				`scale ${elapsedS}/${totalS}s ws_open=${open} pings=${pingCount} rpcs=${counterRpcCount}`,
			);
			jsonl.write({
				event: "progress",
				elapsed_s: elapsedS,
				ws_open: open,
				pings: pingCount,
				counter_rpcs: counterRpcCount,
			});
		}
	})();

	await sleep(durationMs);
	shutdown.abort();
	await counterTask.catch(() => undefined);
	await reporter.catch(() => undefined);

	progress("closing websockets...");
	for (const c of conns) {
		if (c.pingTimer) clearInterval(c.pingTimer);
		try {
			c.ws.close();
		} catch {
			// best-effort
		}
	}
	stats.cycles = pingCount + counterRpcCount;
	return stats;
}

interface Verdict {
	pass: boolean;
	notes: string[];
	memory_max_util: number | null;
	memory_p95_util: number | null;
	cpu_max_util: number | null;
	instance_count_max: number | null;
	error_count: number;
}

function quantile(values: number[], q: number): number | null {
	if (values.length === 0) return null;
	const sorted = [...values].sort((a, b) => a - b);
	const idx = Math.min(sorted.length - 1, Math.floor(q * sorted.length));
	return sorted[idx];
}

function computeVerdict(
	mode: Mode,
	metrics: MetricSeries[],
	errorLogs: LogEntry[],
	stats: Stats,
): Verdict {
	const notes: string[] = [];
	let pass = true;

	const memSeries = metrics.filter((m) => m.metric.endsWith("/memory/utilizations"));
	const cpuSeries = metrics.filter((m) => m.metric.endsWith("/cpu/utilizations"));
	const instSeries = metrics.filter((m) => m.metric.endsWith("/instance_count"));

	const memValues = memSeries.flatMap((s) => s.points.map((p) => p.value).filter((v): v is number => v !== null));
	const cpuValues = cpuSeries.flatMap((s) => s.points.map((p) => p.value).filter((v): v is number => v !== null));
	const instValues = instSeries.flatMap((s) => s.points.map((p) => p.value).filter((v): v is number => v !== null));

	const memMax = memValues.length ? Math.max(...memValues) : null;
	const memP95 = quantile(memValues, 0.95);
	const cpuMax = cpuValues.length ? Math.max(...cpuValues) : null;
	const instMax = instValues.length ? Math.max(...instValues) : null;

	if (stats.failures > 0) {
		pass = false;
		notes.push(`${stats.failures} workload assertion failures`);
	}
	if (errorLogs.length > 0) {
		pass = false;
		notes.push(`${errorLogs.length} error log entries`);
	}

	if (mode === "churn" || mode === "steady") {
		if (instMax !== null && instMax > 1) {
			pass = false;
			notes.push(
				`instance_count peaked at ${instMax}; ${mode} expects 1 instance — verdict inconclusive`,
			);
		}
		if (memMax !== null && memMax >= 0.95) {
			pass = false;
			notes.push(`memory utilization reached ${(memMax * 100).toFixed(1)}% (>=95%)`);
		}
	}

	if (mode === "scale") {
		if (instMax === null) {
			pass = false;
			notes.push("no instance_count series found");
		} else if (instMax < 2) {
			pass = false;
			notes.push(`instance_count peaked at ${instMax}; expected >=2`);
		}
	}

	return {
		pass,
		notes,
		memory_max_util: memMax,
		memory_p95_util: memP95,
		cpu_max_util: cpuMax,
		instance_count_max: instMax,
		error_count: errorLogs.length,
	};
}

async function main(): Promise<void> {
	const args = parseArgs(process.argv.slice(2));
	const runId = `soak-${args.mode}-${timestampSlug()}-${randomBytes(2).toString("hex")}`;
	const outPath = `${tmpdir()}/${runId}.jsonl`;
	const jsonl = new JsonlWriter(outPath);

	progress(`run_id=${runId}`);
	progress(`result=${outPath}`);
	progress(`mode=${args.mode}`);

	const token = await getStagingSecretToken();
	const endpoint = `https://${NAMESPACE}:${token}@${ENGINE_HOST}`;
	const client = createClient<typeof registry>(endpoint);

	let revision: string;
	if (args.skipRevisionBump) {
		revision = await getCurrentRevision();
		progress(`reusing revision=${revision} (no bump)`);
	} else {
		const prevRevision = await getCurrentRevision();
		progress(`bumping SOAK_RUN_ID env var (prev revision=${prevRevision})...`);
		revision = await bumpRevision(runId, prevRevision);
		progress(`revision=${revision}`);
	}

	const durationMs =
		(args.durationMin ?? defaultDurationMin(args.mode)) * 60_000;
	jsonl.write({
		event: "start",
		run_id: runId,
		mode: args.mode,
		revision,
		service: SERVICE,
		namespace: NAMESPACE,
		duration_ms: durationMs,
	});

	const startISO = new Date().toISOString();
	let stats: Stats = { cycles: 0, failures: 0 };
	let workloadError: string | undefined;
	try {
		if (args.mode === "churn") {
			stats = await runChurn(client, runId, jsonl, durationMs);
		} else if (args.mode === "steady") {
			stats = await runSteady(client, runId, jsonl);
		} else {
			stats = await runScale(client, runId, jsonl, durationMs);
		}
	} catch (err) {
		workloadError = err instanceof Error ? err.message : String(err);
		jsonl.write({ event: "workload_error", error: workloadError });
		process.stderr.write(`[soak] workload error: ${workloadError}\n`);
	} finally {
		const endISO = new Date().toISOString();
		jsonl.write({ event: "workload_end", stats, error: workloadError });

		progress("querying Cloud Monitoring...");
		const monToken = await gcloudAccessToken();
		const allMetrics: MetricSeries[] = [];
		for (const m of METRIC_TYPES) {
			try {
				const series = await fetchMetricSeries(m, revision, startISO, endISO, monToken);
				for (const s of series) {
					jsonl.write({ event: "metric", ...s });
					allMetrics.push(s);
				}
			} catch (err) {
				jsonl.write({
					event: "metric_fetch_failed",
					metric: m,
					error: err instanceof Error ? err.message : String(err),
				});
				process.stderr.write(
					`[soak] metric fetch failed (${m}): ${err instanceof Error ? err.message : String(err)}\n`,
				);
			}
		}

		progress("querying Cloud Logging for errors...");
		let errorLogs: LogEntry[] = [];
		try {
			errorLogs = await fetchErrorLogs(revision, startISO, endISO);
			for (const e of errorLogs) jsonl.write({ event: "log_error", entry: e });
		} catch (err) {
			jsonl.write({
				event: "log_fetch_failed",
				error: err instanceof Error ? err.message : String(err),
			});
			process.stderr.write(
				`[soak] log fetch failed: ${err instanceof Error ? err.message : String(err)}\n`,
			);
		}

		const verdict = computeVerdict(args.mode, allMetrics, errorLogs, stats);
		jsonl.write({ event: "verdict", ...verdict });
		await jsonl.close();

		progress(
			`complete pass=${verdict.pass} cycles=${stats.cycles} failures=${stats.failures} errors=${verdict.error_count}`,
		);
		if (verdict.memory_max_util !== null) {
			progress(
				`memory max=${(verdict.memory_max_util * 100).toFixed(1)}% p95=${verdict.memory_p95_util !== null ? (verdict.memory_p95_util * 100).toFixed(1) + "%" : "n/a"}`,
			);
		}
		if (verdict.cpu_max_util !== null) {
			progress(`cpu max=${(verdict.cpu_max_util * 100).toFixed(1)}%`);
		}
		if (verdict.instance_count_max !== null) {
			progress(`instance_count max=${verdict.instance_count_max}`);
		}
		for (const note of verdict.notes) progress(`  note: ${note}`);
		progress(`result file: ${outPath}`);
		if (!verdict.pass) process.exitCode = 2;
	}
}

let sigintFired = false;
process.on("SIGINT", () => {
	if (sigintFired) {
		process.stderr.write("[soak] second SIGINT, hard exit\n");
		process.exit(130);
	}
	sigintFired = true;
	process.stderr.write("[soak] SIGINT received, attempting graceful shutdown...\n");
});

main().catch((err) => {
	process.stderr.write(`[soak] fatal: ${err instanceof Error ? err.stack : String(err)}\n`);
	process.exit(1);
});

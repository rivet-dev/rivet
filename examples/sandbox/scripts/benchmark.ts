import { execFile } from "node:child_process";
import { lookup } from "node:dns/promises";
import { performance } from "node:perf_hooks";
import { promisify } from "node:util";
import { type Client, createClient } from "rivetkit/client";
import type { registry } from "../src/index.ts";

const execFileAsync = promisify(execFile);

const DEFAULT_BASELINE_REGION = "us-east-1";
const DEFAULT_BASELINE_SAMPLES = 5;
const DEFAULT_BENCHMARK_SAMPLES = 5;

interface Args {
	endpoint?: string;
	baselineHost?: string;
	baselineSamples: number;
	benchmarkSamples: number;
}

interface BaselineSample {
	statusCode: string;
	pretransferMs: number;
	starttransferMs: number;
	totalMs: number;
	roundTripMs: number;
	responseTransferMs: number;
}

interface BaselineResult {
	hostname: string;
	address: string;
	family: number;
	samples: BaselineSample[];
	medianRoundTripMs: number;
	averageRoundTripMs: number;
}

interface StepResult<T> {
	durationMs: number;
	result: T;
}

interface NoConnectionBenchmarkResult {
	key: string[];
	actorId: string;
	resolveMs: number;
	firstActionMs: number;
	secondActionMs: number;
	firstActionResult: number;
	secondActionResult: number;
}

interface ConnectionBenchmarkResult extends NoConnectionBenchmarkResult {
	connectMs: number;
}

type BenchmarkResult = NoConnectionBenchmarkResult | ConnectionBenchmarkResult;

interface BenchmarkSummary {
	resolveMs: number[];
	firstActionMs: number[];
	secondActionMs: number[];
	connectMs?: number[];
}

interface MeasureBaselineOpts {
	onSample?: (sample: BaselineSample, index: number, total: number) => void;
}

interface SampleProgressOpts {
	onLine?: (line: string) => void;
}

function parseArgs(argv: string[]): Args {
	const args: Args = {
		endpoint: process.env.RIVET_ENDPOINT,
		baselineHost: process.env.RIVET_BASELINE_HOST,
		baselineSamples: Number.parseInt(
			process.env.RIVET_BASELINE_SAMPLES ??
				String(DEFAULT_BASELINE_SAMPLES),
			10,
		),
		benchmarkSamples: Number.parseInt(
			process.env.RIVET_BENCHMARK_SAMPLES ??
				String(DEFAULT_BENCHMARK_SAMPLES),
			10,
		),
	};

	for (let i = 0; i < argv.length; i += 1) {
		const arg = argv[i];
		if (arg === "--endpoint") {
			args.endpoint = argv[i + 1];
			i += 1;
		} else if (arg === "--baseline-host") {
			args.baselineHost = argv[i + 1];
			i += 1;
		} else if (arg === "--baseline-samples") {
			args.baselineSamples = Number.parseInt(argv[i + 1] ?? "", 10);
			i += 1;
		} else if (arg === "--samples") {
			args.benchmarkSamples = Number.parseInt(argv[i + 1] ?? "", 10);
			i += 1;
		}
	}

	if (!args.endpoint) {
		throw new Error(
			"Missing endpoint. Pass --endpoint or set RIVET_ENDPOINT.",
		);
	}

	if (!Number.isFinite(args.baselineSamples) || args.baselineSamples < 1) {
		throw new Error(
			"Invalid baseline sample count. Pass a positive integer.",
		);
	}

	if (!Number.isFinite(args.benchmarkSamples) || args.benchmarkSamples < 1) {
		throw new Error(
			"Invalid benchmark sample count. Pass a positive integer.",
		);
	}

	return args;
}

function deriveRegionalHost(endpoint: string, region: string): string {
	const { hostname } = new URL(endpoint);
	if (hostname.startsWith("api.")) {
		return `api-${region}.${hostname.slice(4)}`;
	}
	return hostname;
}

function median(values: number[]): number {
	const sorted = [...values].sort((a, b) => a - b);
	const middle = Math.floor(sorted.length / 2);
	if (sorted.length % 2 === 0) {
		return (sorted[middle - 1] + sorted[middle]) / 2;
	}
	return sorted[middle];
}

function average(values: number[]): number {
	return values.reduce((sum, value) => sum + value, 0) / values.length;
}

function formatMs(value: number): string {
	return `${value.toFixed(2)} ms`;
}

async function runCurlBaselineSample(
	hostname: string,
	address: string,
): Promise<BaselineSample> {
	const url = `https://${hostname}/`;
	const { stdout } = await execFileAsync("curl", [
		"-I",
		"-sS",
		"-o",
		"/dev/null",
		"--resolve",
		`${hostname}:443:${address}`,
		"-w",
		"%{time_pretransfer} %{time_starttransfer} %{time_total} %{http_code}",
		url,
	]);

	const [pretransferStr, starttransferStr, totalStr, statusCode] = stdout
		.trim()
		.split(/\s+/);
	const pretransferMs = Number.parseFloat(pretransferStr) * 1000;
	const starttransferMs = Number.parseFloat(starttransferStr) * 1000;
	const totalMs = Number.parseFloat(totalStr) * 1000;

	return {
		statusCode,
		pretransferMs,
		starttransferMs,
		totalMs,
		roundTripMs: starttransferMs - pretransferMs,
		responseTransferMs: totalMs - starttransferMs,
	};
}

async function measureBaseline(
	hostname: string,
	samples: number,
	opts?: MeasureBaselineOpts,
): Promise<BaselineResult> {
	const { address, family } = await lookup(hostname);

	// Warm DNS-independent TLS routing before taking the timed samples.
	await runCurlBaselineSample(hostname, address);

	const measurements: BaselineSample[] = [];
	for (let i = 0; i < samples; i += 1) {
		const sample = await runCurlBaselineSample(hostname, address);
		measurements.push(sample);
		opts?.onSample?.(sample, i, samples);
	}

	const roundTrips = measurements.map((sample) => sample.roundTripMs);
	return {
		hostname,
		address,
		family,
		samples: measurements,
		medianRoundTripMs: median(roundTrips),
		averageRoundTripMs: average(roundTrips),
	};
}

async function measureStep<T>(fn: () => Promise<T>): Promise<StepResult<T>> {
	const startedAt = performance.now();
	const result = await fn();
	return {
		durationMs: performance.now() - startedAt,
		result,
	};
}

function createBenchmarkKey(actorName: string, kind: string): string[] {
	return [
		"benchmark",
		actorName,
		kind,
		new Date().toISOString(),
		crypto.randomUUID(),
	];
}

function assertCounterResults(firstValue: number, secondValue: number): void {
	if (firstValue !== 1) {
		throw new Error(
			`Expected first increment to return 1. Received ${firstValue}.`,
		);
	}

	if (secondValue !== 2) {
		throw new Error(
			`Expected second increment to return 2. Received ${secondValue}.`,
		);
	}
}

async function benchmarkWithoutConnection(
	actorName: string,
	client: Client<typeof registry>,
	opts?: SampleProgressOpts,
): Promise<NoConnectionBenchmarkResult> {
	const key = createBenchmarkKey(actorName, "no-connection");
	const handle = (client as any)[actorName].getOrCreate(key);
	opts?.onLine?.(`      key: ${key.join("/")}`);
	opts?.onLine?.("      resolve...");

	const resolve = await measureStep(() => handle.resolve());
	opts?.onLine?.(
		`      resolve: ${formatMs(resolve.durationMs)} -> actorId=${resolve.result}`,
	);
	opts?.onLine?.("      first action...");
	const firstAction = await measureStep(() => handle.increment(1));
	opts?.onLine?.(
		`      first action: ${formatMs(firstAction.durationMs)} -> ${firstAction.result}`,
	);
	opts?.onLine?.("      second action...");
	const secondAction = await measureStep(() => handle.increment(1));
	opts?.onLine?.(
		`      second action: ${formatMs(secondAction.durationMs)} -> ${secondAction.result}`,
	);

	assertCounterResults(firstAction.result, secondAction.result);

	return {
		key,
		actorId: resolve.result,
		resolveMs: resolve.durationMs,
		firstActionMs: firstAction.durationMs,
		secondActionMs: secondAction.durationMs,
		firstActionResult: firstAction.result,
		secondActionResult: secondAction.result,
	};
}

async function benchmarkWithConnection(
	actorName: string,
	client: Client<typeof registry>,
	opts?: SampleProgressOpts,
): Promise<ConnectionBenchmarkResult> {
	const key = createBenchmarkKey(actorName, "connection");
	const handle = (client as any)[actorName].getOrCreate(key);
	opts?.onLine?.(`      key: ${key.join("/")}`);
	opts?.onLine?.("      resolve...");

	const resolve = await measureStep(() => handle.resolve());
	opts?.onLine?.(
		`      resolve: ${formatMs(resolve.durationMs)} -> actorId=${resolve.result}`,
	);
	opts?.onLine?.("      connection handle creation...");

	const connectStartedAt = performance.now();
	const connection = handle.connect();
	const connectMs = performance.now() - connectStartedAt;
	opts?.onLine?.(`      connection handle created: ${formatMs(connectMs)}`);

	try {
		opts?.onLine?.("      first action...");
		const firstAction = await measureStep(() => connection.increment(1));
		opts?.onLine?.(
			`      first action: ${formatMs(firstAction.durationMs)} -> ${firstAction.result}`,
		);
		opts?.onLine?.("      second action...");
		const secondAction = await measureStep(() => connection.increment(1));
		opts?.onLine?.(
			`      second action: ${formatMs(secondAction.durationMs)} -> ${secondAction.result}`,
		);

		assertCounterResults(firstAction.result, secondAction.result);

		return {
			key,
			actorId: resolve.result,
			resolveMs: resolve.durationMs,
			connectMs,
			firstActionMs: firstAction.durationMs,
			secondActionMs: secondAction.durationMs,
			firstActionResult: firstAction.result,
			secondActionResult: secondAction.result,
		};
	} finally {
		await connection.dispose().catch(() => undefined);
	}
}

function printSequence(label: string, result: BenchmarkResult): void {
	console.log(`\n${label}`);
	console.log(`  key: ${result.key.join("/")}`);
	console.log(`  actorId: ${result.actorId}`);
	console.log(`  resolve: ${formatMs(result.resolveMs)}`);
	if ("connectMs" in result) {
		console.log(`  connect handle creation: ${formatMs(result.connectMs)}`);
	}
	console.log(
		`  first action: ${formatMs(result.firstActionMs)} -> ${result.firstActionResult}`,
	);
	console.log(
		`  second action: ${formatMs(result.secondActionMs)} -> ${result.secondActionResult}`,
	);
}

function summarize(results: BenchmarkResult[]): BenchmarkSummary {
	const summary: BenchmarkSummary = {
		resolveMs: results.map((result) => result.resolveMs),
		firstActionMs: results.map((result) => result.firstActionMs),
		secondActionMs: results.map((result) => result.secondActionMs),
	};

	if (results.every((result) => "connectMs" in result)) {
		summary.connectMs = results.map(
			(result) => (result as ConnectionBenchmarkResult).connectMs,
		);
	}

	return summary;
}

function printSummary(label: string, summary: BenchmarkSummary): void {
	console.log(`\n  ${label} summary`);
	console.log(
		`    resolve median: ${formatMs(median(summary.resolveMs))}, average: ${formatMs(average(summary.resolveMs))}`,
	);
	if (summary.connectMs) {
		console.log(
			`    connect median: ${formatMs(median(summary.connectMs))}, average: ${formatMs(average(summary.connectMs))}`,
		);
	}
	console.log(
		`    first action median: ${formatMs(median(summary.firstActionMs))}, average: ${formatMs(average(summary.firstActionMs))}`,
	);
	console.log(
		`    second action median: ${formatMs(median(summary.secondActionMs))}, average: ${formatMs(average(summary.secondActionMs))}`,
	);
}

async function runBenchmarkSeries<T extends BenchmarkResult>(
	label: string,
	totalSamples: number,
	runSample: (opts: SampleProgressOpts) => Promise<T>,
): Promise<T[]> {
	console.log(`\n  ${label}`);

	const results: T[] = [];
	for (let i = 0; i < totalSamples; i += 1) {
		console.log(`    sample ${i + 1}/${totalSamples}`);
		const result = await runSample({
			onLine: (line) => {
				console.log(line);
			},
		});
		results.push(result);
	}

	return results;
}

async function main(): Promise<void> {
	const args = parseArgs(process.argv.slice(2));
	const baselineHost =
		args.baselineHost ??
		deriveRegionalHost(args.endpoint!, DEFAULT_BASELINE_REGION);

	console.log("Actor benchmark");
	console.log(`Endpoint: ${new URL(args.endpoint!).origin}`);
	console.log(`Baseline host: ${baselineHost}`);
	console.log("Metadata lookup disabled: true");
	console.log(`Benchmark samples per flow: ${args.benchmarkSamples}`);

	console.log("\nRegional baseline");
	console.log("  warming baseline connection...");
	const baseline = await measureBaseline(baselineHost, args.baselineSamples, {
		onSample: (sample, index, total) => {
			console.log(
				`  sample ${index + 1}/${total}: rtt=${formatMs(sample.roundTripMs)} pretransfer=${formatMs(sample.pretransferMs)} total=${formatMs(sample.totalMs)} status=${sample.statusCode}`,
			);
		},
	});

	console.log(
		`  host: ${baseline.hostname} (${baseline.address}, IPv${baseline.family})`,
	);
	console.log(
		`  median request RTT after pretransfer: ${formatMs(baseline.medianRoundTripMs)}`,
	);
	console.log(
		`  average request RTT after pretransfer: ${formatMs(baseline.averageRoundTripMs)}`,
	);
	console.log(
		`  raw samples: ${baseline.samples.map((sample) => formatMs(sample.roundTripMs)).join(", ")}`,
	);
	console.log(
		`  sample status codes: ${baseline.samples.map((sample) => sample.statusCode).join(", ")}`,
	);

	const client = createClient<typeof registry>({
		endpoint: args.endpoint,
		disableMetadataLookup: true,
	});

	const actors = ["testCounter", "testCounterSqlite", "testSqliteLoad"];

	try {
		for (const actorName of actors) {
			console.log(`\n${"=".repeat(60)}`);
			console.log(`Actor: ${actorName}`);
			console.log("=".repeat(60));

			// const noConnectionResults = await runBenchmarkSeries(
			// 	"Without connection",
			// 	args.benchmarkSamples,
			// 	(opts) => benchmarkWithoutConnection(actorName, client, opts),
			// );
			// printSummary(
			// 	"Without connection",
			// 	summarize(noConnectionResults),
			// );

			const withConnectionResults = await runBenchmarkSeries(
				"With connection",
				args.benchmarkSamples,
				(opts) => benchmarkWithConnection(actorName, client, opts),
			);
			printSummary("With connection", summarize(withConnectionResults));
		}
	} finally {
		await client.dispose().catch(() => undefined);
	}
}

main().catch((error: unknown) => {
	console.error(error);
	process.exit(1);
});

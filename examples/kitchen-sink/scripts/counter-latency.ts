// Counter-latency mini load test.
//
// Two modes:
//
// rtt        Every --interval ms, spawn a background worker that:
//              1. Generates a new key
//              2. handle = client.counter.getOrCreate([key])
//              3. connection = handle.connect({ skipReadyWait: true })
//              4. measures connect (via no-op ws roundtrip), first increment, second increment
//            Workers run concurrently by default; the interval does not wait on prior workers.
//            Set SERIAL=1 to force serial execution.
//
// concurrent Ramps up to --concurrency persistent connections (one new connection
//            every --interval ms). Each connection holds open and increments once
//            every 10s. On disconnect, immediately reconnects, logging connect
//            time again. Color codes by elapsed ms: < 800 green, 800-1500 orange, > 1500 red.
//
// Usage:
//   tsx scripts/counter-latency.ts --mode <rtt|concurrent> --interval <ms> [--concurrency N] <endpoint>
//
//   --mode         "rtt" or "concurrent" (required)
//   --interval     gap in ms between worker starts (rtt) or ramp-up between connections (concurrent)
//   --concurrency  number of persistent connections (concurrent mode only)
//
//   BATCHES        total workers spawned before exit in rtt mode (default infinite)
//   SERIAL         "1" / "true" to await each worker before the next in rtt mode (default off)
//
// Examples:
//   tsx scripts/counter-latency.ts --mode rtt --interval 1000 \
//     "http://default:TOKEN@34.110.160.16:80"
//
//   tsx scripts/counter-latency.ts --mode concurrent --interval 50 --concurrency 100 \
//     "http://default:TOKEN@34.110.160.16:80"

import { createClient } from "rivetkit/client";
import type { registry } from "../src/index.ts";

interface Args {
	mode: "rtt" | "concurrent";
	interval: number;
	concurrency?: number;
	endpoint: string;
}

function parseArgs(argv: string[]): Args {
	let mode: string | undefined;
	let interval: number | undefined;
	let concurrency: number | undefined;
	let endpoint: string | undefined;

	for (let i = 0; i < argv.length; i++) {
		const arg = argv[i];
		if (arg === "--mode") {
			mode = argv[++i];
		} else if (arg === "--interval") {
			interval = Number(argv[++i]);
		} else if (arg === "--concurrency") {
			concurrency = Number(argv[++i]);
		} else if (arg === "--help" || arg === "-h") {
			usage();
			process.exit(0);
		} else if (arg.startsWith("--")) {
			console.error(`unknown arg: ${arg}`);
			usage();
			process.exit(1);
		} else if (!endpoint) {
			endpoint = arg;
		} else {
			console.error(`unexpected positional arg: ${arg}`);
			usage();
			process.exit(1);
		}
	}

	if (!mode) {
		console.error("--mode is required");
		usage();
		process.exit(1);
	}
	if (mode !== "rtt" && mode !== "concurrent") {
		console.error(`--mode must be "rtt" or "concurrent", got: ${mode}`);
		process.exit(1);
	}
	if (interval === undefined || !Number.isFinite(interval) || interval < 0) {
		console.error("--interval is required (ms, >= 0)");
		process.exit(1);
	}
	if (mode === "concurrent") {
		if (
			concurrency === undefined ||
			!Number.isFinite(concurrency) ||
			concurrency < 1
		) {
			console.error("--concurrency is required for mode=concurrent (>= 1)");
			process.exit(1);
		}
	}
	if (!endpoint) {
		console.error("endpoint is required");
		usage();
		process.exit(1);
	}

	return { mode, interval, concurrency, endpoint };
}

function usage(): void {
	console.error(
		"usage: tsx scripts/counter-latency.ts --mode <rtt|concurrent> --interval <ms> [--concurrency N] <endpoint>",
	);
}

const ARGS = parseArgs(process.argv.slice(2));
const BATCHES = Number(process.env.BATCHES ?? "0"); // 0 = infinite (rtt mode)
const SERIAL = ((v) => v === "1" || v === "true")(process.env.SERIAL ?? "");
const INCREMENT_INTERVAL_MS = 10_000;

const ANSI = {
	reset: "\x1b[0m",
	green: "\x1b[38;2;0;255;0m",
	red: "\x1b[38;2;255;0;0m",
	dim: "\x1b[2m",
	bold: "\x1b[1m",
};

const COLOR_MIN_MS = 800;
const COLOR_MAX_MS = 2000;

function gradientColor(ms: number): string {
	const clamped = Math.max(COLOR_MIN_MS, Math.min(COLOR_MAX_MS, ms));
	const t = (clamped - COLOR_MIN_MS) / (COLOR_MAX_MS - COLOR_MIN_MS);
	let r: number;
	let g: number;
	if (t <= 0.5) {
		r = Math.round(t * 2 * 255);
		g = 255;
	} else {
		r = 255;
		g = Math.round((1 - (t - 0.5) * 2) * 255);
	}
	return `\x1b[38;2;${r};${g};0m`;
}

function colorMs(ms: number): string {
	const fixed = ms.toFixed(0).padStart(5);
	return `${gradientColor(ms)}${fixed}ms${ANSI.reset}`;
}

function pad(s: string, n: number): string {
	return s.length >= n ? s : s + " ".repeat(n - s.length);
}

function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

const client = createClient<typeof registry>(ARGS.endpoint);

interface Sample {
	worker: number;
	key: string;
	connectMs: number;
	firstMs: number;
	secondMs: number;
	totalMs: number;
	error?: string;
}

async function runRttWorker(worker: number): Promise<Sample> {
	const key = `cl-${worker}-${Date.now().toString(36)}`;

	const t0 = performance.now();
	try {
		const handle = client.counter.getOrCreate([key]);
		const connection = handle.connect({ skipReadyWait: true });

		// Probe ws open with a no-op so connect time is measured separately
		// from the first user-visible action.
		await connection.noop();
		const tConnect = performance.now();

		await connection.increment(1);
		const tFirst = performance.now();

		await connection.increment(1);
		const tSecond = performance.now();

		// Best-effort cleanup; do not block measurements.
		void connection.dispose().catch(() => {});

		return {
			worker,
			key,
			connectMs: tConnect - t0,
			firstMs: tFirst - tConnect,
			secondMs: tSecond - tFirst,
			totalMs: tSecond - t0,
		};
	} catch (err) {
		const tEnd = performance.now();
		return {
			worker,
			key,
			connectMs: 0,
			firstMs: 0,
			secondMs: 0,
			totalMs: tEnd - t0,
			error: err instanceof Error ? err.message : String(err),
		};
	}
}

function printRttSample(s: Sample): void {
	const ts = new Date().toISOString();
	const prefix = `${ANSI.dim}${ts}${ANSI.reset} [w=${String(s.worker).padStart(5)}]`;
	if (s.error) {
		console.log(
			`${prefix} ${pad(s.key, 32)} ${ANSI.red}ERROR ${s.error}${ANSI.reset} (${colorMs(s.totalMs)})`,
		);
		return;
	}
	console.log(
		`${prefix} ${pad(s.key, 32)} connect=${colorMs(s.connectMs)} first=${colorMs(s.firstMs)} second=${colorMs(s.secondMs)} total=${colorMs(s.totalMs)}`,
	);
}

function logConnect(worker: number, key: string, connectMs: number): void {
	const ts = new Date().toISOString();
	const prefix = `${ANSI.dim}${ts}${ANSI.reset} [w=${String(worker).padStart(5)}]`;
	console.log(`${prefix} ${pad(key, 32)} connect=${colorMs(connectMs)}`);
}

function logIncrement(worker: number, key: string, incrementMs: number): void {
	const ts = new Date().toISOString();
	const prefix = `${ANSI.dim}${ts}${ANSI.reset} [w=${String(worker).padStart(5)}]`;
	console.log(`${prefix} ${pad(key, 32)} increment=${colorMs(incrementMs)}`);
}

function logDisconnect(worker: number, key: string, reason: string): void {
	const ts = new Date().toISOString();
	const prefix = `${ANSI.dim}${ts}${ANSI.reset} [w=${String(worker).padStart(5)}]`;
	console.log(
		`${prefix} ${pad(key, 32)} ${ANSI.red}DISCONNECT ${reason}${ANSI.reset}`,
	);
}

function logConnectError(
	worker: number,
	key: string,
	elapsedMs: number,
	reason: string,
): void {
	const ts = new Date().toISOString();
	const prefix = `${ANSI.dim}${ts}${ANSI.reset} [w=${String(worker).padStart(5)}]`;
	console.log(
		`${prefix} ${pad(key, 32)} ${ANSI.red}CONNECT-ERROR ${reason}${ANSI.reset} (${colorMs(elapsedMs)})`,
	);
}

async function runConcurrentWorker(worker: number): Promise<void> {
	const key = `cl-c-${worker}-${Date.now().toString(36)}`;

	while (true) {
		const t0 = performance.now();
		let connection: ReturnType<
			ReturnType<typeof client.counter.getOrCreate>["connect"]
		> | null = null;
		try {
			const handle = client.counter.getOrCreate([key]);
			connection = handle.connect({ skipReadyWait: true });

			// Probe ws open with a no-op to measure connect time.
			await connection.noop();
			const connectMs = performance.now() - t0;
			logConnect(worker, key, connectMs);

			// Hold open and increment every INCREMENT_INTERVAL_MS.
			while (true) {
				await sleep(INCREMENT_INTERVAL_MS);
				const incStart = performance.now();
				try {
					await connection.increment(1);
					const incMs = performance.now() - incStart;
					logIncrement(worker, key, incMs);
				} catch (err) {
					logDisconnect(
						worker,
						key,
						err instanceof Error ? err.message : String(err),
					);
					break;
				}
			}
		} catch (err) {
			const elapsed = performance.now() - t0;
			logConnectError(
				worker,
				key,
				elapsed,
				err instanceof Error ? err.message : String(err),
			);
		} finally {
			if (connection) {
				void connection.dispose().catch(() => {});
			}
		}
	}
}

async function runRttMode(): Promise<void> {
	let workerId = 0;
	const inflight: Promise<void>[] = [];

	while (BATCHES === 0 || workerId < BATCHES) {
		workerId++;
		const id = workerId;
		if (SERIAL) {
			const sample = await runRttWorker(id);
			printRttSample(sample);
		} else {
			inflight.push(runRttWorker(id).then((s) => printRttSample(s)));
		}
		if (BATCHES === 0 || workerId < BATCHES) {
			await sleep(ARGS.interval);
		}
	}

	await Promise.all(inflight);
}

async function runConcurrentMode(): Promise<void> {
	const concurrency = ARGS.concurrency!;
	const workers: Promise<void>[] = [];
	for (let i = 0; i < concurrency; i++) {
		const id = i + 1;
		workers.push(runConcurrentWorker(id));
		if (i < concurrency - 1) {
			await sleep(ARGS.interval);
		}
	}
	await Promise.all(workers);
}

async function main(): Promise<void> {
	const url = new URL(ARGS.endpoint);
	const header = `${ANSI.bold}counter-latency${ANSI.reset} endpoint=${url.protocol}//${url.host} ns=${decodeURIComponent(url.username)} mode=${ARGS.mode} interval=${ARGS.interval}ms`;
	if (ARGS.mode === "rtt") {
		console.log(`${header} batches=${BATCHES || "∞"} serial=${SERIAL}`);
	} else {
		console.log(
			`${header} concurrency=${ARGS.concurrency} increment-every=${INCREMENT_INTERVAL_MS}ms`,
		);
	}
	console.log(
		`${ANSI.dim}gradient: ${gradientColor(COLOR_MIN_MS)}${COLOR_MIN_MS}ms${ANSI.reset}${ANSI.dim} → ${gradientColor((COLOR_MIN_MS + COLOR_MAX_MS) / 2)}${(COLOR_MIN_MS + COLOR_MAX_MS) / 2}ms${ANSI.reset}${ANSI.dim} → ${gradientColor(COLOR_MAX_MS)}${COLOR_MAX_MS}ms${ANSI.reset}`,
	);
	console.log();

	if (ARGS.mode === "rtt") {
		await runRttMode();
	} else {
		await runConcurrentMode();
	}
}

main().catch((err) => {
	console.error("fatal:", err);
	process.exit(1);
});

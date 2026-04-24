#!/usr/bin/env -S pnpm exec tsx

import { randomBytes } from "node:crypto";
import { createClient } from "rivetkit/client";
import type { registry } from "../src/index.ts";

type WorkloadMode =
	| "balanced"
	| "hot"
	| "transactions"
	| "payloads"
	| "edge"
	| "fragmentation"
	| "schema"
	| "index"
	| "relational"
	| "constraints"
	| "savepoints"
	| "pragma"
	| "prepared"
	| "growth"
	| "readwrite"
	| "truncate"
	| "boundary-keys"
	| "shadow"
	| "actual-nul"
	| "nasty-script"
	| "nasty"
	| "kitchen-sink";

interface Args {
	endpoint: string;
	seed: string;
	iterations: number;
	concurrency: number;
	actorCount: number;
	mode: WorkloadMode;
	sleepEvery: number;
	opsPerPhase: number;
	keySpace: number;
	maxPayloadBytes: number;
	growthTargetBytes: number;
	wakeDelayMs: number;
	localEnvoyWarmupMs: number;
	reset: boolean;
	disableMetadataLookup: boolean;
	startLocalEnvoy: boolean;
}

interface ValidationSummary {
	totalEvents: number;
	activeRows: number;
	expectedRows: number;
	missingRows: number;
	extraRows: number;
	mismatchedRows: number;
	duplicateKeys: number;
	actualVersionSum: number;
	expectedVersionSum: number;
	actualPayloadChecksumSum: number;
	expectedPayloadChecksumSum: number;
	accountCount: number;
	accountBalanceSum: number;
	expectedAccountBalanceSum: number;
	accountBalanceMismatch: number;
	integrityCheck: string;
	quickCheck: string;
	edgeRows: number;
	edgeExpectedRows: number;
	edgeMismatches: number;
	indexRows: number;
	indexMismatches: number;
	relationalOrders: number;
	relationalMismatches: number;
	constraintAttempts: number;
	constraintLeaks: number;
	savepointRows: number;
	savepointMismatches: number;
	idempotentOps: number;
	idempotentMismatches: number;
	schemaObjects: number;
	schemaMissingObjects: number;
	probeRows: number;
	probeMismatches: number;
	preparedRows: number;
	preparedMismatches: number;
	shadowRows: number;
	shadowMismatches: number;
}

interface PhaseResult {
	seed: string;
	phase: number;
	mode: WorkloadMode;
	iterations: number;
	ops: Record<string, number>;
	validation: ValidationSummary;
}

interface ActorStats {
	key: string[];
	phases: number;
	events: number;
	activeRows: number;
}

const DEFAULT_MODE: WorkloadMode = "balanced";
const VALID_MODES = new Set<WorkloadMode>([
	"balanced",
	"hot",
	"transactions",
	"payloads",
	"edge",
	"fragmentation",
	"schema",
	"index",
	"relational",
	"constraints",
	"savepoints",
	"pragma",
	"prepared",
	"growth",
	"readwrite",
	"truncate",
	"boundary-keys",
	"shadow",
	"actual-nul",
	"nasty-script",
	"nasty",
	"kitchen-sink",
]);

function usage(): never {
	console.error(`Usage:
  pnpm --filter kitchen-sink exec tsx scripts/db-fuzz.ts --endpoint <url> [options]

Options:
  --seed <seed>                 Repro seed. Defaults to a generated seed.
  --iterations <n>              Phases per actor. Default: 5.
  --concurrency <n>             Concurrent phase drivers. Default: 2.
  --actor-count <n>             Fuzzer actor instances. Default: 2.
  --mode <mode>                 balanced, hot, transactions, payloads, edge, fragmentation, schema, index, relational, constraints, savepoints, pragma, prepared, growth, readwrite, truncate, boundary-keys, shadow, actual-nul, nasty-script, nasty, kitchen-sink. Default: balanced.
  --sleep-every <n>             Sleep/wake after every N phases per actor. Default: 0.
  --ops-per-phase <n>           Raw SQLite operations per phase. Default: 50.
  --key-space <n>               Number of item keys per actor. Default: 64.
  --max-payload-bytes <n>       Largest payload written by item ops. Default: 8192.
  --growth-target-bytes <n>     Total bytes for growth mode per phase. Default: 1048576.
  --wake-delay-ms <n>           Delay after c.sleep() before next validation. Default: 1000.
  --local-envoy-warmup-ms <n>   Delay after --start-local-envoy before calls. Default: 3000.
  --no-reset                    Reuse existing actor databases instead of clearing them.
  --disable-metadata-lookup     Treat --endpoint as the direct Rivet engine endpoint.
  --start-local-envoy           Start this registry's local envoy before driving it.

Environment:
  RIVET_ENDPOINT, DB_FUZZ_SEED, DB_FUZZ_ITERATIONS, DB_FUZZ_CONCURRENCY,
  DB_FUZZ_ACTOR_COUNT, DB_FUZZ_MODE, DB_FUZZ_SLEEP_EVERY,
  DB_FUZZ_OPS_PER_PHASE, DB_FUZZ_KEY_SPACE, DB_FUZZ_MAX_PAYLOAD_BYTES,
  DB_FUZZ_GROWTH_TARGET_BYTES, DB_FUZZ_WAKE_DELAY_MS,
  DB_FUZZ_LOCAL_ENVOY_WARMUP_MS`);
	process.exit(1);
}

function readFlag(argv: string[], name: string): string | undefined {
	const prefix = `${name}=`;
	const inline = argv.find((arg) => arg.startsWith(prefix));
	if (inline) return inline.slice(prefix.length);
	const index = argv.indexOf(name);
	if (index >= 0) return argv[index + 1];
	return undefined;
}

function readNumber(
	argv: string[],
	flag: string,
	envName: string,
	defaultValue: number,
): number {
	const raw = readFlag(argv, flag) ?? process.env[envName];
	if (raw === undefined) return defaultValue;
	const value = Number.parseInt(raw, 10);
	if (!Number.isFinite(value) || value < 0) {
		throw new Error(`invalid ${flag}: ${raw}`);
	}
	return value;
}

function parseArgs(argv: string[]): Args {
	if (argv.includes("--help") || argv.includes("-h")) usage();

	const endpoint = readFlag(argv, "--endpoint") ?? process.env.RIVET_ENDPOINT;
	if (!endpoint) {
		throw new Error("missing endpoint. Pass --endpoint or set RIVET_ENDPOINT.");
	}

	const modeRaw = readFlag(argv, "--mode") ?? process.env.DB_FUZZ_MODE ?? DEFAULT_MODE;
	if (!VALID_MODES.has(modeRaw as WorkloadMode)) {
		throw new Error(`invalid --mode: ${modeRaw}`);
	}

	const seed =
		readFlag(argv, "--seed") ??
		process.env.DB_FUZZ_SEED ??
		`db-fuzz-${Date.now()}-${randomBytes(3).toString("hex")}`;

	const args: Args = {
		endpoint,
		seed,
		iterations: readNumber(argv, "--iterations", "DB_FUZZ_ITERATIONS", 5),
		concurrency: readNumber(argv, "--concurrency", "DB_FUZZ_CONCURRENCY", 2),
		actorCount: readNumber(argv, "--actor-count", "DB_FUZZ_ACTOR_COUNT", 2),
		mode: modeRaw as WorkloadMode,
		sleepEvery: readNumber(argv, "--sleep-every", "DB_FUZZ_SLEEP_EVERY", 0),
		opsPerPhase: readNumber(argv, "--ops-per-phase", "DB_FUZZ_OPS_PER_PHASE", 50),
		keySpace: readNumber(argv, "--key-space", "DB_FUZZ_KEY_SPACE", 64),
		maxPayloadBytes: readNumber(
			argv,
			"--max-payload-bytes",
			"DB_FUZZ_MAX_PAYLOAD_BYTES",
			8192,
		),
		growthTargetBytes: readNumber(
			argv,
			"--growth-target-bytes",
			"DB_FUZZ_GROWTH_TARGET_BYTES",
			1024 * 1024,
		),
		wakeDelayMs: readNumber(argv, "--wake-delay-ms", "DB_FUZZ_WAKE_DELAY_MS", 1000),
		localEnvoyWarmupMs: readNumber(
			argv,
			"--local-envoy-warmup-ms",
			"DB_FUZZ_LOCAL_ENVOY_WARMUP_MS",
			3000,
		),
		reset: !argv.includes("--no-reset"),
		disableMetadataLookup: argv.includes("--disable-metadata-lookup"),
		startLocalEnvoy: argv.includes("--start-local-envoy"),
	};

	for (const [name, value] of [
		["--iterations", args.iterations],
		["--concurrency", args.concurrency],
		["--actor-count", args.actorCount],
		["--ops-per-phase", args.opsPerPhase],
		["--key-space", args.keySpace],
		["--max-payload-bytes", args.maxPayloadBytes],
		["--growth-target-bytes", args.growthTargetBytes],
	] as const) {
		if (value < 1) throw new Error(`${name} must be >= 1`);
	}

	return args;
}

function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

function assertValidation(
	validation: ValidationSummary,
	context: Record<string, unknown>,
): void {
	const failed =
		validation.integrityCheck !== "ok" ||
		validation.quickCheck !== "ok" ||
		validation.activeRows !== validation.expectedRows ||
		validation.missingRows !== 0 ||
		validation.extraRows !== 0 ||
		validation.mismatchedRows !== 0 ||
		validation.duplicateKeys !== 0 ||
		validation.actualVersionSum !== validation.expectedVersionSum ||
		validation.actualPayloadChecksumSum !== validation.expectedPayloadChecksumSum ||
		validation.accountCount !== 8 ||
		validation.accountBalanceSum !== validation.expectedAccountBalanceSum ||
		validation.accountBalanceMismatch !== 0 ||
		validation.edgeRows !== validation.edgeExpectedRows ||
		validation.edgeMismatches !== 0 ||
		validation.indexMismatches !== 0 ||
		validation.relationalMismatches !== 0 ||
		validation.constraintLeaks !== 0 ||
		validation.savepointMismatches !== 0 ||
		validation.idempotentMismatches !== 0 ||
		validation.schemaMissingObjects !== 0 ||
		validation.probeMismatches !== 0 ||
		validation.preparedMismatches !== 0 ||
		validation.shadowMismatches !== 0;

	if (failed) {
		throw new Error(
			`invariant failed ${JSON.stringify({ ...context, validation })}`,
		);
	}
}

function formatOps(ops: Record<string, number>): string {
	return Object.entries(ops)
		.sort(([a], [b]) => a.localeCompare(b))
		.map(([key, value]) => `${key}=${value}`)
		.join(" ");
}

async function runPhase(
	handle: ReturnType<
		ReturnType<typeof createClient<typeof registry>>["rawSqliteFuzzer"]["getOrCreate"]
	>,
	args: Args,
	actorIndex: number,
	key: string[],
	phase: number,
): Promise<PhaseResult> {
	try {
		const result = await handle.runPhase({
			seed: `${args.seed}:actor:${actorIndex}`,
			phase,
			iterations: args.opsPerPhase,
			mode: args.mode,
			maxPayloadBytes: args.maxPayloadBytes,
			growthTargetBytes: args.growthTargetBytes,
			keySpace: args.keySpace,
		});
		assertValidation(result.validation, {
			seed: args.seed,
			actorKey: key,
			phase,
			mode: args.mode,
		});
		return result;
	} catch (err) {
		const message = err instanceof Error ? err.stack ?? err.message : String(err);
		throw new Error(
			`phase failed seed=${args.seed} actorKey=${key.join("/")} phase=${phase} mode=${args.mode}: ${message}`,
		);
	}
}

async function main(): Promise<void> {
	const args = parseArgs(process.argv.slice(2));

	if (args.startLocalEnvoy) {
		await import("../../../rivetkit-typescript/packages/sql-loader/dist/register.js");
		const { registry } = await import("../src/index.ts");
		registry.start();
		await sleep(args.localEnvoyWarmupMs);
	}

	const client = createClient<typeof registry>({
		endpoint: args.endpoint,
		disableMetadataLookup: args.disableMetadataLookup,
	});
	const actorKeys = Array.from({ length: args.actorCount }, (_, i) => [
		"db-fuzz",
		args.seed,
		args.mode,
		String(i),
	]);
	const handles = actorKeys.map((key) => client.rawSqliteFuzzer.getOrCreate(key));
	const stats = new Map<number, ActorStats>();

	console.log(
		[
			`seed=${args.seed}`,
			`mode=${args.mode}`,
			`actors=${args.actorCount}`,
			`iterations=${args.iterations}`,
			`ops_per_phase=${args.opsPerPhase}`,
			`concurrency=${args.concurrency}`,
			`sleep_every=${args.sleepEvery}`,
			`endpoint=${args.endpoint}`,
			`disable_metadata_lookup=${args.disableMetadataLookup}`,
			`start_local_envoy=${args.startLocalEnvoy}`,
		].join(" "),
	);
	console.log(`actor_keys=${actorKeys.map((key) => key.join("/")).join(",")}`);

	try {
		if (args.reset) {
			console.log("resetting actor databases...");
			await Promise.all(
				handles.map(async (handle, actorIndex) => {
					const validation = await handle.reset();
					assertValidation(validation, {
						seed: args.seed,
						actorKey: actorKeys[actorIndex],
						phase: "reset",
					});
				}),
			);
		}

		const totalJobs = args.actorCount * args.iterations;
		let nextJob = 0;

		async function worker(workerId: number): Promise<void> {
			for (;;) {
				const job = nextJob;
				nextJob += 1;
				if (job >= totalJobs) return;

				const actorIndex = job % args.actorCount;
				const phase = Math.floor(job / args.actorCount);
				const key = actorKeys[actorIndex]!;
				const handle = handles[actorIndex]!;
				const startedAt = performance.now();
				const result = await runPhase(handle, args, actorIndex, key, phase);
				const durationMs = performance.now() - startedAt;

				stats.set(actorIndex, {
					key,
					phases: (stats.get(actorIndex)?.phases ?? 0) + 1,
					events: result.validation.totalEvents,
					activeRows: result.validation.activeRows,
				});

				console.log(
					`phase ok worker=${workerId} actor=${actorIndex} phase=${phase} events=${result.validation.totalEvents} rows=${result.validation.activeRows} edge=${result.validation.edgeRows} index=${result.validation.indexRows} orders=${result.validation.relationalOrders} probes=${result.validation.probeRows} prepared=${result.validation.preparedRows} shadow=${result.validation.shadowRows} ms=${durationMs.toFixed(1)} ${formatOps(result.ops)}`,
				);

				if (args.sleepEvery > 0 && (phase + 1) % args.sleepEvery === 0) {
					console.log(`sleep actor=${actorIndex} phase=${phase}`);
					await handle.goToSleep();
					await sleep(args.wakeDelayMs);
					const reacquired = client.rawSqliteFuzzer.getOrCreate(key);
					const validation = await reacquired.validate();
					assertValidation(validation, {
						seed: args.seed,
						actorKey: key,
						phase,
						afterSleep: true,
					});
					console.log(
						`wake ok actor=${actorIndex} phase=${phase} events=${validation.totalEvents} rows=${validation.activeRows} edge=${validation.edgeRows} index=${validation.indexRows} orders=${validation.relationalOrders} probes=${validation.probeRows} prepared=${validation.preparedRows} shadow=${validation.shadowRows}`,
					);
				}
			}
		}

		await Promise.all(
			Array.from({ length: args.concurrency }, (_, i) => worker(i)),
		);

		for (let actorIndex = 0; actorIndex < handles.length; actorIndex += 1) {
			const reacquired = client.rawSqliteFuzzer.getOrCreate(actorKeys[actorIndex]!);
			const validation = await reacquired.validate();
			assertValidation(validation, {
				seed: args.seed,
				actorKey: actorKeys[actorIndex],
				phase: "final",
			});
			stats.set(actorIndex, {
				key: actorKeys[actorIndex]!,
				phases: stats.get(actorIndex)?.phases ?? 0,
				events: validation.totalEvents,
				activeRows: validation.activeRows,
			});
		}

		console.log("summary:");
		for (const [actorIndex, actorStats] of [...stats.entries()].sort(
			([a], [b]) => a - b,
		)) {
			console.log(
				`  actor=${actorIndex} key=${actorStats.key.join("/")} phases=${actorStats.phases} events=${actorStats.events} rows=${actorStats.activeRows}`,
			);
		}
	} finally {
		await client.dispose().catch(() => undefined);
	}
}

main()
	.then(() => {
		process.exit(0);
	})
	.catch((err: unknown) => {
		const message = err instanceof Error ? err.stack ?? err.message : String(err);
		console.error(message);
		process.exit(1);
	});

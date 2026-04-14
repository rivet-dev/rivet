// Global smoke test: exercise cross-region actor key semantics against the
// Rivet staging engine. Runs three scenarios in serial, each of which races N
// parallel operations against the same fresh key for ROUNDS rounds.
//
// Scenario 1 - create race:
//   Fire N parallel `counter.create(key)` calls split across regions and
//   assert that exactly one call wins while the rest fail with
//   `actor.duplicate_key`.
//
// Scenario 2 - getOrCreate race:
//   Fire N parallel `counter.getOrCreate(key).resolve()` calls across regions
//   and assert that every call succeeds AND all N return the same actor ID.
//   Different actor IDs would indicate the engine created multiple actors for
//   the same key.
//
// Scenario 3 - cross-region visibility:
//   Create the actor in one region (the "creator"), read back the actor ID
//   from the creator's response, then from every other region call
//   `counter.get(key).resolve()` and assert it returns the exact same actor
//   ID.
//
// Usage:
//   pnpm --filter sandbox exec tsx scripts/global-smoke-test.ts
//
// Environment:
//   RIVET_ENDPOINT_US_EAST    override the US East endpoint
//   RIVET_ENDPOINT_EU_CENTRAL override the EU Central endpoint
//   PARALLELISM               parallel ops per round (default 10)
//   ROUNDS                    rounds per scenario (default 50)
//   ATTEMPT_TIMEOUT_MS        per-op timeout in ms (default 5000)
//   JITTER_MS                 per-op start jitter window in ms (default 300,
//                             set to 0 to fire every op in lockstep)

import { ActorError, createClient } from "rivetkit/client";
import type { registry } from "../src/index.ts";

// STAGING:
// const DEFAULT_US_EAST =
// 	"https://kitchen-sink-gv34-production-d4ob:pk_bOhFqJQb0AnFXTqdSVcRd8sk3v1YWvrP9o4GV8O1CzxBDUM1QhxcDx1YO0sW3vQz@api-us-east-1.staging.rivet.dev";
// const DEFAULT_EU_CENTRAL =
// 	"https://kitchen-sink-gv34-production-d4ob:pk_bOhFqJQb0AnFXTqdSVcRd8sk3v1YWvrP9o4GV8O1CzxBDUM1QhxcDx1YO0sW3vQz@api-eu-central-1.staging.rivet.dev";

// PROD:
const DEFAULT_US_EAST =
	"https://kitchen-sink-29a8-test-23-pfwh:pk_jq6e1pTRLWYdmJXUKQ9pGcMk6XU8ZghEmpNLmU2MeR0cVr1JGpZo7ZX9RSylyglw@api-us-east-1.rivet.dev";
const DEFAULT_EU_CENTRAL =
	"https://kitchen-sink-29a8-test-23-pfwh:pk_jq6e1pTRLWYdmJXUKQ9pGcMk6XU8ZghEmpNLmU2MeR0cVr1JGpZo7ZX9RSylyglw@api-eu-central-1.rivet.dev";

const REGIONS = [
	{
		label: "us-east-1",
		endpoint: process.env.RIVET_ENDPOINT_US_EAST ?? DEFAULT_US_EAST,
	},
	{
		label: "eu-central-1",
		endpoint: process.env.RIVET_ENDPOINT_EU_CENTRAL ?? DEFAULT_EU_CENTRAL,
	},
] as const;

const PARALLELISM = Number(process.env.PARALLELISM ?? "1");
const ROUNDS = Number(process.env.ROUNDS ?? "100");
const ATTEMPT_TIMEOUT_MS = Number(process.env.ATTEMPT_TIMEOUT_MS ?? "5000");
// Per-attempt start-time jitter. Each parallel op sleeps for a uniform random
// delay in [0, JITTER_MS) before firing, so races are spread over a window
// instead of all landing in the same microsecond. Set to 0 to disable.
const JITTER_MS = Number(process.env.JITTER_MS ?? "300");

function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

async function jitter(): Promise<void> {
	if (JITTER_MS > 0) await sleep(Math.random() * JITTER_MS);
}

const clients = REGIONS.map((region) => ({
	label: region.label,
	client: createClient<typeof registry>(region.endpoint),
}));

type Client = (typeof clients)[number];

const TIMEOUT_SENTINEL = Symbol("timeout");
type Timeout = typeof TIMEOUT_SENTINEL;

async function withAttemptTimeout<T>(
	fn: () => Promise<T>,
): Promise<T | Timeout> {
	let timeoutId: ReturnType<typeof setTimeout> | undefined;
	try {
		return await Promise.race<T | Timeout>([
			fn(),
			new Promise<Timeout>((resolve) => {
				timeoutId = setTimeout(
					() => resolve(TIMEOUT_SENTINEL),
					ATTEMPT_TIMEOUT_MS,
				);
			}),
		]);
	} finally {
		if (timeoutId) clearTimeout(timeoutId);
	}
}

function formatError(err: unknown): string {
	if (err instanceof ActorError) return `${err.group}.${err.code} - ${err.message}`;
	if (err instanceof Error) return `${err.name}: ${err.message}`;
	return String(err);
}

function makeKey(scenario: string, round: number): string {
	return `global-smoke-${scenario}-${Date.now()}-${round}-${Math.random()
		.toString(36)
		.slice(2, 10)}`;
}

// MARK: Scenario 1 - create race

interface CreateAttempt {
	region: string;
	durationMs: number;
	outcome:
		| { kind: "success" }
		| { kind: "duplicate" }
		| { kind: "timeout" }
		| { kind: "error"; error: unknown };
}

async function attemptCreate(
	region: Client,
	key: string,
): Promise<CreateAttempt> {
	await jitter();
	const start = Date.now();
	try {
		const result = await withAttemptTimeout(() =>
			region.client.counter.create([key]),
		);
		if (result === TIMEOUT_SENTINEL) {
			return {
				region: region.label,
				durationMs: Date.now() - start,
				outcome: { kind: "timeout" },
			};
		}
		return {
			region: region.label,
			durationMs: Date.now() - start,
			outcome: { kind: "success" },
		};
	} catch (error) {
		const durationMs = Date.now() - start;
		if (
			error instanceof ActorError &&
			error.group === "actor" &&
			error.code === "duplicate_key"
		) {
			return { region: region.label, durationMs, outcome: { kind: "duplicate" } };
		}
		return { region: region.label, durationMs, outcome: { kind: "error", error } };
	}
}

async function runCreateRaceRound(round: number): Promise<boolean> {
	const key = makeKey("create", round);
	console.log(`\n--- Round ${round + 1}/${ROUNDS} key=${key} ---`);

	const attempts = Array.from({ length: PARALLELISM }, (_, i) =>
		attemptCreate(clients[i % clients.length]!, key),
	);
	const results = await Promise.all(attempts);

	const successes = results.filter((r) => r.outcome.kind === "success");
	const duplicates = results.filter((r) => r.outcome.kind === "duplicate");
	const timeouts = results.filter((r) => r.outcome.kind === "timeout");
	const errors = results.filter((r) => r.outcome.kind === "error");

	for (const r of results) {
		const tag =
			r.outcome.kind === "success"
				? "[WIN]      "
				: r.outcome.kind === "duplicate"
					? "[DUPLICATE]"
					: r.outcome.kind === "timeout"
						? "[TIMEOUT]  "
						: "[ERROR]    ";
		console.log(`  ${tag} ${r.region.padEnd(14)} ${r.durationMs}ms`);
	}

	// Check duplicate winners first so uniqueness violations surface even when
	// the round also has flaky errors.
	if (successes.length > 1) {
		console.error(
			`  FAIL: ${successes.length} winners for the same key (global uniqueness violated)`,
		);
		return false;
	}

	if (errors.length > 0 || timeouts.length > 0) {
		console.error(
			`  FAIL: ${errors.length} error(s), ${timeouts.length} timeout(s) after ${ATTEMPT_TIMEOUT_MS}ms`,
		);
		for (const r of errors) {
			if (r.outcome.kind === "error") {
				console.error(`    ${r.region}: ${formatError(r.outcome.error)}`);
			}
		}
		return false;
	}

	if (successes.length !== 1) {
		console.error(
			`  FAIL: expected exactly 1 winner, saw ${successes.length} successes and ${duplicates.length} duplicates`,
		);
		return false;
	}

	console.log(
		`  PASS: 1 winner in ${successes[0]!.region}, ${duplicates.length} duplicate_key rejections`,
	);
	return true;
}

// MARK: Scenario 2 - getOrCreate race

interface GetOrCreateAttempt {
	region: string;
	durationMs: number;
	outcome:
		| { kind: "resolved"; actorId: string }
		| { kind: "timeout" }
		| { kind: "error"; error: unknown };
}

async function attemptGetOrCreate(
	region: Client,
	key: string,
): Promise<GetOrCreateAttempt> {
	await jitter();
	const start = Date.now();
	try {
		// `getOrCreate()` is synchronous (no server call). `resolve()` is what
		// triggers the server-side get-or-create and returns the actor ID.
		const handle = region.client.counter.getOrCreate([key]);
		const result = await withAttemptTimeout(() => handle.resolve());
		if (result === TIMEOUT_SENTINEL) {
			return {
				region: region.label,
				durationMs: Date.now() - start,
				outcome: { kind: "timeout" },
			};
		}
		return {
			region: region.label,
			durationMs: Date.now() - start,
			outcome: { kind: "resolved", actorId: result },
		};
	} catch (error) {
		return {
			region: region.label,
			durationMs: Date.now() - start,
			outcome: { kind: "error", error },
		};
	}
}

async function runGetOrCreateRaceRound(round: number): Promise<boolean> {
	const key = makeKey("getOrCreate", round);
	console.log(`\n--- Round ${round + 1}/${ROUNDS} key=${key} ---`);

	const attempts = Array.from({ length: PARALLELISM }, (_, i) =>
		attemptGetOrCreate(clients[i % clients.length]!, key),
	);
	const results = await Promise.all(attempts);

	const resolved = results.filter(
		(r): r is GetOrCreateAttempt & { outcome: { kind: "resolved"; actorId: string } } =>
			r.outcome.kind === "resolved",
	);
	const timeouts = results.filter((r) => r.outcome.kind === "timeout");
	const errors = results.filter((r) => r.outcome.kind === "error");

	for (const r of results) {
		if (r.outcome.kind === "resolved") {
			console.log(
				`  [RESOLVED] ${r.region.padEnd(14)} ${r.durationMs}ms actorId=${r.outcome.actorId}`,
			);
		} else if (r.outcome.kind === "timeout") {
			console.log(`  [TIMEOUT]  ${r.region.padEnd(14)} ${r.durationMs}ms`);
		} else {
			console.log(`  [ERROR]    ${r.region.padEnd(14)} ${r.durationMs}ms`);
		}
	}

	// Detect multiple actors for the same key by comparing resolved actor IDs.
	const uniqueActorIds = new Set(resolved.map((r) => r.outcome.actorId));
	if (uniqueActorIds.size > 1) {
		console.error(
			`  FAIL: ${uniqueActorIds.size} distinct actor IDs observed for the same key (expected exactly 1)`,
		);
		for (const id of uniqueActorIds) console.error(`    actorId=${id}`);
		return false;
	}

	if (errors.length > 0 || timeouts.length > 0) {
		console.error(
			`  FAIL: ${errors.length} error(s), ${timeouts.length} timeout(s) after ${ATTEMPT_TIMEOUT_MS}ms`,
		);
		for (const r of errors) {
			if (r.outcome.kind === "error") {
				console.error(`    ${r.region}: ${formatError(r.outcome.error)}`);
			}
		}
		return false;
	}

	if (resolved.length !== PARALLELISM) {
		console.error(
			`  FAIL: expected ${PARALLELISM} resolved handles, got ${resolved.length}`,
		);
		return false;
	}

	console.log(
		`  PASS: all ${resolved.length} getOrCreate calls resolved to actorId=${[...uniqueActorIds][0]}`,
	);
	return true;
}

// MARK: Scenario 3 - cross-region visibility

interface GetAttempt {
	region: string;
	durationMs: number;
	outcome:
		| { kind: "resolved"; actorId: string }
		| { kind: "timeout" }
		| { kind: "error"; error: unknown };
}

async function attemptGet(region: Client, key: string): Promise<GetAttempt> {
	await jitter();
	const start = Date.now();
	try {
		const handle = region.client.counter.get([key]);
		const result = await withAttemptTimeout(() => handle.resolve());
		if (result === TIMEOUT_SENTINEL) {
			return {
				region: region.label,
				durationMs: Date.now() - start,
				outcome: { kind: "timeout" },
			};
		}
		return {
			region: region.label,
			durationMs: Date.now() - start,
			outcome: { kind: "resolved", actorId: result },
		};
	} catch (error) {
		return {
			region: region.label,
			durationMs: Date.now() - start,
			outcome: { kind: "error", error },
		};
	}
}

async function runCrossRegionRound(round: number): Promise<boolean> {
	const key = makeKey("cross-region", round);
	// Rotate the creator region each round so both directions are exercised.
	const creator = clients[round % clients.length]!;
	const observers = clients.filter((_, i) => i !== round % clients.length);
	console.log(
		`\n--- Round ${round + 1}/${ROUNDS} key=${key} creator=${creator.label} ---`,
	);

	// Step 1: create the actor in the creator region and capture the actor ID
	// from the create response (as requested: read the result from the DC that
	// performed the create, not from a follow-up lookup).
	let createdActorId: string;
	const createStart = Date.now();
	try {
		const handleOrTimeout = await withAttemptTimeout(() =>
			creator.client.counter.create([key]),
		);
		if (handleOrTimeout === TIMEOUT_SENTINEL) {
			console.error(
				`  FAIL: create in ${creator.label} timed out after ${ATTEMPT_TIMEOUT_MS}ms`,
			);
			return false;
		}
		const idOrTimeout = await withAttemptTimeout(() => handleOrTimeout.resolve());
		if (idOrTimeout === TIMEOUT_SENTINEL) {
			console.error(
				`  FAIL: resolve after create in ${creator.label} timed out after ${ATTEMPT_TIMEOUT_MS}ms`,
			);
			return false;
		}
		createdActorId = idOrTimeout;
	} catch (err) {
		console.error(`  FAIL: create in ${creator.label} errored: ${formatError(err)}`);
		return false;
	}
	console.log(
		`  [CREATE]   ${creator.label.padEnd(14)} ${Date.now() - createStart}ms actorId=${createdActorId}`,
	);

	// Step 2: from every other region, call get() on the key and verify it
	// resolves to the exact actor ID returned by the creator.
	const observations = await Promise.all(
		observers.map((observer) => attemptGet(observer, key)),
	);

	for (const r of observations) {
		if (r.outcome.kind === "resolved") {
			console.log(
				`  [GET]      ${r.region.padEnd(14)} ${r.durationMs}ms actorId=${r.outcome.actorId}`,
			);
		} else if (r.outcome.kind === "timeout") {
			console.log(`  [TIMEOUT]  ${r.region.padEnd(14)} ${r.durationMs}ms`);
		} else {
			console.log(`  [ERROR]    ${r.region.padEnd(14)} ${r.durationMs}ms`);
		}
	}

	const mismatches = observations.filter(
		(r) => r.outcome.kind === "resolved" && r.outcome.actorId !== createdActorId,
	);
	if (mismatches.length > 0) {
		console.error(
			`  FAIL: ${mismatches.length} observer region(s) saw a different actor ID than the creator`,
		);
		for (const r of mismatches) {
			if (r.outcome.kind === "resolved") {
				console.error(
					`    ${r.region}: got ${r.outcome.actorId}, expected ${createdActorId}`,
				);
			}
		}
		return false;
	}

	const timeouts = observations.filter((r) => r.outcome.kind === "timeout");
	const errors = observations.filter((r) => r.outcome.kind === "error");
	if (errors.length > 0 || timeouts.length > 0) {
		console.error(
			`  FAIL: ${errors.length} error(s), ${timeouts.length} timeout(s) after ${ATTEMPT_TIMEOUT_MS}ms`,
		);
		for (const r of errors) {
			if (r.outcome.kind === "error") {
				console.error(`    ${r.region}: ${formatError(r.outcome.error)}`);
			}
		}
		return false;
	}

	console.log(
		`  PASS: ${observers.length} observer region(s) agree on actorId=${createdActorId}`,
	);
	return true;
}

// MARK: Runner

interface Scenario {
	name: string;
	description: string;
	runRound: (round: number) => Promise<boolean>;
}

const SCENARIOS: Scenario[] = [
	{
		name: "create-race",
		description: "N parallel create() calls, expect exactly 1 winner",
		runRound: runCreateRaceRound,
	},
	// {
	// 	name: "getOrCreate-race",
	// 	description: "N parallel getOrCreate() calls, expect all to resolve to the same actor ID",
	// 	runRound: runGetOrCreateRaceRound,
	// },
	// {
	// 	name: "cross-region-visibility",
	// 	description: "create in one region, get() from every other region, expect identical actor IDs",
	// 	runRound: runCrossRegionRound,
	// },
];

async function main() {
	console.log(
		`Global smoke test: ${SCENARIOS.length} scenarios x ${ROUNDS} round(s) x ${PARALLELISM} parallel ops (jitter=${JITTER_MS}ms)`,
	);
	for (const region of REGIONS) {
		const host = new URL(region.endpoint).host;
		console.log(`  region=${region.label.padEnd(14)} host=${host}`);
	}

	const scenarioResults: { name: string; passed: number; failed: number }[] = [];
	let totalFailed = 0;

	for (const scenario of SCENARIOS) {
		console.log(`\n======== ${scenario.name} ========`);
		console.log(scenario.description);
		let passed = 0;
		let failed = 0;
		for (let i = 0; i < ROUNDS; i++) {
			const ok = await scenario.runRound(i);
			if (ok) passed++;
			else failed++;
		}
		scenarioResults.push({ name: scenario.name, passed, failed });
		totalFailed += failed;
		console.log(`\n[${scenario.name}] ${passed}/${ROUNDS} passed, ${failed}/${ROUNDS} failed`);
	}

	console.log(`\n======== Summary ========`);
	for (const s of scenarioResults) {
		console.log(`  ${s.name.padEnd(26)} ${s.passed}/${ROUNDS} passed, ${s.failed} failed`);
	}

	if (totalFailed > 0) process.exit(1);
}

main().catch((err) => {
	console.error("fatal error:", err);
	process.exit(1);
});

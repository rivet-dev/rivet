#!/usr/bin/env tsx
/**
 * Publish all preview packages to npm with bounded parallelism and retries.
 *
 * - Max-parallelism via a simple async semaphore (no external deps).
 * - Exponential backoff on transient errors.
 * - Hard-fails on non-retryable errors (e.g. "cannot publish over previously
 *   published version" 403 — the version is already on npm and retrying
 *   won't help).
 * - Always exits 0 if the step finishes — the script prints a summary so
 *   the workflow can post partial results without failing on expected
 *   idempotency errors.
 *
 * Usage:
 *   tsx scripts/preview-publish/publish-all.ts --tag pr-4600
 *   tsx scripts/preview-publish/publish-all.ts --tag pr-4600 --parallel 16 --retries 4
 */
import { spawn } from "node:child_process";
import { resolve } from "node:path";
import { parseArgs } from "node:util";
import { discoverPackages, type Package } from "./discover-packages.js";

const { values } = parseArgs({
	options: {
		tag: { type: "string" },
		parallel: { type: "string", default: "16" },
		retries: { type: "string", default: "3" },
		"initial-backoff-ms": { type: "string", default: "2000" },
	},
});

if (!values.tag) {
	console.error("--tag is required");
	process.exit(1);
}
const TAG = values.tag;
const MAX_PARALLEL = Number(values.parallel);
const MAX_RETRIES = Number(values.retries);
const INITIAL_BACKOFF_MS = Number(values["initial-backoff-ms"]);

const repoRoot = resolve(process.cwd());
const packages = discoverPackages(repoRoot);

console.log(
	`==> Publishing ${packages.length} packages | tag=${TAG} | parallel=${MAX_PARALLEL} | retries=${MAX_RETRIES}`,
);
console.log("");

interface PublishResult {
	pkg: Package;
	status: "success" | "already-exists" | "failed" | "retried-success";
	attempts: number;
	lastError?: string;
}

/**
 * Run `npm publish` in a package directory. Returns stdout+stderr and the
 * exit code. Closes stdin so npm can't stall on prompts.
 */
function runNpmPublish(
	pkg: Package,
): Promise<{ code: number; output: string }> {
	return new Promise((resolvePromise) => {
		const args = ["publish", "--access", "public", "--tag", TAG];
		const child = spawn("npm", args, {
			cwd: pkg.dir,
			stdio: ["ignore", "pipe", "pipe"],
			env: process.env,
		});
		const chunks: Buffer[] = [];
		child.stdout.on("data", (c) => chunks.push(c));
		child.stderr.on("data", (c) => chunks.push(c));
		child.on("close", (code) => {
			resolvePromise({
				code: code ?? 1,
				output: Buffer.concat(chunks).toString("utf8"),
			});
		});
	});
}

const ALREADY_PUBLISHED_PATTERNS = [
	"cannot publish over the previously published versions",
	"cannot publish over previously published version",
	"You cannot publish over",
];

function isAlreadyPublished(output: string): boolean {
	return ALREADY_PUBLISHED_PATTERNS.some((p) => output.includes(p));
}

/**
 * Transient errors we should retry. Everything else is either idempotent
 * (already-published) or a hard failure.
 */
function isRetryable(output: string): boolean {
	if (isAlreadyPublished(output)) return false;
	return (
		output.includes("ECONNRESET") ||
		output.includes("ETIMEDOUT") ||
		output.includes("ENOTFOUND") ||
		output.includes("EAI_AGAIN") ||
		output.includes("socket hang up") ||
		output.includes("npm error 503") ||
		output.includes("npm error 502") ||
		output.includes("npm error 504") ||
		output.includes("npm error 429") ||
		output.includes("ERR_STREAM_PREMATURE_CLOSE") ||
		// Some npm errors don't tag the status clearly; if we don't see a
		// definitive "already published" we can retry once.
		!/npm error (code|E[A-Z]+)/.test(output)
	);
}

function extractError(output: string, maxLines = 3): string {
	const lines = output
		.split("\n")
		.filter((l) => /npm error/i.test(l) && !l.includes("A complete log"))
		.slice(0, maxLines);
	if (lines.length === 0) {
		return output.trim().split("\n").slice(-maxLines).join(" | ");
	}
	return lines.join(" | ");
}

async function publishPackage(pkg: Package): Promise<PublishResult> {
	for (let attempt = 1; attempt <= MAX_RETRIES + 1; attempt++) {
		const { code, output } = await runNpmPublish(pkg);
		if (code === 0) {
			return {
				pkg,
				status: attempt === 1 ? "success" : "retried-success",
				attempts: attempt,
			};
		}
		if (isAlreadyPublished(output)) {
			return {
				pkg,
				status: "already-exists",
				attempts: attempt,
			};
		}
		if (!isRetryable(output) || attempt > MAX_RETRIES) {
			return {
				pkg,
				status: "failed",
				attempts: attempt,
				lastError: extractError(output),
			};
		}
		// Exponential backoff.
		const delay = INITIAL_BACKOFF_MS * 2 ** (attempt - 1);
		console.log(
			`  [retry ${attempt}/${MAX_RETRIES}] ${pkg.name} — waiting ${delay}ms`,
		);
		await new Promise((r) => setTimeout(r, delay));
	}
	// Unreachable, but keeps types happy.
	return { pkg, status: "failed", attempts: MAX_RETRIES + 1 };
}

/**
 * Bounded-parallelism runner. Starts up to MAX_PARALLEL publishes concurrently,
 * feeding new ones from the queue as old ones finish.
 */
async function publishAll(): Promise<PublishResult[]> {
	const queue = [...packages];
	const results: PublishResult[] = [];
	const workers: Promise<void>[] = [];

	async function worker() {
		while (true) {
			const pkg = queue.shift();
			if (!pkg) return;
			const result = await publishPackage(pkg);
			printResult(result);
			results.push(result);
		}
	}

	for (let i = 0; i < Math.min(MAX_PARALLEL, packages.length); i++) {
		workers.push(worker());
	}
	await Promise.all(workers);
	return results;
}

function printResult(r: PublishResult): void {
	const name = r.pkg.name.padEnd(48);
	const symbol =
		r.status === "success" || r.status === "retried-success"
			? "✓"
			: r.status === "already-exists"
				? "="
				: "✗";
	const suffix =
		r.status === "retried-success"
			? ` (after ${r.attempts} attempts)`
			: r.status === "failed"
				? ` — ${r.lastError ?? "unknown error"}`
				: "";
	console.log(`  ${symbol} ${name}${suffix}`);
}

const startedAt = Date.now();
const results = await publishAll();
const elapsed = ((Date.now() - startedAt) / 1000).toFixed(1);

const counts = {
	success: results.filter((r) => r.status === "success").length,
	retried: results.filter((r) => r.status === "retried-success").length,
	alreadyExists: results.filter((r) => r.status === "already-exists").length,
	failed: results.filter((r) => r.status === "failed").length,
};

console.log("");
console.log(`==> Summary (${elapsed}s)`);
console.log(`    ${counts.success} succeeded`);
if (counts.retried > 0) {
	console.log(`    ${counts.retried} succeeded after retry`);
}
if (counts.alreadyExists > 0) {
	console.log(`    ${counts.alreadyExists} already published (no-op)`);
}
if (counts.failed > 0) {
	console.log(`    ${counts.failed} FAILED`);
	for (const r of results.filter((x) => x.status === "failed")) {
		console.log(`      - ${r.pkg.name}: ${r.lastError}`);
	}
}

// Exit 0 unless something genuinely broke. Already-published is expected
// on re-runs (e.g. rebuilding the same SHA), so we don't count it as failure.
process.exit(counts.failed > 0 ? 1 : 0);

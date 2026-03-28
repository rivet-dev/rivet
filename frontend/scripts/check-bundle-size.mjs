#!/usr/bin/env node
/**
 * Bundle size guardrail for the engine build.
 *
 * Checks:
 *   1. The main entry chunk (index-*.js) does not exceed the gzip size limit.
 *   2. Known heavy libraries that were intentionally removed are not present in
 *      the main bundle's source map (regression detection).
 *
 * Usage:
 *   node scripts/check-bundle-size.mjs [--dist <path>]
 *
 * The dist path defaults to dist/assets (relative to the frontend directory).
 */

import { createReadStream, readdirSync, readFileSync } from "node:fs";
import { createGzip } from "node:zlib";
import { join, resolve } from "node:path";
import { pipeline } from "node:stream/promises";
import { Writable } from "node:stream";

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/** Maximum allowed gzip size for the main entry chunk, in bytes. */
const MAIN_CHUNK_GZIP_LIMIT_BYTES = 1_030_000; // ~1 MB gzip (baseline ~935 kB + ~10% headroom)

/**
 * Packages whose source files must NOT appear in the main bundle's source map.
 * Each entry is matched against source map source paths as a substring.
 */
const BANNED_FROM_MAIN_BUNDLE = [
	{
		match: "/node_modules/.pnpm/posthog-js",
		label: "posthog-js",
		hint: "Ensure posthog imports are guarded by __APP_TYPE__ === \"cloud\" or use the posthogStubPlugin.",
	},
	{
		// @clerk/clerk-js is the heavy Clerk browser runtime (~900 kB gzip). The lighter
		// @clerk/clerk-react hooks and @clerk/shared utilities may legitimately remain.
		match: "node_modules/.pnpm/@clerk+clerk-js",
		label: "@clerk/clerk-js",
		hint: "Ensure ClerkProvider is inside __APP_TYPE__ === \"cloud\" branches in __root.tsx.",
	},
	{
		match: "node_modules/lodash/lodash.js",
		label: "lodash (monolithic)",
		hint: "Replace `import _ from 'lodash'` with native JS or individual lodash function imports.",
	},
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Gzip a file and return the compressed byte count. */
async function gzipSize(filePath) {
	let size = 0;
	const counter = new Writable({
		write(chunk, _enc, cb) {
			size += chunk.length;
			cb();
		},
	});
	await pipeline(createReadStream(filePath), createGzip(), counter);
	return size;
}

function formatBytes(bytes) {
	return `${(bytes / 1024).toFixed(1)} kB`;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const args = process.argv.slice(2);
const distFlagIdx = args.indexOf("--dist");
const distDir = distFlagIdx >= 0 ? resolve(args[distFlagIdx + 1]) : resolve("dist/assets");

let failed = false;
const errors = [];

// Find the main entry chunk: the largest index-<hash>.js (not .map, not index.all-*).
// A single build produces several index-*.js shared chunks; the main bundle is always
// the largest one.
import { statSync } from "node:fs";

const allAssets = readdirSync(distDir);
const mainChunkCandidates = allAssets
	.filter((f) => /^index-[^.]+\.js$/.test(f) && !f.includes("index.all"))
	.map((f) => ({ name: f, size: statSync(join(distDir, f)).size }))
	.sort((a, b) => b.size - a.size);

if (mainChunkCandidates.length === 0) {
	console.error("ERROR: No main entry chunk found in", distDir);
	process.exit(1);
}

const mainChunk = mainChunkCandidates[0].name;
const mainChunkPath = join(distDir, mainChunk);
const mainChunkMapPath = `${mainChunkPath}.map`;

// 1. Check gzip size of the main chunk.
const gzBytes = await gzipSize(mainChunkPath);
const status = gzBytes <= MAIN_CHUNK_GZIP_LIMIT_BYTES ? "PASS" : "FAIL";
console.log(
	`${status}  Main bundle gzip size: ${formatBytes(gzBytes)} (limit: ${formatBytes(MAIN_CHUNK_GZIP_LIMIT_BYTES)})`,
);
if (status === "FAIL") {
	failed = true;
	errors.push(
		`Main bundle gzip size ${formatBytes(gzBytes)} exceeds limit of ${formatBytes(MAIN_CHUNK_GZIP_LIMIT_BYTES)}.`,
	);
}

// 2. Check source map for banned packages.
let sourceMap;
try {
	sourceMap = JSON.parse(readFileSync(mainChunkMapPath, "utf8"));
} catch {
	console.warn(`WARNING: Could not read source map at ${mainChunkMapPath}; skipping source checks.`);
}

if (sourceMap?.sources) {
	for (const banned of BANNED_FROM_MAIN_BUNDLE) {
		const found = sourceMap.sources.some((s) => s && s.includes(banned.match));
		const checkStatus = found ? "FAIL" : "PASS";
		console.log(`${checkStatus}  "${banned.label}" absent from main bundle`);
		if (found) {
			failed = true;
			errors.push(
				`"${banned.label}" was found in the engine main bundle.\n       Hint: ${banned.hint}`,
			);
		}
	}
}

// ---------------------------------------------------------------------------
// Summary
// ---------------------------------------------------------------------------

if (failed) {
	console.error("\nBundle size check FAILED:");
	for (const err of errors) {
		console.error(`  - ${err}`);
	}
	process.exit(1);
} else {
	console.log("\nAll bundle size checks passed.");
}

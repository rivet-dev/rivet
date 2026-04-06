#!/usr/bin/env -S npx tsx

/**
 * Benchmark Report Generator
 *
 * Runs the SQLite benchmark twice (native and WASM) and generates a
 * markdown comparison report.
 *
 * Usage:
 *   RIVET_ENDPOINT=http://127.0.0.1:6420 npx tsx scripts/bench-report.ts
 *   RIVET_ENDPOINT=http://127.0.0.1:6420 npx tsx scripts/bench-report.ts --quick
 */

import { execSync } from "node:child_process";
import { readFileSync, writeFileSync, renameSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const QUICK = process.argv.includes("--quick") ? "--quick" : "";
const endpoint = process.env.RIVET_ENDPOINT;
if (!endpoint) {
	console.error("RIVET_ENDPOINT is required");
	process.exit(1);
}

const NATIVE_NODE = join(
	__dirname,
	"../../../rivetkit-typescript/packages/sqlite-native/sqlite-native.linux-x64-gnu.node",
);
const NATIVE_BAK = NATIVE_NODE + ".bak";

interface BenchEntry {
	name: string;
	elapsedMs: number;
	detail?: string;
}

function runBench(label: string, outputFile: string): BenchEntry[] {
	console.log(`\n${"=".repeat(60)}`);
	console.log(`Running ${label} benchmark...`);
	console.log("=".repeat(60));

	try {
		execSync(
			`BENCH_REPORT=${outputFile} RIVET_ENDPOINT=${endpoint} npx tsx scripts/bench-sqlite.ts ${QUICK}`,
			{ stdio: "inherit", timeout: 600_000 },
		);
	} catch {
		console.error(`${label} benchmark failed`);
	}

	const jsonFile = outputFile.replace(/\.md$/, ".json");
	if (!existsSync(jsonFile)) return [];
	const data = readFileSync(jsonFile, "utf-8");
	return JSON.parse(data) as BenchEntry[];
}

// Run native (default).
const nativeResults = runBench("Native KV Channel", "/tmp/bench-native.md");

// Hide native addon to force WASM fallback.
if (existsSync(NATIVE_NODE)) {
	renameSync(NATIVE_NODE, NATIVE_BAK);
}

let wasmResults: BenchEntry[];
try {
	wasmResults = runBench("WASM VFS", "/tmp/bench-wasm.md");
} finally {
	// Restore native addon.
	if (existsSync(NATIVE_BAK)) {
		renameSync(NATIVE_BAK, NATIVE_NODE);
	}
}

// Build lookup maps.
const nativeMap = new Map(nativeResults.map((e) => [e.name, e]));
const wasmMap = new Map(wasmResults.map((e) => [e.name, e]));

// Collect all unique names in order (native first, then any WASM-only).
const allNames: string[] = [];
const seen = new Set<string>();
for (const e of nativeResults) {
	if (!seen.has(e.name)) { allNames.push(e.name); seen.add(e.name); }
}
for (const e of wasmResults) {
	if (!seen.has(e.name)) { allNames.push(e.name); seen.add(e.name); }
}

// Generate markdown.
const lines: string[] = [];
const now = new Date().toISOString().slice(0, 19).replace("T", " ");
lines.push(`# SQLite Benchmark Report`);
lines.push(``);
lines.push(`Generated: ${now} UTC`);
lines.push(`Engine: ${endpoint}`);
lines.push(`Mode: ${QUICK ? "quick" : "full"}`);
lines.push(``);

// Summary table.
lines.push(`## Results`);
lines.push(``);
lines.push(`| Benchmark | Native (ms) | WASM (ms) | Speedup | Native/op | WASM/op |`);
lines.push(`|-----------|------------:|----------:|--------:|----------:|--------:|`);

for (const name of allNames) {
	const n = nativeMap.get(name);
	const w = wasmMap.get(name);

	const nMs = n && n.elapsedMs > 0 ? n.elapsedMs : null;
	const wMs = w && w.elapsedMs > 0 ? w.elapsedMs : null;

	const nStr = nMs !== null ? nMs.toFixed(1) : n?.detail === "TIMEOUT" ? "TIMEOUT" : "-";
	const wStr = wMs !== null ? wMs.toFixed(1) : w?.detail === "TIMEOUT" ? "TIMEOUT" : "-";

	let speedup = "-";
	if (nMs !== null && wMs !== null && nMs > 0) {
		const ratio = wMs / nMs;
		speedup = ratio >= 1.1 ? `**${ratio.toFixed(1)}x**` : ratio <= 0.9 ? `${ratio.toFixed(1)}x` : `~1.0x`;
	}

	const nOp = n?.detail && n.detail !== "TIMEOUT" ? n.detail : "-";
	const wOp = w?.detail && w.detail !== "TIMEOUT" ? w.detail : "-";

	lines.push(`| ${name} | ${nStr} | ${wStr} | ${speedup} | ${nOp} | ${wOp} |`);
}

// Scale sweep tables (group by prefix).
const scalePrefixes = ["Insert single", "Insert batch", "Insert TX", "Point read", "Mixed OLTP", "Hot row updates"];
for (const prefix of scalePrefixes) {
	const scaleEntries = allNames.filter((name) => name.startsWith(prefix + " x"));
	if (scaleEntries.length < 2) continue;

	lines.push(``);
	lines.push(`### ${prefix} (scale sweep)`);
	lines.push(``);
	lines.push(`| N | Native (ms) | Native/op | WASM (ms) | WASM/op | Speedup |`);
	lines.push(`|--:|------------:|----------:|----------:|--------:|--------:|`);

	for (const name of scaleEntries) {
		const n = nativeMap.get(name);
		const w = wasmMap.get(name);
		const nMs = n && n.elapsedMs > 0 ? n.elapsedMs : null;
		const wMs = w && w.elapsedMs > 0 ? w.elapsedMs : null;

		// Extract N from name like "Insert single x1000"
		const nMatch = name.match(/x(\d+)/);
		const count = nMatch ? nMatch[1] : "?";

		const nStr = nMs !== null ? nMs.toFixed(1) : n?.detail === "TIMEOUT" ? "TIMEOUT" : "-";
		const wStr = wMs !== null ? wMs.toFixed(1) : w?.detail === "TIMEOUT" ? "TIMEOUT" : "-";
		const nOp = n?.detail && n.detail !== "TIMEOUT" ? n.detail : "-";
		const wOp = w?.detail && w.detail !== "TIMEOUT" ? w.detail : "-";

		let speedup = "-";
		if (nMs !== null && wMs !== null && nMs > 0) {
			const ratio = wMs / nMs;
			speedup = ratio >= 1.1 ? `**${ratio.toFixed(1)}x**` : ratio <= 0.9 ? `${ratio.toFixed(1)}x` : `~1.0x`;
		}

		lines.push(`| ${count} | ${nStr} | ${nOp} | ${wStr} | ${wOp} | ${speedup} |`);
	}
}

// Totals.
const nTotal = nativeResults.reduce((s, e) => s + (e.elapsedMs > 0 ? e.elapsedMs : 0), 0);
const wTotal = wasmResults.reduce((s, e) => s + (e.elapsedMs > 0 ? e.elapsedMs : 0), 0);
lines.push(``);
lines.push(`## Totals`);
lines.push(``);
lines.push(`- **Native total**: ${(nTotal / 1000).toFixed(1)}s`);
lines.push(`- **WASM total**: ${(wTotal / 1000).toFixed(1)}s`);
lines.push(`- **Overall speedup**: ${(wTotal / nTotal).toFixed(1)}x`);
lines.push(``);

const reportPath = process.env.BENCH_REPORT || "/home/nathan/rivet-5/.agent/notes/bench-report.md";
writeFileSync(reportPath, lines.join("\n"));
console.log(`\nReport written to ${reportPath}`);

process.exit(0);

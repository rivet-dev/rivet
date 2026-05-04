import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const runnerPath = `${root}/scripts/sqlite-realworld-bench.ts`;
const actorPath = `${root}/src/actors/testing/sqlite-realworld-bench.ts`;

function read(path: string) {
	return readFileSync(path, "utf8");
}

function extractWorkloads(source: string) {
	const match = /const WORKLOADS = \[([\s\S]*?)\] as const;/.exec(source);
	assert.ok(match, "WORKLOADS catalog should exist");
	return [...match[1].matchAll(/"([^"]+)"/g)].map((entry) => entry[1]);
}

test("SQLite real-world benchmark catalogs stay in sync", () => {
	const runnerWorkloads = extractWorkloads(read(runnerPath));
	const actorWorkloads = extractWorkloads(read(actorPath));

	assert.deepEqual(actorWorkloads, runnerWorkloads);
});

test("SQLite real-world benchmark includes read-mode/write-mode scenarios", () => {
	const runner = read(runnerPath);
	const actor = read(actorPath);

	for (const workload of [
		"parallel-read-aggregates",
		"parallel-read-write-transition",
	]) {
		assert.match(runner, new RegExp(`name: "${workload}"`));
		assert.match(actor, new RegExp(`case "${workload}"`));
	}

	assert.match(
		runner,
		/read mode may hold multiple read-only connections, while write mode must close readers and hold exactly one writable connection|read-mode to write-mode transition|read-only SQLite connections overlap VFS misses/,
	);
	assert.match(actor, /Promise\.all\(\[/);
	assert.match(actor, /UPDATE rw_orders SET total_cents = total_cents \+ 1/);
	assert.match(
		runner,
		/\| workload \| category \| size \| server_ms \| get_pages \| fetched_pages \|/,
	);
});

test("SQLite real-world benchmark defines an optimization impact matrix", () => {
	const runner = read(runnerPath);
	const actor = read(actorPath);

	assert.match(runner, /--matrix <name>/);
	assert.match(runner, /--vfs-round-trip-latency-ms <n>/);
	assert.match(runner, /RIVETKIT_SQLITE_BENCH_VFS_ROUND_TRIP_LATENCY_MS/);
	assert.match(runner, /const SQLITE_OPTIMIZATION_MATRIX_SCENARIOS/);
	for (const scenario of [
		"defaults",
		"all-off",
		"vfs-cache-only",
		"read-ahead-no-cache",
		"cache-read-ahead-no-preload",
		"no-read-ahead",
		"no-vfs-cache",
		"no-preload",
	]) {
		assert.match(runner, new RegExp(`id: "${scenario}"`));
	}
	for (const workload of [
		"rowid-range-forward",
		"secondary-index-scattered-table",
		"parallel-read-aggregates",
		"parallel-read-write-transition",
		"chat-log-select-indexed",
		"chat-log-sum",
		"chat-tool-read-fanout",
		"chat-tool-script",
		"migration-create-indexes-large",
	]) {
		assert.match(runner, new RegExp(`"${workload}"`));
	}
	assert.match(actor, /rw_chat_log/);
	assert.match(actor, /chat-tool-read-fanout/);
	assert.match(actor, /chat-tool-script/);
	assert.match(runner, /matrix-results\.json/);
	assert.match(runner, /matrix-summary\.md/);
});

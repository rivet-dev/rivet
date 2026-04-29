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
	for (const metric of [
		"sqlite_read_pool_routed_read_queries_total",
		"sqlite_read_pool_write_fallback_queries_total",
		"sqlite_read_pool_mode_transitions_total",
	]) {
		assert.match(runner, new RegExp(metric));
	}
	assert.match(
		runner,
		/\| workload \| category \| size \| server_ms \| routed_reads \| write_fallbacks \| mode_transitions \|/,
	);
});

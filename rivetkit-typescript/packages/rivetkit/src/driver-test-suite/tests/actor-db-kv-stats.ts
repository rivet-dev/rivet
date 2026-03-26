import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest, waitFor } from "../utils";

export function runActorDbKvStatsTests(driverTestConfig: DriverTestConfig) {
	describe("Actor Database KV Stats Tests", () => {
		// -- Warm path tests --
		// These call warmUp first to prime the pager cache and reset
		// stats, then measure the exact KV behavior of subsequent ops.
		// This is the steady-state path for a live actor.

		test("warm UPDATE uses BATCH_ATOMIC: exactly 1 putBatch, 0 reads, no journal", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.dbKvStatsActor.getOrCreate([
				`kv-stats-ba-${crypto.randomUUID()}`,
			]);

			await actor.warmUp();

			await actor.increment();
			const stats = await actor.getStats();
			const log = await actor.getLog();

			expect(stats.putBatchCalls).toBe(1);
			expect(stats.getBatchCalls).toBe(0);

			const allKeys = log.flatMap((e: { keys: string[] }) => e.keys);
			const journalKeys = allKeys.filter((k: string) =>
				k.includes("journal"),
			);
			expect(journalKeys.length).toBe(0);
		}, 30_000);

		test("warm SELECT uses 0 KV round trips", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.dbKvStatsActor.getOrCreate([
				`kv-stats-2-${crypto.randomUUID()}`,
			]);

			await actor.warmUp();

			await actor.getCount();
			const stats = await actor.getStats();

			expect(stats.getBatchCalls).toBe(0);
			expect(stats.putBatchCalls).toBe(0);
		}, 30_000);

		test("warm SELECT after UPDATE adds no KV round trips", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.dbKvStatsActor.getOrCreate([
				`kv-stats-3-${crypto.randomUUID()}`,
			]);

			await actor.warmUp();

			await actor.increment();
			const updateStats = await actor.getStats();

			await actor.resetStats();
			await actor.incrementAndRead();
			const combinedStats = await actor.getStats();

			expect(combinedStats.putBatchCalls).toBe(updateStats.putBatchCalls);
			expect(combinedStats.getBatchCalls).toBe(updateStats.getBatchCalls);
		}, 30_000);

		test("warm multi-page INSERT writes multiple chunk keys", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.dbKvStatsActor.getOrCreate([
				`kv-stats-4-${crypto.randomUUID()}`,
			]);

			// First call creates table/index and primes cache
			await actor.insertWithIndex();
			await actor.resetStats();

			await actor.insertWithIndex();
			const stats = await actor.getStats();
			const log = await actor.getLog();

			expect(stats.putBatchCalls).toBeGreaterThanOrEqual(1);
			expect(stats.putBatchEntries).toBeGreaterThan(1);

			const putOps = log.filter(
				(e: { op: string }) => e.op === "putBatch" || e.op === "put",
			);
			const allKeys = putOps.flatMap((e: { keys: string[] }) => e.keys);
			const mainChunkKeys = allKeys.filter((k: string) =>
				k.startsWith("chunk:main["),
			);
			expect(mainChunkKeys.length).toBeGreaterThanOrEqual(1);
		}, 30_000);

		test("warm ROLLBACK produces no data page writes", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.dbKvStatsActor.getOrCreate([
				`kv-stats-5-${crypto.randomUUID()}`,
			]);

			await actor.rollbackTest();
			await actor.resetStats();

			await actor.rollbackTest();
			const log = await actor.getLog();

			const putOps = log.filter(
				(e: { op: string }) => e.op === "putBatch" || e.op === "put",
			);
			const mainChunkKeys = putOps
				.flatMap((e: { keys: string[] }) => e.keys)
				.filter((k: string) => k.startsWith("chunk:main["));
			expect(mainChunkKeys.length).toBe(0);
		}, 30_000);

		test("warm multi-statement transaction produces writes", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.dbKvStatsActor.getOrCreate([
				`kv-stats-6-${crypto.randomUUID()}`,
			]);

			await actor.multiStmtTx();
			await actor.resetStats();

			await actor.multiStmtTx();
			const stats = await actor.getStats();

			expect(stats.putBatchCalls).toBeGreaterThanOrEqual(1);
		}, 30_000);

		// -- Structural property tests --
		// These assert invariants that hold regardless of cache state.

		test("no WAL or SHM operations occur", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.dbKvStatsActor.getOrCreate([
				`kv-stats-7-${crypto.randomUUID()}`,
			]);

			await actor.warmUp();

			await actor.increment();
			const log = await actor.getLog();

			const allKeys = log.flatMap((e: { keys: string[] }) => e.keys);
			const walOrShmKeys = allKeys.filter(
				(k: string) => k.includes("wal") || k.includes("shm"),
			);
			expect(walOrShmKeys.length).toBe(0);
		}, 30_000);

		test("every putBatch has at most 128 keys", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.dbKvStatsActor.getOrCreate([
				`kv-stats-8-${crypto.randomUUID()}`,
			]);

			await actor.warmUp();

			await actor.increment();
			const log = await actor.getLog();

			const putBatchOps = log.filter(
				(e: { op: string }) => e.op === "putBatch",
			);
			for (const entry of putBatchOps) {
				expect(
					(entry as { keys: string[] }).keys.length,
				).toBeLessThanOrEqual(128);
			}
		}, 30_000);

		// -- Large transaction tests --

		test("large transaction falls back to journal when exceeding 127 dirty pages", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.dbKvStatsActor.getOrCreate([
				`kv-stats-9-${crypto.randomUUID()}`,
			]);

			await actor.warmUp();

			await actor.bulkInsertLarge();
			const stats = await actor.getStats();
			const log = await actor.getLog();

			expect(stats.putBatchCalls).toBeGreaterThan(1);

			const allKeys = log.flatMap((e: { keys: string[] }) => e.keys);
			const journalKeys = allKeys.filter((k: string) =>
				k.includes("journal"),
			);
			expect(journalKeys.length).toBeGreaterThan(0);

			const putBatchOps = log.filter(
				(e: { op: string }) => e.op === "putBatch",
			);
			for (const entry of putBatchOps) {
				expect(
					(entry as { keys: string[] }).keys.length,
				).toBeLessThanOrEqual(128);
			}
		}, 60_000);

		test("large transaction data integrity: 200 rows and integrity check pass", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.dbKvStatsActor.getOrCreate([
				`kv-stats-10-${crypto.randomUUID()}`,
			]);

			await actor.bulkInsertLarge();

			const count = await actor.getRowCount();
			expect(count).toBe(200);

			const integrity = await actor.runIntegrityCheck();
			expect(integrity).toBe("ok");
		}, 60_000);

		test("large transaction survives actor sleep and wake", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.dbKvStatsActor.getOrCreate([
				`kv-stats-11-${crypto.randomUUID()}`,
			]);

			await actor.bulkInsertLarge();
			const countBefore = await actor.getRowCount();
			expect(countBefore).toBe(200);

			await actor.triggerSleep();
			await waitFor(driverTestConfig, 250);

			const countAfter = await actor.getRowCount();
			expect(countAfter).toBe(200);

			const integrity = await actor.runIntegrityCheck();
			expect(integrity).toBe("ok");
		}, 60_000);
	});
}

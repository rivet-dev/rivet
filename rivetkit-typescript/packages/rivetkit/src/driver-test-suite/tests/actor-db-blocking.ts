import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

const TEST_TIMEOUT_MS = 120_000;

export function runActorDbBlockingTests(driverTestConfig: DriverTestConfig) {
	describe("Actor Database Blocking Tests", () => {
		test(
			"heavy SQLite query blocks event loop (concurrency test)",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const actor = client.dbBlockingActor.getOrCreate([
					`db-blocking-concurrency-${crypto.randomUUID()}`,
				]);

				// Run the concurrency test with enough rows to make the cross-join slow
				const timeline = await actor.concurrencyTest(500);

				console.log("\n=== SQLite Blocking Timeline ===");
				const startTs = timeline[0]!.ts;
				for (const entry of timeline) {
					console.log(
						`  ${entry.label}: +${entry.ts - startTs}ms`,
					);
				}

				// Analyze the timeline
				const seeded = timeline.find((e) => e.label === "seeded");
				const queryDone = timeline.find(
					(e) => e.label === "query_done",
				);
				const afterMicrotask = timeline.find(
					(e) => e.label === "after_microtask",
				);
				const afterSettimeout = timeline.find(
					(e) => e.label === "after_settimeout0",
				);

				expect(seeded).toBeDefined();
				expect(queryDone).toBeDefined();
				expect(afterMicrotask).toBeDefined();
				expect(afterSettimeout).toBeDefined();

				const queryDurationMs = queryDone!.ts - seeded!.ts;
				console.log(`\nQuery duration: ${queryDurationMs}ms`);

				// If the event loop is blocked, setTimeout(0) will resolve
				// AFTER the query completes (not during it)
				const settimeoutDelay = afterSettimeout!.ts - seeded!.ts;
				console.log(`setTimeout(0) resolved: +${settimeoutDelay}ms after seed`);

				// If setTimeout(0) resolves at roughly the same time as query_done,
				// it means the event loop was blocked during the query
				if (settimeoutDelay > queryDurationMs * 0.8) {
					console.log(
						"\n*** BLOCKED: setTimeout(0) was delayed by the SQLite query ***",
					);
					console.log(
						"This confirms the WASM execution blocks the JS event loop.",
					);
				} else {
					console.log(
						"\n*** NOT BLOCKED: setTimeout(0) resolved during the query ***",
					);
				}
			},
			TEST_TIMEOUT_MS,
		);

		test(
			"concurrent actions are serialized by SQLite mutex",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const actor = client.dbBlockingActor.getOrCreate([
					`db-blocking-mutex-${crypto.randomUUID()}`,
				]);

				// Seed data
				const seeded = await actor.seedData(500);
				expect(seeded).toBe(500);

				// Fire off two heavy queries concurrently
				const startTs = Date.now();
				const [result1, result2] = await Promise.all([
					actor.heavyQuery(),
					actor.heavyQuery(),
				]);
				const totalMs = Date.now() - startTs;

				console.log("\n=== Concurrent Query Test ===");
				console.log(`Query 1: ${result1.durationMs}ms`);
				console.log(`Query 2: ${result2.durationMs}ms`);
				console.log(`Total wall time: ${totalMs}ms`);

				// If mutex serializes them, total should be roughly sum of both
				// If they ran in parallel, total would be roughly max of both
				const sumMs = result1.durationMs + result2.durationMs;
				const ratio = totalMs / Math.max(result1.durationMs, result2.durationMs);
				console.log(`Serialization ratio: ${ratio.toFixed(2)}x (1.0 = parallel, 2.0 = fully serialized)`);

				if (ratio > 1.5) {
					console.log("\n*** SERIALIZED: Queries ran sequentially due to mutex ***");
				} else {
					console.log("\n*** PARALLEL: Queries ran concurrently ***");
				}
			},
			TEST_TIMEOUT_MS,
		);

		test(
			"action during heavy query is delayed",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const actor = client.dbBlockingActor.getOrCreate([
					`db-blocking-action-delay-${crypto.randomUUID()}`,
				]);

				// Seed data
				await actor.seedData(500);
				await actor.clearEventLog();

				// Start heavy query and try to log an event concurrently
				const startTs = Date.now();
				const [queryResult, eventTs] = await Promise.all([
					actor.heavyQuery(),
					// This action should be delayed if the event loop is blocked
					actor.logEvent("during_heavy_query"),
				]);
				const totalMs = Date.now() - startTs;

				console.log("\n=== Action During Heavy Query ===");
				console.log(`Heavy query: ${queryResult.durationMs}ms`);
				console.log(`logEvent returned at: +${eventTs - startTs}ms`);
				console.log(`Total: ${totalMs}ms`);

				// If the logEvent action was delayed by the query, its timestamp
				// will be close to the query completion time
				const eventDelay = eventTs - startTs;
				if (eventDelay > queryResult.durationMs * 0.5) {
					console.log(
						"\n*** BLOCKED: logEvent was delayed by the heavy query ***",
					);
				} else {
					console.log(
						"\n*** NOT BLOCKED: logEvent ran independently ***",
					);
				}
			},
			TEST_TIMEOUT_MS,
		);
	});
}

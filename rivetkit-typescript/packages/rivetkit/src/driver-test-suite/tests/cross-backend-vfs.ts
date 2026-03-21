import { describe, expect, test } from "vitest";
import {
	nativeSqliteAvailable,
	_resetNativeDetection,
} from "@/db/native-sqlite";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest, waitFor } from "../utils";

const SLEEP_WAIT_MS = 500;
const CROSS_BACKEND_TIMEOUT_MS = 30_000;

/**
 * Cross-backend VFS compatibility tests.
 *
 * Verifies that data written by the WASM VFS can be read by the native VFS
 * and vice versa. Both VFS implementations store data in the same KV format
 * (chunk keys, chunk data, metadata encoding). These tests catch encoding
 * mismatches like the metadata version prefix difference fixed in US-024.
 *
 * Skipped when the native SQLite addon is not available.
 */
export function runCrossBackendVfsTests(driverTestConfig: DriverTestConfig) {
	const nativeAvailable = nativeSqliteAvailable();

	describe.skipIf(!nativeAvailable)(
		"Cross-Backend VFS Compatibility Tests",
		() => {
			test(
				"WASM-to-native: data written with WASM VFS is readable with native VFS",
				async (c) => {
					// Restore native detection on cleanup
					c.onTestFinished(() => {
						_resetNativeDetection();
					});

					// Phase 1: Force WASM VFS
					_resetNativeDetection(true);

					const { client } = await setupDriverTest(
						c,
						driverTestConfig,
					);
					const actorId = `cross-w2n-${crypto.randomUUID()}`;
					const actor = client.dbActorRaw.getOrCreate([actorId]);

					// Write structured data with various sizes to exercise
					// chunk boundaries (CHUNK_SIZE = 4096).
					await actor.insertValue("wasm-alpha");
					await actor.insertValue("wasm-beta");
					await actor.insertMany(10);

					// Large payload spanning multiple chunks
					const { id: largeId } =
						await actor.insertPayloadOfSize(8192);

					const wasmCount = await actor.getCount();
					expect(wasmCount).toBe(13);

					const wasmValues = await actor.getValues();
					const wasmLargePayloadSize =
						await actor.getPayloadSize(largeId);
					expect(wasmLargePayloadSize).toBe(8192);

					// Sleep the actor to flush all data to KV
					await actor.triggerSleep();
					await waitFor(driverTestConfig, SLEEP_WAIT_MS);

					// Phase 2: Restore native VFS detection
					_resetNativeDetection();

					// Recreate the actor. The db() provider now uses native
					// SQLite, reading data written by the WASM VFS.
					const actor2 = client.dbActorRaw.getOrCreate([actorId]);

					const nativeCount = await actor2.getCount();
					expect(nativeCount).toBe(13);

					const nativeValues = await actor2.getValues();
					expect(nativeValues).toHaveLength(wasmValues.length);
					for (let i = 0; i < wasmValues.length; i++) {
						expect(nativeValues[i].value).toBe(
							wasmValues[i].value,
						);
					}

					const nativeLargePayloadSize =
						await actor2.getPayloadSize(largeId);
					expect(nativeLargePayloadSize).toBe(8192);

					// Verify integrity
					const integrity = await actor2.integrityCheck();
					expect(integrity).toBe("ok");
				},
				CROSS_BACKEND_TIMEOUT_MS,
			);

			test(
				"native-to-WASM: data written with native VFS is readable with WASM VFS",
				async (c) => {
					// Restore native detection on cleanup
					c.onTestFinished(() => {
						_resetNativeDetection();
					});

					// Phase 1: Use native VFS (default when addon is available)
					_resetNativeDetection();

					const { client } = await setupDriverTest(
						c,
						driverTestConfig,
					);
					const actorId = `cross-n2w-${crypto.randomUUID()}`;
					const actor = client.dbActorRaw.getOrCreate([actorId]);

					// Write structured data with various sizes
					await actor.insertValue("native-alpha");
					await actor.insertValue("native-beta");
					await actor.insertMany(10);

					// Large payload spanning multiple chunks
					const { id: largeId } =
						await actor.insertPayloadOfSize(8192);

					const nativeCount = await actor.getCount();
					expect(nativeCount).toBe(13);

					const nativeValues = await actor.getValues();
					const nativeLargePayloadSize =
						await actor.getPayloadSize(largeId);
					expect(nativeLargePayloadSize).toBe(8192);

					// Sleep the actor to flush all data to KV
					await actor.triggerSleep();
					await waitFor(driverTestConfig, SLEEP_WAIT_MS);

					// Phase 2: Force WASM VFS
					_resetNativeDetection(true);

					// Recreate the actor. The db() provider now uses WASM
					// SQLite, reading data written by the native VFS.
					const actor2 = client.dbActorRaw.getOrCreate([actorId]);

					const wasmCount = await actor2.getCount();
					expect(wasmCount).toBe(13);

					const wasmValues = await actor2.getValues();
					expect(wasmValues).toHaveLength(nativeValues.length);
					for (let i = 0; i < nativeValues.length; i++) {
						expect(wasmValues[i].value).toBe(
							nativeValues[i].value,
						);
					}

					const wasmLargePayloadSize =
						await actor2.getPayloadSize(largeId);
					expect(wasmLargePayloadSize).toBe(8192);

					// Verify integrity
					const integrity = await actor2.integrityCheck();
					expect(integrity).toBe("ok");
				},
				CROSS_BACKEND_TIMEOUT_MS,
			);
		},
	);
}

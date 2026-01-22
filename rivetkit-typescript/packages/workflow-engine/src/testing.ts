import type { EngineDriver, KVEntry, KVWrite } from "./driver.js";
import { createDefaultMessageDriver } from "./storage.js";
import { compareKeys, keyStartsWith, keyToHex } from "./keys.js";
import { sleep } from "./utils.js";

/**
 * In-memory implementation of EngineDriver for testing.
 * Uses binary keys (Uint8Array) with hex encoding for internal Map storage.
 */
export class InMemoryDriver implements EngineDriver {
	// Map from hex-encoded key to { originalKey, value }
	private kv = new Map<string, { key: Uint8Array; value: Uint8Array }>();
	private alarms = new Map<string, number>();

	/** Simulated latency per operation (ms) */
	latency = 10;

	/** How often the worker polls for work */
	workerPollInterval = 100;
	messageDriver = createDefaultMessageDriver(this);

	async get(key: Uint8Array): Promise<Uint8Array | null> {
		await sleep(this.latency);
		const entry = this.kv.get(keyToHex(key));
		return entry?.value ?? null;
	}

	async set(key: Uint8Array, value: Uint8Array): Promise<void> {
		await sleep(this.latency);
		this.kv.set(keyToHex(key), { key, value });
	}

	async delete(key: Uint8Array): Promise<void> {
		await sleep(this.latency);
		this.kv.delete(keyToHex(key));
	}

	async deletePrefix(prefix: Uint8Array): Promise<void> {
		await sleep(this.latency);
		for (const [hexKey, entry] of this.kv) {
			if (keyStartsWith(entry.key, prefix)) {
				this.kv.delete(hexKey);
			}
		}
	}

	async list(prefix: Uint8Array): Promise<KVEntry[]> {
		await sleep(this.latency);
		const results: KVEntry[] = [];
		for (const entry of this.kv.values()) {
			if (keyStartsWith(entry.key, prefix)) {
				results.push({ key: entry.key, value: entry.value });
			}
		}
		// Sort by key lexicographically
		return results.sort((a, b) => compareKeys(a.key, b.key));
	}

	async batch(writes: KVWrite[]): Promise<void> {
		await sleep(this.latency);
		for (const { key, value } of writes) {
			this.kv.set(keyToHex(key), { key, value });
		}
	}

	async setAlarm(workflowId: string, wakeAt: number): Promise<void> {
		await sleep(this.latency);
		this.alarms.set(workflowId, wakeAt);
	}

	async clearAlarm(workflowId: string): Promise<void> {
		await sleep(this.latency);
		this.alarms.delete(workflowId);
	}

	/**
	 * Get the alarm time for a workflow (for testing).
	 */
	getAlarm(workflowId: string): number | undefined {
		return this.alarms.get(workflowId);
	}

	/**
	 * Check if any alarms are due and return their workflow IDs.
	 */
	getDueAlarms(): string[] {
		const now = Date.now();
		const due: string[] = [];
		for (const [workflowId, wakeAt] of this.alarms) {
			if (wakeAt <= now) {
				due.push(workflowId);
			}
		}
		return due;
	}

	/**
	 * Clear all data (for testing).
	 */
	clear(): void {
		this.kv.clear();
		this.alarms.clear();
	}

	/**
	 * Get a snapshot of all data (for testing/debugging).
	 */
	snapshot(): {
		kv: Record<string, Uint8Array>;
		alarms: Record<string, number>;
	} {
		const kvSnapshot: Record<string, Uint8Array> = {};
		for (const [hexKey, entry] of this.kv) {
			kvSnapshot[hexKey] = entry.value;
		}
		return {
			kv: kvSnapshot,
			alarms: Object.fromEntries(this.alarms),
		};
	}

	/**
	 * Get all hex-encoded keys (for testing).
	 */
	keys(): string[] {
		return [...this.kv.keys()];
	}
}

// Export serde functions for testing
export { serializeMessage } from "../schemas/serde.js";
// Re-export main exports for convenience
export * from "./index.js";

// Export key builders for testing
export { buildMessageKey } from "./keys.js";

import type { ActorDriver } from "../driver.js";

/**
 * Collects KV write entries during new actor initialization and flushes them
 * in a single kvBatchPut call. This reduces 3 sequential write round-trips
 * (persist data, queue metadata, inspector token) to 1 batched round-trip.
 */
export class WriteCollector {
	#entries: [Uint8Array, Uint8Array][] = [];
	#driver: ActorDriver;
	#actorId: string;

	constructor(driver: ActorDriver, actorId: string) {
		this.#driver = driver;
		this.#actorId = actorId;
	}

	/** Adds a key-value pair to the batch. */
	add(key: Uint8Array, value: Uint8Array): void {
		this.#entries.push([key, value]);
	}

	/** Sends all collected entries in a single kvBatchPut call. */
	async flush(): Promise<void> {
		if (this.#entries.length === 0) return;
		await this.#driver.kvBatchPut(this.#actorId, this.#entries);
		this.#entries = [];
	}
}

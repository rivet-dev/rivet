/**
 * A key-value entry returned from list operations.
 */
export interface KVEntry {
	key: Uint8Array;
	value: Uint8Array;
}

/**
 * A write operation for batch writes.
 */
export interface KVWrite {
	key: Uint8Array;
	value: Uint8Array;
}

/**
 * The engine driver provides the KV and scheduling interface.
 * Implementations must provide these methods to integrate with different backends.
 *
 * IMPORTANT: Each workflow instance must have its own isolated driver/KV namespace.
 * The workflow engine is the sole reader/writer of its KV during execution.
 * KV operations do not include workflow IDs because isolation is provided externally
 * by the host system (e.g., Cloudflare Durable Objects, dedicated actor processes).
 *
 * External systems may only write messages to the KV (via WorkflowHandle.message()).
 * See architecture.md "Isolation Model" for details.
 */
export interface EngineDriver {
	// === KV Operations ===

	/**
	 * Get a value by key.
	 * Returns null if the key doesn't exist.
	 */
	get(key: Uint8Array): Promise<Uint8Array | null>;

	/**
	 * Set a value by key.
	 */
	set(key: Uint8Array, value: Uint8Array): Promise<void>;

	/**
	 * Delete a key.
	 */
	delete(key: Uint8Array): Promise<void>;

	/**
	 * Delete all keys with a given prefix.
	 */
	deletePrefix(prefix: Uint8Array): Promise<void>;

	/**
	 * List all key-value pairs with a given prefix.
	 *
	 * IMPORTANT: Results MUST be sorted by key in lexicographic byte order.
	 * The workflow engine relies on this ordering for correct message FIFO
	 * processing and name registry reconstruction. Failing to sort will
	 * cause non-deterministic replay behavior.
	 */
	list(prefix: Uint8Array): Promise<KVEntry[]>;

	/**
	 * Batch write multiple key-value pairs.
	 * Should be atomic if possible.
	 */
	batch(writes: KVWrite[]): Promise<void>;

	// === Scheduling ===

	/**
	 * Set an alarm to wake the workflow at a specific time.
	 * @param workflowId The workflow to wake
	 * @param wakeAt Timestamp in milliseconds when to wake
	 */
	setAlarm(workflowId: string, wakeAt: number): Promise<void>;

	/**
	 * Clear any pending alarm for a workflow.
	 */
	clearAlarm(workflowId: string): Promise<void>;

	/**
	 * How often the worker polls for work (in milliseconds).
	 * Affects the threshold for in-memory vs scheduled sleeps.
	 */
	readonly workerPollInterval: number;
}

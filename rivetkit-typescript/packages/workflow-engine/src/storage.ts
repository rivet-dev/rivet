import {
	deserializeEntry,
	deserializeEntryMetadata,
	deserializeName,
	deserializeWorkflowError,
	deserializeWorkflowOutput,
	deserializeWorkflowState,
	serializeEntry,
	serializeEntryMetadata,
	serializeName,
	serializeWorkflowError,
	serializeWorkflowOutput,
	serializeWorkflowState,
} from "../schemas/serde.js";
import type { EngineDriver, KVWrite } from "./driver.js";
import {
	buildEntryMetadataKey,
	buildEntryMetadataPrefix,
	buildHistoryKey,
	buildHistoryPrefix,
	buildHistoryPrefixAll,
	buildNameKey,
	buildNamePrefix,
	buildWorkflowErrorKey,
	buildWorkflowOutputKey,
	buildWorkflowStateKey,
	compareKeys,
	parseEntryMetadataKey,
	parseNameKey,
} from "./keys.js";
import { isLocationPrefix, locationToKey } from "./location.js";
import type {
	Entry,
	EntryKind,
	EntryMetadata,
	Location,
	Storage,
	WorkflowEntryMetadataSnapshot,
	WorkflowHistoryEntry,
	WorkflowHistorySnapshot,
} from "./types.js";

export const MAX_KV_BATCH_ENTRIES = 128;
export const MAX_KV_BATCH_PAYLOAD_BYTES = 976 * 1024;

/** Max delete ops (one transaction/permit each) run at once, under the 128-permit cap. */
export const MAX_CONCURRENT_DELETES = 64;

/**
 * Create an empty storage instance.
 */
export function createStorage(): Storage {
	return {
		nameRegistry: [],
		flushedNameCount: 0,
		history: { entries: new Map() },
		entryMetadata: new Map(),
		output: undefined,
		state: "pending",
		flushedState: undefined,
		error: undefined,
		flushedError: undefined,
		flushedOutput: undefined,
	};
}

/**
 * Create a snapshot of workflow history for observers.
 */
export function createHistorySnapshot(
	storage: Storage,
): WorkflowHistorySnapshot {
	const entryMetadata = new Map<string, WorkflowEntryMetadataSnapshot>();
	for (const [id, metadata] of storage.entryMetadata) {
		const { dirty, ...rest } = metadata;
		entryMetadata.set(id, rest);
	}

	const entries: WorkflowHistoryEntry[] = [];
	const entryKeys = Array.from(storage.history.entries.keys()).sort();
	for (const key of entryKeys) {
		const entry = storage.history.entries.get(key);
		if (!entry) continue;
		const { dirty, ...rest } = entry;
		entries.push(rest);
	}

	return {
		nameRegistry: [...storage.nameRegistry],
		entries,
		entryMetadata,
	};
}

/**
 * Generate a UUID v4.
 */
export function generateId(): string {
	return crypto.randomUUID();
}

/**
 * Create a new entry.
 */
export function createEntry(location: Location, kind: EntryKind): Entry {
	return {
		id: generateId(),
		location,
		kind,
		dirty: true,
	};
}

/**
 * Create or get metadata for an entry.
 */
export function getOrCreateMetadata(
	storage: Storage,
	entryId: string,
): EntryMetadata {
	let metadata = storage.entryMetadata.get(entryId);
	if (!metadata) {
		metadata = {
			status: "pending",
			attempts: 0,
			lastAttemptAt: 0,
			createdAt: Date.now(),
			rollbackCompletedAt: undefined,
			rollbackError: undefined,
			dirty: true,
		};
		storage.entryMetadata.set(entryId, metadata);
	}
	return metadata;
}

/**
 * Load storage from the driver.
 */
export async function loadStorage(driver: EngineDriver): Promise<Storage> {
	const storage = createStorage();

	// Load name registry
	const nameEntries = await driver.list(buildNamePrefix());
	// Sort by index to ensure correct order
	nameEntries.sort((a, b) => compareKeys(a.key, b.key));
	for (const entry of nameEntries) {
		const index = parseNameKey(entry.key);
		storage.nameRegistry[index] = deserializeName(entry.value);
	}
	// Track how many names are already persisted
	storage.flushedNameCount = storage.nameRegistry.length;

	// Load history entries
	const historyEntries = await driver.list(buildHistoryPrefixAll());
	for (const entry of historyEntries) {
		const parsed = deserializeEntry(entry.value);
		parsed.dirty = false;
		// Use locationToKey to match how context.ts looks up entries
		const key = locationToKey(storage, parsed.location);
		storage.history.entries.set(key, parsed);
	}

	// Load entry metadata so observers can reconstruct workflow state after
	// the actor wakes and rebuilds storage from persisted history.
	const metadataEntries = await driver.list(buildEntryMetadataPrefix());
	for (const entry of metadataEntries) {
		const entryId = parseEntryMetadataKey(entry.key);
		const metadata = deserializeEntryMetadata(entry.value);
		metadata.dirty = false;
		storage.entryMetadata.set(entryId, metadata);
	}

	// Load workflow state
	const stateValue = await driver.get(buildWorkflowStateKey());
	if (stateValue) {
		storage.state = deserializeWorkflowState(stateValue);
		storage.flushedState = storage.state;
	}

	// Load output if present
	const outputValue = await driver.get(buildWorkflowOutputKey());
	if (outputValue) {
		storage.output = deserializeWorkflowOutput(outputValue);
		storage.flushedOutput = storage.output;
	}

	// Load error if present
	const errorValue = await driver.get(buildWorkflowErrorKey());
	if (errorValue) {
		storage.error = deserializeWorkflowError(errorValue);
		storage.flushedError = storage.error;
	}

	return storage;
}

/**
 * Load metadata for an entry (lazy loading).
 */
export async function loadMetadata(
	storage: Storage,
	driver: EngineDriver,
	entryId: string,
): Promise<EntryMetadata> {
	// Check if already loaded
	const existing = storage.entryMetadata.get(entryId);
	if (existing) {
		return existing;
	}

	// Load from driver
	const value = await driver.get(buildEntryMetadataKey(entryId));
	if (value) {
		const metadata = deserializeEntryMetadata(value);
		metadata.dirty = false;
		storage.entryMetadata.set(entryId, metadata);
		return metadata;
	}

	// Create new metadata
	return getOrCreateMetadata(storage, entryId);
}

/**
 * Pending deletions collected by collectLoopPruning to be included
 * in the next flush alongside the state write.
 */
export interface PendingDeletions {
	prefixes: Uint8Array[];
	keys: Uint8Array[];
	ranges: { start: Uint8Array; end: Uint8Array }[];
}

interface FlushQueue {
	/** Settles when every save started or queued so far has finished. */
	tail: Promise<void>;
	/** The next save, while it has not started. Other callers can join it. */
	waiting?: Promise<void>;
}

// Saves of one workflow run one at a time, so an older snapshot never lands
// after a newer one.
const flushQueues = new WeakMap<Storage, FlushQueue>();

/**
 * Save workflow history in order, preserving mutations made during each write.
 *
 * A save that has not started reads storage when it starts, so it also saves
 * the changes of callers that join it. They share its result. Saves with
 * deletions do not share, so pruning runs in order with the writes around it.
 */
export function flush(
	storage: Storage,
	driver: EngineDriver,
	onHistoryUpdated?: () => void,
	pendingDeletions?: PendingDeletions,
): Promise<void> {
	let queue = flushQueues.get(storage);
	if (!queue) {
		queue = { tail: Promise.resolve() };
		flushQueues.set(storage, queue);
	}
	if (queue.waiting && !pendingDeletions) {
		return queue.waiting;
	}

	const activeQueue = queue;
	const next: Promise<void> = activeQueue.tail.then(() => {
		if (activeQueue.waiting === next) {
			activeQueue.waiting = undefined;
		}
		return flushSnapshot(
			storage,
			driver,
			onHistoryUpdated,
			pendingDeletions,
		);
	});
	if (!pendingDeletions) {
		activeQueue.waiting = next;
	}
	activeQueue.tail = next.catch(() => undefined);
	return next;
}

/**
 * Flush all dirty data to the driver. Optionally includes pending
 * deletions so that history pruning happens alongside the
 * state write.
 */
async function flushSnapshot(
	storage: Storage,
	driver: EngineDriver,
	onHistoryUpdated?: () => void,
	pendingDeletions?: PendingDeletions,
): Promise<void> {
	const writes: KVWrite[] = [];
	const dirtyEntries: Entry[] = [];
	const dirtyMetadata: EntryMetadata[] = [];
	let historyUpdated = false;

	// Flush only new names (those added since last flush)
	for (
		let i = storage.flushedNameCount;
		i < storage.nameRegistry.length;
		i++
	) {
		const name = storage.nameRegistry[i];
		if (name !== undefined) {
			writes.push({
				key: buildNameKey(i),
				value: serializeName(name),
			});
			historyUpdated = true;
		}
	}

	// Flush dirty entries
	for (const [, entry] of storage.history.entries) {
		if (entry.dirty) {
			writes.push({
				key: buildHistoryKey(entry.location),
				value: serializeEntry(entry),
			});
			dirtyEntries.push(entry);
			historyUpdated = true;
		}
	}

	// Flush dirty metadata
	for (const [id, metadata] of storage.entryMetadata) {
		if (metadata.dirty) {
			writes.push({
				key: buildEntryMetadataKey(id),
				value: serializeEntryMetadata(metadata),
			});
			dirtyMetadata.push(metadata);
			historyUpdated = true;
		}
	}

	// Flush workflow state if changed
	if (storage.state !== storage.flushedState) {
		writes.push({
			key: buildWorkflowStateKey(),
			value: serializeWorkflowState(storage.state),
		});
	}

	// Flush output if changed
	if (
		storage.output !== undefined &&
		storage.output !== storage.flushedOutput
	) {
		writes.push({
			key: buildWorkflowOutputKey(),
			value: serializeWorkflowOutput(storage.output),
		});
	}

	// Flush error if changed (compare by message since objects aren't reference-equal)
	const errorChanged =
		storage.error !== undefined &&
		(storage.flushedError === undefined ||
			storage.error.name !== storage.flushedError.name ||
			storage.error.message !== storage.flushedError.message);
	if (errorChanged) {
		writes.push({
			key: buildWorkflowErrorKey(),
			value: serializeWorkflowError(storage.error!),
		});
	}

	// Record what this write contains before awaiting it. A concurrent branch
	// can change storage during the write, and those changes must stay dirty.
	const savedNameCount = storage.nameRegistry.length;
	const savedState = storage.state;
	const savedOutput = storage.output;
	const savedError = storage.error;
	for (const entry of dirtyEntries) {
		entry.dirty = false;
	}
	for (const metadata of dirtyMetadata) {
		metadata.dirty = false;
	}
	try {
		if (writes.length > 0) {
			if (driver.atomicBatch) {
				await driver.batch(writes);
			} else {
				for (const chunk of splitBatchWrites(writes)) {
					await driver.batch(chunk);
				}
			}
		}

		// Apply pending deletions after the batch write. These are collected
		// by collectLoopPruning so pruning happens alongside the state write.
		if (pendingDeletions) {
			const didChange = await runDeletes(driver, pendingDeletions);
			if (didChange) {
				historyUpdated = true;
			}
		}
	} catch (error) {
		for (const entry of dirtyEntries) {
			entry.dirty = true;
		}
		for (const metadata of dirtyMetadata) {
			metadata.dirty = true;
		}
		throw error;
	}

	storage.flushedNameCount = savedNameCount;
	storage.flushedState = savedState;
	storage.flushedOutput = savedOutput;
	storage.flushedError = savedError;

	if (historyUpdated && onHistoryUpdated) {
		onHistoryUpdated();
	}
}

function splitBatchWrites(writes: KVWrite[]): KVWrite[][] {
	const chunks: KVWrite[][] = [];
	let chunk: KVWrite[] = [];
	let chunkBytes = 0;

	for (const write of writes) {
		const writeBytes = write.key.byteLength + write.value.byteLength;
		if (writeBytes > MAX_KV_BATCH_PAYLOAD_BYTES) {
			throw new Error(
				`KV batch write is ${writeBytes} bytes, exceeding the ${MAX_KV_BATCH_PAYLOAD_BYTES} byte limit`,
			);
		}

		if (
			chunk.length >= MAX_KV_BATCH_ENTRIES ||
			(chunk.length > 0 &&
				chunkBytes + writeBytes > MAX_KV_BATCH_PAYLOAD_BYTES)
		) {
			chunks.push(chunk);
			chunk = [];
			chunkBytes = 0;
		}

		chunk.push(write);
		chunkBytes += writeBytes;
	}

	if (chunk.length > 0) {
		chunks.push(chunk);
	}

	return chunks;
}

/**
 * Split delete keys into batches within one KV transaction (MAX_KV_BATCH_ENTRIES).
 */
function splitBatchDeletes(keys: Uint8Array[]): Uint8Array[][] {
	const chunks: Uint8Array[][] = [];
	for (let i = 0; i < keys.length; i += MAX_KV_BATCH_ENTRIES) {
		chunks.push(keys.slice(i, i + MAX_KV_BATCH_ENTRIES));
	}
	return chunks;
}

/**
 * Apply deletions concurrently in bounded rounds; returns whether anything was deleted.
 */
async function runDeletes(
	driver: EngineDriver,
	deletions: PendingDeletions,
): Promise<boolean> {
	const metadataOps = splitBatchDeletes(deletions.keys).map(
		(chunk) => () => driver.batchDelete(chunk),
	);
	const historyOps = [
		...deletions.prefixes.map(
			(prefix) => () => driver.deletePrefix(prefix),
		),
		...deletions.ranges.map(
			(range) => () => driver.deleteRange(range.start, range.end),
		),
	];
	// Keep history discoverable until its metadata is removed, including after a restart.
	for (const ops of [metadataOps, historyOps]) {
		for (let i = 0; i < ops.length; i += MAX_CONCURRENT_DELETES) {
			const results = await Promise.allSettled(
				ops
					.slice(i, i + MAX_CONCURRENT_DELETES)
					.map((op) => Promise.resolve().then(op)),
			);
			const failed = results.find(
				(result) => result.status === "rejected",
			);
			if (failed?.status === "rejected") throw failed.reason;
		}
	}
	return metadataOps.length > 0 || historyOps.length > 0;
}

/**
 * Remove the entries under a location prefix from memory, and return the
 * deletions that remove them from the driver. Pass the result to `flush`, so
 * the deletions run only after the writes that must land first.
 */
export function takeEntriesWithPrefix(
	storage: Storage,
	prefixLocation: Location,
): PendingDeletions {
	const metadataKeys: Uint8Array[] = [];
	for (const [key, entry] of [...storage.history.entries]) {
		if (!isLocationPrefix(prefixLocation, entry.location)) {
			continue;
		}
		metadataKeys.push(buildEntryMetadataKey(entry.id));
		storage.entryMetadata.delete(entry.id);
		storage.history.entries.delete(key);
	}
	return {
		prefixes: [buildHistoryPrefix(prefixLocation)],
		keys: metadataKeys,
		ranges: [],
	};
}

/**
 * Delete entries with a given location prefix (used for loop forgetting).
 * Also cleans up associated metadata from both memory and driver.
 */
export async function deleteEntriesWithPrefix(
	storage: Storage,
	driver: EngineDriver,
	prefixLocation: Location,
	onHistoryUpdated?: () => void,
): Promise<void> {
	const entries = [...storage.history.entries].filter(([, entry]) =>
		isLocationPrefix(prefixLocation, entry.location),
	);
	await runDeletes(driver, {
		prefixes: [buildHistoryPrefix(prefixLocation)],
		keys: entries.map(([, entry]) => buildEntryMetadataKey(entry.id)),
		ranges: [],
	});
	for (const [key, entry] of entries) {
		if (storage.history.entries.get(key) === entry) {
			storage.entryMetadata.delete(entry.id);
			storage.history.entries.delete(key);
		}
	}
	if (entries.length > 0 && onHistoryUpdated) onHistoryUpdated();
}

/**
 * Get an entry by location.
 */
export function getEntry(
	storage: Storage,
	location: Location,
): Entry | undefined {
	const key = locationToKey(storage, location);
	return storage.history.entries.get(key);
}

/**
 * Set an entry by location.
 */
export function setEntry(
	storage: Storage,
	location: Location,
	entry: Entry,
): void {
	const key = locationToKey(storage, location);
	storage.history.entries.set(key, entry);
}

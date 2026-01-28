import {
	deserializeEntry,
	deserializeEntryMetadata,
	deserializeMessage,
	deserializeName,
	deserializeWorkflowError,
	deserializeWorkflowOutput,
	deserializeWorkflowState,
	serializeEntry,
	serializeEntryMetadata,
	serializeMessage,
	serializeName,
	serializeWorkflowError,
	serializeWorkflowOutput,
	serializeWorkflowState,
} from "../schemas/serde.js";
import type { EngineDriver, KVWrite } from "./driver.js";
import {
	buildEntryMetadataKey,
	buildHistoryKey,
	buildHistoryPrefix,
	buildHistoryPrefixAll,
	buildMessageKey,
	buildMessagePrefix,
	buildNameKey,
	buildNamePrefix,
	buildWorkflowErrorKey,
	buildWorkflowOutputKey,
	buildWorkflowStateKey,
	compareKeys,
	parseNameKey,
} from "./keys.js";
import { isLocationPrefix, locationToKey } from "./location.js";
import type {
	Entry,
	EntryKind,
	EntryMetadata,
	Location,
	Message,
	Storage,
} from "./types.js";

/**
 * Create an empty storage instance.
 */
export function createStorage(): Storage {
	return {
		nameRegistry: [],
		flushedNameCount: 0,
		history: { entries: new Map() },
		entryMetadata: new Map(),
		messages: [],
		output: undefined,
		state: "pending",
		flushedState: undefined,
		error: undefined,
		flushedError: undefined,
		flushedOutput: undefined,
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

	// Load messages
	const messageEntries = await driver.list(buildMessagePrefix());
	// Sort by index to ensure correct FIFO order
	messageEntries.sort((a, b) => compareKeys(a.key, b.key));
	for (const entry of messageEntries) {
		const message = deserializeMessage(entry.value);
		storage.messages.push(message);
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
 * Flush all dirty data to the driver.
 */
export async function flush(
	storage: Storage,
	driver: EngineDriver,
): Promise<void> {
	const writes: KVWrite[] = [];

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
		}
	}

	// Flush dirty entries
	for (const [, entry] of storage.history.entries) {
		if (entry.dirty) {
			writes.push({
				key: buildHistoryKey(entry.location),
				value: serializeEntry(entry),
			});
			entry.dirty = false;
		}
	}

	// Flush dirty metadata
	for (const [id, metadata] of storage.entryMetadata) {
		if (metadata.dirty) {
			writes.push({
				key: buildEntryMetadataKey(id),
				value: serializeEntryMetadata(metadata),
			});
			metadata.dirty = false;
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

	if (writes.length > 0) {
		await driver.batch(writes);
	}

	// Update flushed tracking after successful write
	storage.flushedNameCount = storage.nameRegistry.length;
	storage.flushedState = storage.state;
	storage.flushedOutput = storage.output;
	storage.flushedError = storage.error;
}

/**
 * Add a message to the queue.
 */
export async function addMessage(
	storage: Storage,
	driver: EngineDriver,
	name: string,
	data: unknown,
): Promise<void> {
	const message: Message = {
		id: generateId(),
		name,
		data,
		sentAt: Date.now(),
	};

	storage.messages.push(message);

	// Persist immediately using message's unique ID as key
	await driver.set(buildMessageKey(message.id), serializeMessage(message));
}

/**
 * Consume a message from the queue.
 * Returns null if no matching message is found.
 * Deletes from driver first to prevent duplicates on failure.
 */
export async function consumeMessage(
	storage: Storage,
	driver: EngineDriver,
	messageName: string,
): Promise<Message | null> {
	const index = storage.messages.findIndex(
		(message) => message.name === messageName,
	);
	if (index === -1) {
		return null;
	}

	const message = storage.messages[index];

	// Delete from driver first - if this fails, memory is unchanged
	await driver.delete(buildMessageKey(message.id));

	// Only remove from memory after successful driver deletion
	storage.messages.splice(index, 1);

	return message;
}

/**
 * Consume up to N messages from the queue.
 *
 * Uses allSettled to handle partial failures gracefully:
 * - If all deletes succeed, messages are removed from memory
 * - If some deletes fail, only successfully deleted messages are removed
 * - On next load, failed messages will be re-read from KV
 */
export async function consumeMessages(
	storage: Storage,
	driver: EngineDriver,
	messageName: string,
	limit: number,
): Promise<Message[]> {
	// Find all matching messages up to limit (don't modify memory yet)
	const toConsume: { message: Message; index: number }[] = [];
	let count = 0;

	for (let i = 0; i < storage.messages.length && count < limit; i++) {
		if (storage.messages[i].name === messageName) {
			toConsume.push({ message: storage.messages[i], index: i });
			count++;
		}
	}

	if (toConsume.length === 0) {
		return [];
	}

	// Delete from driver using allSettled to handle partial failures
	const deleteResults = await Promise.allSettled(
		toConsume.map(({ message }) =>
			driver.delete(buildMessageKey(message.id)),
		),
	);

	// Track which messages were successfully deleted
	const successfullyDeleted: { message: Message; index: number }[] = [];
	for (let i = 0; i < deleteResults.length; i++) {
		if (deleteResults[i].status === "fulfilled") {
			successfullyDeleted.push(toConsume[i]);
		}
	}

	// Only remove successfully deleted messages from memory
	// Remove in reverse order to preserve indices
	for (let i = successfullyDeleted.length - 1; i >= 0; i--) {
		const { index } = successfullyDeleted[i];
		storage.messages.splice(index, 1);
	}

	return successfullyDeleted.map(({ message }) => message);
}

/**
 * Delete entries with a given location prefix (used for loop forgetting).
 * Also cleans up associated metadata from both memory and driver.
 */
export async function deleteEntriesWithPrefix(
	storage: Storage,
	driver: EngineDriver,
	prefixLocation: Location,
): Promise<void> {
	// Collect entry IDs for metadata cleanup
	const entryIds: string[] = [];

	// Collect entries to delete and their IDs
	for (const [key, entry] of storage.history.entries) {
		// Check if the entry's location starts with the prefix location
		if (isLocationPrefix(prefixLocation, entry.location)) {
			entryIds.push(entry.id);
			storage.entryMetadata.delete(entry.id);
			storage.history.entries.delete(key);
		}
	}

	// Delete entries from driver using binary prefix
	await driver.deletePrefix(buildHistoryPrefix(prefixLocation));

	// Delete metadata from driver in parallel
	await Promise.all(
		entryIds.map((id) => driver.delete(buildEntryMetadataKey(id))),
	);
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

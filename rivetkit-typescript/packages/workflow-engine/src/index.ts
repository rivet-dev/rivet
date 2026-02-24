import type { Logger } from "pino";

// Types

// Context
export {
	DEFAULT_LOOP_COMMIT_INTERVAL,
	DEFAULT_LOOP_HISTORY_EVERY,
	DEFAULT_LOOP_HISTORY_KEEP,
	DEFAULT_MAX_RETRIES,
	DEFAULT_RETRY_BACKOFF_BASE,
	DEFAULT_RETRY_BACKOFF_MAX,
	DEFAULT_STEP_TIMEOUT,
	WorkflowContextImpl,
} from "./context.js";
// Driver
export type { EngineDriver, KVEntry, KVWrite } from "./driver.js";
// Errors
export {
	CancelledError,
	CriticalError,
	EntryInProgressError,
	EvictedError,
	HistoryDivergedError,
	JoinError,
	MessageWaitError,
	RaceError,
	RollbackCheckpointError,
	RollbackError,
	SleepError,
	StepExhaustedError,
	StepFailedError,
} from "./errors.js";

// Location utilities
export {
	appendLoopIteration,
	appendName,
	emptyLocation,
	isLocationPrefix,
	isLoopIterationMarker,
	locationsEqual,
	locationToKey,
	parentLocation,
	registerName,
	resolveName,
} from "./location.js";

// Storage utilities
export {
	createEntry,
	createHistorySnapshot,
	createStorage,
	deleteEntriesWithPrefix,
	flush,
	generateId,
	getEntry,
	getOrCreateMetadata,
	loadMetadata,
	loadStorage,
	setEntry,
} from "./storage.js";
export type {
	BranchConfig,
	BranchOutput,
	BranchStatus,
	BranchStatusType,
	Entry,
	EntryKind,
	EntryKindType,
	EntryMetadata,
	EntryStatus,
	History,
	JoinEntry,
	Location,
	LoopConfig,
	LoopEntry,
	LoopIterationMarker,
	LoopResult,
	Message,
	MessageEntry,
	NameIndex,
	PathSegment,
	RaceEntry,
	RemovedEntry,
	WorkflowEntryMetadataSnapshot,
	WorkflowHistoryEntry,
	WorkflowHistorySnapshot,
	RollbackCheckpointEntry,
	RollbackContextInterface,
	RunWorkflowOptions,
	SleepEntry,
	SleepState,
	StepConfig,
	StepEntry,
	Storage,
	WorkflowContextInterface,
	WorkflowFunction,
	WorkflowHandle,
	WorkflowQueue,
	WorkflowQueueMessage,
	WorkflowQueueNextOptions,
	WorkflowMessageDriver,
	WorkflowResult,
	WorkflowRunMode,
	WorkflowState,
} from "./types.js";

// Loop result helpers
export const Loop = {
	continue: <S>(state: S): { continue: true; state: S } => ({
		continue: true,
		state,
	}),
	break: <T>(value: T): { break: true; value: T } => ({
		break: true,
		value,
	}),
};

import {
	deserializeEntryMetadata,
	deserializeWorkflowInput,
	deserializeWorkflowOutput,
	deserializeWorkflowState,
	serializeEntryMetadata,
	serializeWorkflowInput,
	serializeWorkflowState,
} from "../schemas/serde.js";
import { type RollbackAction, WorkflowContextImpl } from "./context.js";
// Main workflow runner
import type { EngineDriver } from "./driver.js";
import {
	EvictedError,
	MessageWaitError,
	RollbackCheckpointError,
	RollbackStopError,
	SleepError,
	StepFailedError,
} from "./errors.js";
import {
	buildEntryMetadataPrefix,
	buildWorkflowErrorKey,
	buildWorkflowInputKey,
	buildWorkflowOutputKey,
	buildWorkflowStateKey,
} from "./keys.js";
import {
	createHistorySnapshot,
	flush,
	generateId,
	loadMetadata,
	loadStorage,
} from "./storage.js";
import type {
	RollbackContextInterface,
	RunWorkflowOptions,
	Storage,
	WorkflowHistorySnapshot,
	WorkflowFunction,
	WorkflowHandle,
	WorkflowMessageDriver,
	WorkflowResult,
	WorkflowRunMode,
	WorkflowState,
} from "./types.js";
import { setLongTimeout } from "./utils.js";

/**
 * Run a workflow and return a handle for managing it.
 *
 * The workflow starts executing immediately. Use the returned handle to:
 * - `handle.result` - Await workflow completion (or yield in `yield` mode)
 * - `handle.message()` - Send messages to the workflow
 * - `handle.wake()` - Wake the workflow early
 * - `handle.evict()` - Request graceful shutdown
 * - `handle.getOutput()` / `handle.getState()` - Query status
 */
interface LiveRuntime {
	sleepWaiter?: () => void;
	isSleeping: boolean;
}

type HistoryNotifier = (() => void) | undefined;

function createLiveRuntime(): LiveRuntime {
	return {
		isSleeping: false,
	};
}

function createEvictionWait(signal: AbortSignal): {
	promise: Promise<never>;
	cleanup: () => void;
} {
	if (signal.aborted) {
		return {
			promise: Promise.reject(new EvictedError()),
			cleanup: () => {},
		};
	}

	let onAbort: (() => void) | undefined;
	const promise = new Promise<never>((_, reject) => {
		onAbort = () => {
			reject(new EvictedError());
		};
		signal.addEventListener("abort", onAbort, { once: true });
	});

	return {
		promise,
		cleanup: () => {
			if (onAbort) {
				signal.removeEventListener("abort", onAbort);
			}
		},
	};
}

function createRollbackContext(
	workflowId: string,
	abortController: AbortController,
): RollbackContextInterface {
	return {
		workflowId,
		abortSignal: abortController.signal,
		isEvicted: () => abortController.signal.aborted,
	};
}

async function awaitWithEviction<T>(
	promise: Promise<T>,
	abortSignal: AbortSignal,
): Promise<T> {
	const { promise: evictionPromise, cleanup } =
		createEvictionWait(abortSignal);
	try {
		return await Promise.race([promise, evictionPromise]);
	} finally {
		cleanup();
	}
}

async function executeRollback<TInput, TOutput>(
	workflowId: string,
	workflowFn: WorkflowFunction<TInput, TOutput>,
	input: TInput,
	driver: EngineDriver,
	messageDriver: WorkflowMessageDriver,
	abortController: AbortController,
	storage: Storage,
	historyNotifier?: HistoryNotifier,
	logger?: Logger,
): Promise<void> {
	const rollbackActions: RollbackAction[] = [];
	const ctx = new WorkflowContextImpl(
		workflowId,
		storage,
		driver,
		messageDriver,
		undefined,
		abortController,
		"rollback",
		rollbackActions,
		false,
		historyNotifier,
		logger,
	);

	try {
		await workflowFn(ctx, input);
	} catch (error) {
		if (error instanceof EvictedError) {
			throw error;
		}
		if (error instanceof RollbackStopError) {
			// Stop replay once we hit incomplete history during rollback.
		} else {
			// Ignore workflow errors during rollback replay.
		}
	}

	if (rollbackActions.length === 0) {
		return;
	}

	const rollbackContext = createRollbackContext(workflowId, abortController);

	for (let i = rollbackActions.length - 1; i >= 0; i--) {
		if (abortController.signal.aborted) {
			throw new EvictedError();
		}

		const action = rollbackActions[i];
		const metadata = await loadMetadata(storage, driver, action.entryId);
		if (metadata.rollbackCompletedAt !== undefined) {
			continue;
		}

		try {
			await awaitWithEviction(
				action.rollback(rollbackContext, action.output),
				abortController.signal,
			);
			metadata.rollbackCompletedAt = Date.now();
			metadata.rollbackError = undefined;
		} catch (error) {
			if (error instanceof EvictedError) {
				throw error;
			}
			metadata.rollbackError =
				error instanceof Error ? error.message : String(error);
			throw error;
		} finally {
			metadata.dirty = true;
			await flush(storage, driver, historyNotifier);
		}
	}
}

async function setSleepState<TOutput>(
	storage: Storage,
	driver: EngineDriver,
	workflowId: string,
	deadline: number,
	messageNames?: string[],
	historyNotifier?: HistoryNotifier,
): Promise<WorkflowResult<TOutput>> {
	storage.state = "sleeping";
	await flush(storage, driver, historyNotifier);
	await driver.setAlarm(workflowId, deadline);

	return {
		state: "sleeping",
		sleepUntil: deadline,
		waitingForMessages: messageNames,
	};
}

async function setMessageWaitState<TOutput>(
	storage: Storage,
	driver: EngineDriver,
	messageNames: string[],
	historyNotifier?: HistoryNotifier,
): Promise<WorkflowResult<TOutput>> {
	storage.state = "sleeping";
	await flush(storage, driver, historyNotifier);

	return { state: "sleeping", waitingForMessages: messageNames };
}

async function setEvictedState<TOutput>(
	storage: Storage,
	driver: EngineDriver,
	historyNotifier?: HistoryNotifier,
): Promise<WorkflowResult<TOutput>> {
	await flush(storage, driver, historyNotifier);
	return { state: storage.state };
}

async function setRetryState<TOutput>(
	storage: Storage,
	driver: EngineDriver,
	workflowId: string,
	historyNotifier?: HistoryNotifier,
): Promise<WorkflowResult<TOutput>> {
	storage.state = "sleeping";
	await flush(storage, driver, historyNotifier);

	const retryAt = Date.now() + 100;
	await driver.setAlarm(workflowId, retryAt);

	return { state: "sleeping", sleepUntil: retryAt };
}

async function setFailedState(
	storage: Storage,
	driver: EngineDriver,
	error: unknown,
	historyNotifier?: HistoryNotifier,
): Promise<void> {
	storage.state = "failed";
	storage.error = extractErrorInfo(error);
	await flush(storage, driver, historyNotifier);
}

async function waitForSleep(
	runtime: LiveRuntime,
	deadline: number,
	abortSignal: AbortSignal,
): Promise<void> {
	while (true) {
		const remaining = deadline - Date.now();
		if (remaining <= 0) {
			return;
		}

		let timeoutHandle: ReturnType<typeof setLongTimeout> | undefined;
		const timeoutPromise = new Promise<void>((resolve) => {
			timeoutHandle = setLongTimeout(resolve, remaining);
		});

		const wakePromise = new Promise<void>((resolve) => {
			runtime.sleepWaiter = resolve;
		});
		runtime.isSleeping = true;

		try {
			await awaitWithEviction(
				Promise.race([timeoutPromise, wakePromise]),
				abortSignal,
			);
		} finally {
			runtime.isSleeping = false;
			runtime.sleepWaiter = undefined;
			timeoutHandle?.abort();
		}

		if (abortSignal.aborted) {
			throw new EvictedError();
		}

		if (Date.now() >= deadline) {
			return;
		}
	}
}

async function executeLiveWorkflow<TInput, TOutput>(
	workflowId: string,
	workflowFn: WorkflowFunction<TInput, TOutput>,
	input: TInput,
	driver: EngineDriver,
	messageDriver: WorkflowMessageDriver,
	abortController: AbortController,
	runtime: LiveRuntime,
	onHistoryUpdated?: (history: WorkflowHistorySnapshot) => void,
	logger?: Logger,
): Promise<WorkflowResult<TOutput>> {
	let lastResult: WorkflowResult<TOutput> | undefined;

	while (true) {
		const result = await executeWorkflow(
			workflowId,
			workflowFn,
			input,
			driver,
			messageDriver,
			abortController,
			onHistoryUpdated,
			logger,
		);
		lastResult = result;

		if (result.state !== "sleeping") {
			return result;
		}

		const hasMessages = result.waitingForMessages !== undefined;
		const hasDeadline = result.sleepUntil !== undefined;

		if (hasMessages && hasDeadline) {
			// Wait for EITHER a message OR the deadline (for queue.next timeout)
			try {
				const messagePromise = awaitWithEviction(
					driver.waitForMessages(
						result.waitingForMessages!,
						abortController.signal,
					),
					abortController.signal,
				);
				const sleepPromise = waitForSleep(
					runtime,
					result.sleepUntil!,
					abortController.signal,
				);
				await Promise.race([messagePromise, sleepPromise]);
			} catch (error) {
				if (error instanceof EvictedError) {
					return lastResult;
				}
				throw error;
			}
			continue;
		}

		if (hasMessages) {
			try {
				await awaitWithEviction(
					driver.waitForMessages(
						result.waitingForMessages!,
						abortController.signal,
					),
					abortController.signal,
				);
			} catch (error) {
				if (error instanceof EvictedError) {
					return lastResult;
				}
				throw error;
			}
			continue;
		}

		if (hasDeadline) {
			try {
				await waitForSleep(
					runtime,
					result.sleepUntil!,
					abortController.signal,
				);
			} catch (error) {
				if (error instanceof EvictedError) {
					return lastResult;
				}
				throw error;
			}
			continue;
		}

		return result;
	}
}

export function runWorkflow<TInput, TOutput>(
	workflowId: string,
	workflowFn: WorkflowFunction<TInput, TOutput>,
	input: TInput,
	driver: EngineDriver,
	options: RunWorkflowOptions = {},
): WorkflowHandle<TOutput> {
	const messageDriver = driver.messageDriver;
	const abortController = new AbortController();
	const mode: WorkflowRunMode = options.mode ?? "yield";
	const liveRuntime = mode === "live" ? createLiveRuntime() : undefined;

	const logger = options.logger;

	const resultPromise =
		mode === "live" && liveRuntime
			? executeLiveWorkflow(
					workflowId,
					workflowFn,
					input,
					driver,
					messageDriver,
					abortController,
					liveRuntime,
					options.onHistoryUpdated,
					logger,
				)
			: executeWorkflow(
					workflowId,
					workflowFn,
					input,
					driver,
					messageDriver,
					abortController,
					options.onHistoryUpdated,
					logger,
				);

	return {
		workflowId,
		result: resultPromise,

		async message(name: string, data: unknown): Promise<void> {
			const messageId = generateId();
			await messageDriver.addMessage({
				id: messageId,
				name,
				data,
				sentAt: Date.now(),
			});
		},

		async wake(): Promise<void> {
			if (liveRuntime) {
				if (liveRuntime.isSleeping && liveRuntime.sleepWaiter) {
					liveRuntime.sleepWaiter();
				}
				return;
			}
			await driver.setAlarm(workflowId, Date.now());
		},

		async recover(): Promise<void> {
			const stateValue = await driver.get(buildWorkflowStateKey());
			const state = stateValue
				? deserializeWorkflowState(stateValue)
				: "pending";

			if (state !== "failed") {
				return;
			}

			const metadataEntries = await driver.list(
				buildEntryMetadataPrefix(),
			);
			const writes: { key: Uint8Array; value: Uint8Array }[] = [];

			for (const entry of metadataEntries) {
				const metadata = deserializeEntryMetadata(entry.value);
				if (
					metadata.status !== "failed" &&
					metadata.status !== "exhausted"
				) {
					continue;
				}

				metadata.status = "pending";
				metadata.attempts = 0;
				metadata.lastAttemptAt = 0;
				metadata.error = undefined;
				metadata.dirty = false;

				writes.push({
					key: entry.key,
					value: serializeEntryMetadata(metadata),
				});
			}

			if (writes.length > 0) {
				await driver.batch(writes);
			}

			await driver.delete(buildWorkflowErrorKey());
			await driver.set(
				buildWorkflowStateKey(),
				serializeWorkflowState("sleeping"),
			);

			if (liveRuntime) {
				if (liveRuntime.isSleeping && liveRuntime.sleepWaiter) {
					liveRuntime.sleepWaiter();
				}
				return;
			}

			await driver.setAlarm(workflowId, Date.now());
		},

		evict(): void {
			abortController.abort(new EvictedError());
		},

		async cancel(): Promise<void> {
			abortController.abort(new EvictedError());

			await driver.set(
				buildWorkflowStateKey(),
				serializeWorkflowState("cancelled"),
			);

			await driver.clearAlarm(workflowId);
		},

		async getOutput(): Promise<TOutput | undefined> {
			const value = await driver.get(buildWorkflowOutputKey());
			if (!value) {
				return undefined;
			}
			return deserializeWorkflowOutput<TOutput>(value);
		},

		async getState(): Promise<WorkflowState> {
			const value = await driver.get(buildWorkflowStateKey());
			if (!value) {
				return "pending";
			}
			return deserializeWorkflowState(value);
		},
	};
}

/**
 * Internal: Execute the workflow and return the result.
 */
async function executeWorkflow<TInput, TOutput>(
	workflowId: string,
	workflowFn: WorkflowFunction<TInput, TOutput>,
	input: TInput,
	driver: EngineDriver,
	messageDriver: WorkflowMessageDriver,
	abortController: AbortController,
	onHistoryUpdated?: (history: WorkflowHistorySnapshot) => void,
	logger?: Logger,
): Promise<WorkflowResult<TOutput>> {
	const storage = await loadStorage(driver);
	const historyNotifier: HistoryNotifier = onHistoryUpdated
		? () => onHistoryUpdated(createHistorySnapshot(storage))
		: undefined;
	if (historyNotifier) {
		historyNotifier();
	}

	if (logger) {
		const entryKeys = Array.from(storage.history.entries.keys());
		logger.debug({
			msg: "loaded workflow storage",
			state: storage.state,
			entryCount: entryKeys.length,
			entries: entryKeys.slice(0, 10),
			nameRegistry: storage.nameRegistry,
		});
	}

	// Check if workflow was cancelled
	if (storage.state === "cancelled") {
		throw new EvictedError();
	}

	// Input persistence: store on first run, use stored input on resume
	const storedInputBytes = await driver.get(buildWorkflowInputKey());
	let effectiveInput: TInput;

	if (storedInputBytes) {
		// Resume: use stored input for deterministic replay
		effectiveInput = deserializeWorkflowInput<TInput>(storedInputBytes);
	} else {
		// First run: store the input
		effectiveInput = input;
		await driver.set(
			buildWorkflowInputKey(),
			serializeWorkflowInput(input),
		);
	}

	if (storage.state === "rolling_back") {
		try {
			await executeRollback(
				workflowId,
				workflowFn,
				effectiveInput,
				driver,
				messageDriver,
				abortController,
				storage,
				historyNotifier,
				logger,
			);
		} catch (error) {
			if (error instanceof EvictedError) {
				return { state: storage.state };
			}
			throw error;
		}

		storage.state = "failed";
		await flush(storage, driver, historyNotifier);

		const storedError = storage.error
			? new Error(storage.error.message)
			: new Error("Workflow failed");
		if (storage.error?.name) {
			storedError.name = storage.error.name;
		}
		throw storedError;
	}

	const ctx = new WorkflowContextImpl(
		workflowId,
		storage,
		driver,
		messageDriver,
		undefined,
		abortController,
		"forward",
		undefined,
		false,
		historyNotifier,
		logger,
	);

	storage.state = "running";

	try {
		const output = await workflowFn(ctx, effectiveInput);

		storage.state = "completed";
		storage.output = output;
		await flush(storage, driver, historyNotifier);
		await driver.clearAlarm(workflowId);

		return { state: "completed", output };
	} catch (error) {
		if (error instanceof SleepError) {
			return await setSleepState(
				storage,
				driver,
				workflowId,
				error.deadline,
				error.messageNames,
				historyNotifier,
			);
		}

		if (error instanceof MessageWaitError) {
			return await setMessageWaitState(
				storage,
				driver,
				error.messageNames,
				historyNotifier,
			);
		}

		if (error instanceof EvictedError) {
			return await setEvictedState(storage, driver, historyNotifier);
		}

		if (error instanceof StepFailedError) {
			return await setRetryState(
				storage,
				driver,
				workflowId,
				historyNotifier,
			);
		}

		if (error instanceof RollbackCheckpointError) {
			await setFailedState(storage, driver, error, historyNotifier);
			throw error;
		}

		// Unrecoverable error
		storage.error = extractErrorInfo(error);
		storage.state = "rolling_back";
		await flush(storage, driver, historyNotifier);

		try {
			await executeRollback(
				workflowId,
				workflowFn,
				effectiveInput,
				driver,
				messageDriver,
				abortController,
				storage,
				historyNotifier,
				logger,
			);
		} catch (rollbackError) {
			if (rollbackError instanceof EvictedError) {
				return { state: storage.state };
			}
			throw rollbackError;
		}

		storage.state = "failed";
		await flush(storage, driver, historyNotifier);

		throw error;
	}
}

/**
 * Extract structured error information from an error.
 */
function extractErrorInfo(error: unknown): {
	name: string;
	message: string;
	stack?: string;
	metadata?: Record<string, unknown>;
} {
	if (error instanceof Error) {
		const result: {
			name: string;
			message: string;
			stack?: string;
			metadata?: Record<string, unknown>;
		} = {
			name: error.name,
			message: error.message,
			stack: error.stack,
		};

		// Extract custom properties from error
		const metadata: Record<string, unknown> = {};
		for (const key of Object.keys(error)) {
			if (key !== "name" && key !== "message" && key !== "stack") {
				const value = (error as unknown as Record<string, unknown>)[
					key
				];
				// Only include serializable values
				if (
					typeof value === "string" ||
					typeof value === "number" ||
					typeof value === "boolean" ||
					value === null
				) {
					metadata[key] = value;
				}
			}
		}
		if (Object.keys(metadata).length > 0) {
			result.metadata = metadata;
		}

		return result;
	}

	return {
		name: "Error",
		message: String(error),
	};
}

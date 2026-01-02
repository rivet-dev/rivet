// Types
export type {
	BranchConfig,
	BranchOutput,
	BranchStatus,
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
	NameIndex,
	PathSegment,
	RaceEntry,
	RemovedEntry,
	Signal,
	SignalEntry,
	SleepEntry,
	SleepState,
	StepConfig,
	StepEntry,
	Storage,
	WorkflowContextInterface,
	WorkflowFunction,
	WorkflowHandle,
	WorkflowResult,
	WorkflowState,
} from "./types.js";

// Errors
export {
	CancelledError,
	CriticalError,
	EntryInProgressError,
	EvictedError,
	HistoryDivergedError,
	JoinError,
	RaceError,
	SignalWaitError,
	SleepError,
	StepExhaustedError,
	StepFailedError,
} from "./errors.js";

// Driver
export type { EngineDriver, KVEntry, KVWrite } from "./driver.js";

// Location utilities
export {
	appendLoopIteration,
	appendName,
	emptyLocation,
	isLocationPrefix,
	isLoopIterationMarker,
	locationToKey,
	locationsEqual,
	parentLocation,
	registerName,
	resolveName,
} from "./location.js";

// Storage utilities
export {
	addSignal,
	consumeSignal,
	consumeSignals,
	createEntry,
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

// Context
export {
	WorkflowContextImpl,
	DEFAULT_MAX_RETRIES,
	DEFAULT_RETRY_BACKOFF_BASE,
	DEFAULT_RETRY_BACKOFF_MAX,
	DEFAULT_LOOP_COMMIT_INTERVAL,
	DEFAULT_STEP_TIMEOUT,
} from "./context.js";

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

// Main workflow runner
import type { EngineDriver } from "./driver.js";
import {
	EvictedError,
	SignalWaitError,
	SleepError,
	StepFailedError,
} from "./errors.js";
import {
	deserializeWorkflowInput,
	deserializeWorkflowOutput,
	deserializeWorkflowState,
	serializeSignal,
	serializeWorkflowInput,
	serializeWorkflowState,
} from "../schemas/serde.js";
import {
	buildSignalKey,
	buildSignalPrefix,
	buildWorkflowInputKey,
	buildWorkflowOutputKey,
	buildWorkflowStateKey,
} from "./keys.js";
import { flush, loadStorage } from "./storage.js";
import type {
	WorkflowFunction,
	WorkflowHandle,
	WorkflowResult,
	WorkflowState,
} from "./types.js";
import { WorkflowContextImpl } from "./context.js";
import { generateId } from "./storage.js";


/**
 * Run a workflow and return a handle for managing it.
 *
 * The workflow starts executing immediately. Use the returned handle to:
 * - `handle.result` - Await workflow completion or yield
 * - `handle.signal()` - Send signals to the workflow
 * - `handle.wake()` - Wake the workflow early
 * - `handle.evict()` - Request graceful shutdown
 * - `handle.getOutput()` / `handle.getState()` - Query status
 */
export function runWorkflow<TInput, TOutput>(
	workflowId: string,
	workflowFn: WorkflowFunction<TInput, TOutput>,
	input: TInput,
	driver: EngineDriver,
): WorkflowHandle<TOutput> {
	const abortController = new AbortController();

	// Start workflow execution (runs in background)
	const resultPromise = executeWorkflow(
		workflowId,
		workflowFn,
		input,
		driver,
		abortController,
	);

	return {
		workflowId,
		result: resultPromise,

		async signal(name: string, data: unknown): Promise<void> {
			// Use unique ID to avoid race conditions when signaling concurrently
			const signalId = generateId();
			await driver.set(
				buildSignalKey(signalId),
				serializeSignal({
					id: signalId,
					name,
					data,
					sentAt: Date.now(),
				}),
			);
		},

		async wake(): Promise<void> {
			await driver.setAlarm(workflowId, Date.now());
		},

		evict(): void {
			abortController.abort(new EvictedError());
		},

		async cancel(): Promise<void> {
			// Evict the workflow first
			abortController.abort(new EvictedError());

			// Set the workflow state to cancelled
			await driver.set(
				buildWorkflowStateKey(),
				serializeWorkflowState("cancelled"),
			);

			// Clear any pending alarms
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
	abortController: AbortController,
): Promise<WorkflowResult<TOutput>> {
	const storage = await loadStorage(driver);

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
		await driver.set(buildWorkflowInputKey(), serializeWorkflowInput(input));
	}

	const ctx = new WorkflowContextImpl(
		workflowId,
		storage,
		driver,
		undefined,
		abortController,
	);

	storage.state = "running";

	try {
		const output = await workflowFn(ctx, effectiveInput);

		storage.state = "completed";
		storage.output = output;
		await flush(storage, driver);
		await driver.clearAlarm(workflowId);

		return { state: "completed", output };
	} catch (error) {
		if (error instanceof SleepError) {
			storage.state = "sleeping";
			await flush(storage, driver);
			await driver.setAlarm(workflowId, error.deadline);

			return { state: "sleeping", sleepUntil: error.deadline };
		}

		if (error instanceof SignalWaitError) {
			storage.state = "sleeping";
			await flush(storage, driver);

			return { state: "sleeping", waitingForSignals: error.signalNames };
		}

		if (error instanceof EvictedError) {
			// Just save state, workflow will be resumed elsewhere
			await flush(storage, driver);
			return { state: storage.state };
		}

		if (error instanceof StepFailedError) {
			// Step failed but can be retried - yield to scheduler
			storage.state = "sleeping";
			await flush(storage, driver);

			// Set minimal alarm for retry (backoff is handled in executeStep)
			const retryAt = Date.now() + 100;
			await driver.setAlarm(workflowId, retryAt);

			return { state: "sleeping", sleepUntil: retryAt };
		}

		// Unrecoverable error
		storage.state = "failed";
		storage.error = extractErrorInfo(error);
		await flush(storage, driver);

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
				const value = (error as unknown as Record<string, unknown>)[key];
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

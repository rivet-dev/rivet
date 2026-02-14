import type { Logger } from "pino";
import type { EngineDriver } from "./driver.js";
import {
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
	RollbackStopError,
	SleepError,
	StepExhaustedError,
	StepFailedError,
} from "./errors.js";
import {
	appendLoopIteration,
	appendName,
	emptyLocation,
	locationToKey,
	registerName,
} from "./location.js";
import {
	consumeMessage,
	consumeMessages,
	createEntry,
	deleteEntriesWithPrefix,
	flush,
	getEntry,
	getOrCreateMetadata,
	loadMetadata,
	setEntry,
} from "./storage.js";
import type {
	BranchConfig,
	BranchOutput,
	BranchStatus,
	Entry,
	EntryKindType,
	EntryMetadata,
	Location,
	LoopConfig,
	LoopResult,
	Message,
	RollbackContextInterface,
	StepConfig,
	Storage,
	WorkflowContextInterface,
	WorkflowListenMessage,
	WorkflowMessageDriver,
} from "./types.js";
import { sleep } from "./utils.js";

/**
 * Default values for step configuration.
 * These are exported so users can reference them when overriding.
 */
export const DEFAULT_MAX_RETRIES = 3;
export const DEFAULT_RETRY_BACKOFF_BASE = 100;
export const DEFAULT_RETRY_BACKOFF_MAX = 30000;
export const DEFAULT_LOOP_COMMIT_INTERVAL = 20;
export const DEFAULT_LOOP_HISTORY_EVERY = 20;
export const DEFAULT_LOOP_HISTORY_KEEP = 20;
export const DEFAULT_STEP_TIMEOUT = 30000; // 30 seconds

const LISTEN_HISTORY_MESSAGE_MARKER = "__rivetWorkflowListenMessage";

/**
 * Calculate backoff delay with exponential backoff.
 * Uses deterministic calculation (no jitter) for replay consistency.
 */
function calculateBackoff(attempts: number, base: number, max: number): number {
	// Exponential backoff without jitter for determinism
	return Math.min(max, base * 2 ** attempts);
}

/**
 * Error thrown when a step times out.
 */
export class StepTimeoutError extends Error {
	constructor(
		public readonly stepName: string,
		public readonly timeoutMs: number,
	) {
		super(`Step "${stepName}" timed out after ${timeoutMs}ms`);
		this.name = "StepTimeoutError";
	}
}

/**
 * Internal representation of a rollback handler.
 */
export interface RollbackAction<T = unknown> {
	entryId: string;
	name: string;
	output: T;
	rollback: (ctx: RollbackContextInterface, output: T) => Promise<void>;
}

/**
 * Internal implementation of WorkflowContext.
 */
export class WorkflowContextImpl implements WorkflowContextInterface {
	private entryInProgress = false;
	private abortController: AbortController;
	private currentLocation: Location;
	private visitedKeys = new Set<string>();
	private mode: "forward" | "rollback";
	private rollbackActions?: RollbackAction[];
	private rollbackCheckpointSet: boolean;
	/** Track names used in current execution to detect duplicates */
	private usedNamesInExecution = new Set<string>();
	private historyNotifier?: () => void;
	private logger?: Logger;

	constructor(
		public readonly workflowId: string,
		private storage: Storage,
		private driver: EngineDriver,
		private messageDriver: WorkflowMessageDriver,
		location: Location = emptyLocation(),
		abortController?: AbortController,
		mode: "forward" | "rollback" = "forward",
		rollbackActions?: RollbackAction[],
		rollbackCheckpointSet = false,
		historyNotifier?: () => void,
		logger?: Logger,
	) {
		this.currentLocation = location;
		this.abortController = abortController ?? new AbortController();
		this.mode = mode;
		this.rollbackActions = rollbackActions;
		this.rollbackCheckpointSet = rollbackCheckpointSet;
		this.historyNotifier = historyNotifier;
		this.logger = logger;
	}

	get abortSignal(): AbortSignal {
		return this.abortController.signal;
	}

	isEvicted(): boolean {
		return this.abortSignal.aborted;
	}

	private assertNotInProgress(): void {
		if (this.entryInProgress) {
			throw new EntryInProgressError();
		}
	}

	private checkEvicted(): void {
		if (this.abortSignal.aborted) {
			throw new EvictedError();
		}
	}

	private async flushStorage(): Promise<void> {
		await flush(this.storage, this.driver, this.historyNotifier);
	}

	/**
	 * Create a new branch context for parallel/nested execution.
	 */
	createBranch(
		location: Location,
		abortController?: AbortController,
	): WorkflowContextImpl {
		return new WorkflowContextImpl(
			this.workflowId,
			this.storage,
			this.driver,
			this.messageDriver,
			location,
			abortController ?? this.abortController,
			this.mode,
			this.rollbackActions,
			this.rollbackCheckpointSet,
			this.historyNotifier,
			this.logger,
		);
	}

	/**
	 * Log a debug message using the configured logger.
	 */
	private log(level: "debug" | "info" | "warn" | "error", data: Record<string, unknown>): void {
		if (!this.logger) return;
		this.logger[level](data);
	}

	/**
	 * Mark a key as visited.
	 */
	private markVisited(key: string): void {
		this.visitedKeys.add(key);
	}

	/**
	 * Check if a name has already been used at the current location in this execution.
	 * Throws HistoryDivergedError if duplicate detected.
	 */
	private checkDuplicateName(name: string): void {
		const fullKey =
			locationToKey(this.storage, this.currentLocation) + "/" + name;
		if (this.usedNamesInExecution.has(fullKey)) {
			throw new HistoryDivergedError(
				`Duplicate entry name "${name}" at location "${locationToKey(this.storage, this.currentLocation)}". ` +
					`Each step/loop/sleep/listen/join/race must have a unique name within its scope.`,
			);
		}
		this.usedNamesInExecution.add(fullKey);
	}

	private stopRollback(): never {
		throw new RollbackStopError();
	}

	private stopRollbackIfMissing(entry: Entry | undefined): void {
		if (this.mode === "rollback" && !entry) {
			this.stopRollback();
		}
	}

	private stopRollbackIfIncomplete(condition: boolean): void {
		if (this.mode === "rollback" && condition) {
			this.stopRollback();
		}
	}

	private registerRollbackAction<T>(
		config: StepConfig<T>,
		entryId: string,
		output: T,
		metadata: EntryMetadata,
	): void {
		if (!config.rollback) {
			return;
		}
		if (metadata.rollbackCompletedAt !== undefined) {
			return;
		}
		this.rollbackActions?.push({
			entryId,
			name: config.name,
			output: output as unknown,
			rollback: config.rollback as (
				ctx: RollbackContextInterface,
				output: unknown,
			) => Promise<void>,
		});
	}

	/**
	 * Ensure a rollback checkpoint exists before registering rollback handlers.
	 */
	private ensureRollbackCheckpoint<T>(config: StepConfig<T>): void {
		if (!config.rollback) {
			return;
		}
		if (!this.rollbackCheckpointSet) {
			throw new RollbackCheckpointError();
		}
	}

	/**
	 * Validate that all expected entries in the branch were visited.
	 * Throws HistoryDivergedError if there are unvisited entries.
	 */
	validateComplete(): void {
		const prefix = locationToKey(this.storage, this.currentLocation);

		for (const key of this.storage.history.entries.keys()) {
			// Check if this key is under our current location prefix
			// Handle root prefix (empty string) specially - all keys are under root
			const isUnderPrefix =
				prefix === ""
					? true // Root: all keys are children
					: key.startsWith(prefix + "/") || key === prefix;

			if (isUnderPrefix) {
				if (!this.visitedKeys.has(key)) {
					// Entry exists in history but wasn't visited
					// This means workflow code may have changed
					throw new HistoryDivergedError(
						`Entry "${key}" exists in history but was not visited. ` +
							`Workflow code may have changed. Use ctx.removed() to handle migrations.`,
					);
				}
			}
		}
	}

	/**
	 * Evict the workflow.
	 */
	evict(): void {
		this.abortController.abort(new EvictedError());
	}

	/**
	 * Wait for eviction message.
	 *
	 * The event listener uses { once: true } to auto-remove after firing,
	 * preventing memory leaks if this method is called multiple times.
	 */
	waitForEviction(): Promise<never> {
		return new Promise((_, reject) => {
			if (this.abortSignal.aborted) {
				reject(new EvictedError());
				return;
			}
			this.abortSignal.addEventListener(
				"abort",
				() => {
					reject(new EvictedError());
				},
				{ once: true },
			);
		});
	}

	// === Step ===

	async step<T>(
		nameOrConfig: string | StepConfig<T>,
		run?: () => Promise<T>,
	): Promise<T> {
		this.assertNotInProgress();
		this.checkEvicted();

		const config: StepConfig<T> =
			typeof nameOrConfig === "string"
				? { name: nameOrConfig, run: run! }
				: nameOrConfig;

		this.entryInProgress = true;
		try {
			return await this.executeStep(config);
		} finally {
			this.entryInProgress = false;
		}
	}

	private async executeStep<T>(config: StepConfig<T>): Promise<T> {
		this.ensureRollbackCheckpoint(config);
		if (this.mode === "rollback") {
			return await this.executeStepRollback(config);
		}

		// Check for duplicate name in current execution
		this.checkDuplicateName(config.name);

		const location = appendName(
			this.storage,
			this.currentLocation,
			config.name,
		);
		const key = locationToKey(this.storage, location);
		const existing = this.storage.history.entries.get(key);

		// Mark this entry as visited for validateComplete
		this.markVisited(key);

		if (existing) {
			if (existing.kind.type !== "step") {
				throw new HistoryDivergedError(
					`Expected step "${config.name}" at ${key}, found ${existing.kind.type}`,
				);
			}

			const stepData = existing.kind.data;

			// Replay successful result
			if (stepData.output !== undefined) {
				this.log("debug", { msg: "replaying step from history", step: config.name, key });
				return stepData.output as T;
			}

			// Check if we should retry
			const metadata = await loadMetadata(
				this.storage,
				this.driver,
				existing.id,
			);
			const maxRetries = config.maxRetries ?? DEFAULT_MAX_RETRIES;

			if (metadata.attempts >= maxRetries) {
				// Prefer step history error, but fall back to metadata since
				// driver implementations may persist metadata without the history
				// entry error (e.g. partial writes/crashes between attempts).
				const lastError = stepData.error ?? metadata.error;
				throw new StepExhaustedError(config.name, lastError);
			}

			// Calculate backoff and yield to scheduler
			// This allows the workflow to be evicted during backoff
			const backoffDelay = calculateBackoff(
				metadata.attempts,
				config.retryBackoffBase ?? DEFAULT_RETRY_BACKOFF_BASE,
				config.retryBackoffMax ?? DEFAULT_RETRY_BACKOFF_MAX,
			);
			const retryAt = metadata.lastAttemptAt + backoffDelay;
			const now = Date.now();

			if (now < retryAt) {
				// Yield to scheduler - will be woken up at retryAt
				throw new SleepError(retryAt);
			}
		}

		// Execute the step
		const entry =
			existing ?? createEntry(location, { type: "step", data: {} });
		if (!existing) {
			// New entry - register name
			this.log("debug", { msg: "executing new step", step: config.name, key });
			const nameIndex = registerName(this.storage, config.name);
			entry.location = [...location];
			entry.location[entry.location.length - 1] = nameIndex;
			setEntry(this.storage, location, entry);
		} else {
			this.log("debug", { msg: "retrying step", step: config.name, key });
		}

		const metadata = getOrCreateMetadata(this.storage, entry.id);
		metadata.status = "running";
		metadata.attempts++;
		metadata.lastAttemptAt = Date.now();
		metadata.dirty = true;

		// Get timeout configuration
		const timeout = config.timeout ?? DEFAULT_STEP_TIMEOUT;

		try {
			// Execute with timeout
			const output = await this.executeWithTimeout(
				config.run(),
				timeout,
				config.name,
			);

				if (entry.kind.type === "step") {
					entry.kind.data.output = output;
				}
				entry.dirty = true;
				metadata.status = "completed";
				metadata.error = undefined;
				metadata.completedAt = Date.now();

				// Ephemeral steps don't trigger an immediate flush. This avoids the
			// synchronous write overhead for transient operations. Note that the
			// step's entry is still marked dirty and WILL be persisted on the
			// next flush from a non-ephemeral operation. The purpose of ephemeral
			// is to batch writes, not to avoid persistence entirely.
			if (!config.ephemeral) {
				this.log("debug", { msg: "flushing step", step: config.name, key });
				await this.flushStorage();
			}

			this.log("debug", { msg: "step completed", step: config.name, key });
			return output;
		} catch (error) {
				// Timeout errors are treated as critical (no retry)
				if (error instanceof StepTimeoutError) {
					if (entry.kind.type === "step") {
						entry.kind.data.error = String(error);
					}
					entry.dirty = true;
					metadata.status = "exhausted";
					metadata.error = String(error);
					await this.flushStorage();
					throw new CriticalError(error.message);
				}

			if (
				error instanceof CriticalError ||
				error instanceof RollbackError
				) {
					if (entry.kind.type === "step") {
						entry.kind.data.error = String(error);
					}
					entry.dirty = true;
					metadata.status = "exhausted";
					metadata.error = String(error);
					await this.flushStorage();
					throw error;
				}

				if (entry.kind.type === "step") {
					entry.kind.data.error = String(error);
				}
				entry.dirty = true;
				metadata.status = "failed";
				metadata.error = String(error);

				await this.flushStorage();

				throw new StepFailedError(config.name, error, metadata.attempts);
		}
	}

	/**
	 * Execute a promise with timeout.
	 *
	 * Note: This does NOT cancel the underlying operation. JavaScript Promises
	 * cannot be cancelled once started. When a timeout occurs:
	 * - The step is marked as failed with StepTimeoutError
	 * - The underlying async operation continues running in the background
	 * - Any side effects from the operation may still occur
	 *
	 * For cancellable operations, pass ctx.abortSignal to APIs that support AbortSignal:
	 *
	 *     return fetch(url, { signal: ctx.abortSignal });

	 *   });
	 *
	 * Or check ctx.isEvicted() periodically in long-running loops.
	 */
	private async executeStepRollback<T>(config: StepConfig<T>): Promise<T> {
		this.checkDuplicateName(config.name);
		this.ensureRollbackCheckpoint(config);

		const location = appendName(
			this.storage,
			this.currentLocation,
			config.name,
		);
		const key = locationToKey(this.storage, location);
		const existing = this.storage.history.entries.get(key);

		this.markVisited(key);

		if (!existing || existing.kind.type !== "step") {
			this.stopRollback();
		}

		const metadata = await loadMetadata(
			this.storage,
			this.driver,
			existing.id,
		);
		if (metadata.status !== "completed") {
			this.stopRollback();
		}

		const output = existing.kind.data.output as T;
		this.registerRollbackAction(config, existing.id, output, metadata);

		return output;
	}

	private async executeWithTimeout<T>(
		promise: Promise<T>,
		timeoutMs: number,
		stepName: string,
	): Promise<T> {
		if (timeoutMs <= 0) {
			return promise;
		}

		let timeoutId: ReturnType<typeof setTimeout> | undefined;
		const timeoutPromise = new Promise<never>((_, reject) => {
			timeoutId = setTimeout(() => {
				reject(new StepTimeoutError(stepName, timeoutMs));
			}, timeoutMs);
		});

		try {
			return await Promise.race([promise, timeoutPromise]);
		} finally {
			if (timeoutId !== undefined) {
				clearTimeout(timeoutId);
			}
		}
	}

	// === Loop ===

	async loop<S, T>(
		nameOrConfig: string | LoopConfig<S, T>,
		run?: (
			ctx: WorkflowContextInterface,
		) => Promise<LoopResult<undefined, T>>,
	): Promise<T> {
		this.assertNotInProgress();
		this.checkEvicted();

		const config: LoopConfig<S, T> =
			typeof nameOrConfig === "string"
				? { name: nameOrConfig, run: run as LoopConfig<S, T>["run"] }
				: nameOrConfig;

		this.entryInProgress = true;
		try {
			return await this.executeLoop(config);
		} finally {
			this.entryInProgress = false;
		}
	}

	private async executeLoop<S, T>(config: LoopConfig<S, T>): Promise<T> {
		// Check for duplicate name in current execution
		this.checkDuplicateName(config.name);

		const location = appendName(
			this.storage,
			this.currentLocation,
			config.name,
		);
		const key = locationToKey(this.storage, location);
		const existing = this.storage.history.entries.get(key);

		// Mark this entry as visited for validateComplete
		this.markVisited(key);

		let entry: Entry;
		let state: S;
		let iteration: number;
		let rollbackSingleIteration = false;
		let rollbackIterationRan = false;
		let rollbackOutput: T | undefined;
		const rollbackMode = this.mode === "rollback";

		if (existing) {
			if (existing.kind.type !== "loop") {
				throw new HistoryDivergedError(
					`Expected loop "${config.name}" at ${key}, found ${existing.kind.type}`,
				);
			}

			const loopData = existing.kind.data;

			if (rollbackMode) {
				if (loopData.output !== undefined) {
					return loopData.output as T;
				}
				rollbackSingleIteration = true;
				rollbackIterationRan = false;
				rollbackOutput = undefined;
			}

			// Loop already completed
			if (loopData.output !== undefined) {
				return loopData.output as T;
			}

			// Resume from saved state
			entry = existing;
			state = loopData.state as S;
			iteration = loopData.iteration;
			if (rollbackMode) {
				rollbackOutput = loopData.output as T | undefined;
				rollbackIterationRan = rollbackOutput !== undefined;
			}
		} else {
			this.stopRollbackIfIncomplete(true);

			// New loop
			state = config.state as S;
			iteration = 0;
			entry = createEntry(location, {
				type: "loop",
				data: { state, iteration },
			});
			setEntry(this.storage, location, entry);
		}

		// TODO: Add validation for commitInterval (must be > 0)
		const commitInterval =
			config.commitInterval ?? DEFAULT_LOOP_COMMIT_INTERVAL;
		const historyEvery =
			config.historyEvery ??
			config.commitInterval ??
			DEFAULT_LOOP_HISTORY_EVERY;
		const historyKeep =
			config.historyKeep ??
			config.commitInterval ??
			DEFAULT_LOOP_HISTORY_KEEP;

		// Execute loop iterations
		while (true) {
			if (rollbackMode && rollbackSingleIteration) {
				if (rollbackIterationRan) {
					return rollbackOutput as T;
				}
				this.stopRollbackIfIncomplete(true);
			}
			this.checkEvicted();

			// Create branch for this iteration
			const iterationLocation = appendLoopIteration(
				this.storage,
				location,
				config.name,
				iteration,
			);
			const branchCtx = this.createBranch(iterationLocation);

			// Execute iteration
			const result = await config.run(branchCtx, state);

			// Validate branch completed cleanly
			branchCtx.validateComplete();

			if ("break" in result && result.break) {
				// Loop complete
				if (entry.kind.type === "loop") {
					entry.kind.data.output = result.value;
					entry.kind.data.state = state;
					entry.kind.data.iteration = iteration;
				}
				entry.dirty = true;

				await this.flushStorage();
				await this.forgetOldIterations(
					location,
					iteration + 1,
					historyEvery,
					historyKeep,
				);

				if (rollbackMode && rollbackSingleIteration) {
					rollbackOutput = result.value;
					rollbackIterationRan = true;
					continue;
				}

				return result.value;
			}

			// Continue with new state
			if ("continue" in result && result.continue) {
				state = result.state;
			}
			iteration++;

			// Periodic commit
			if (iteration % commitInterval === 0) {
				if (entry.kind.type === "loop") {
					entry.kind.data.state = state;
					entry.kind.data.iteration = iteration;
				}
				entry.dirty = true;

				await this.flushStorage();
				await this.forgetOldIterations(
					location,
					iteration,
					historyEvery,
					historyKeep,
				);
			}
		}
	}

	/**
	 * Delete old loop iteration entries to save storage space.
	 *
	 * Loop locations always end with a NameIndex (number) because loops are
	 * created via appendName(). Even for nested loops, the innermost loop's
	 * location ends with its name index:
	 *
	 *   ctx.loop("outer") → location: [outerIndex]
	 *     iteration 0    → location: [{ loop: outerIndex, iteration: 0 }]
	 *       ctx.loop("inner") → location: [{ loop: outerIndex, iteration: 0 }, innerIndex]
	 *
	 * This function removes iterations older than (currentIteration - historyKeep)
	 * every historyEvery iterations.
	 */
	private async forgetOldIterations(
		loopLocation: Location,
		currentIteration: number,
		historyEvery: number,
		historyKeep: number,
	): Promise<void> {
		if (historyEvery <= 0 || historyKeep <= 0) {
			return;
		}
		if (currentIteration === 0 || currentIteration % historyEvery !== 0) {
			return;
		}
		const keepFrom = Math.max(0, currentIteration - historyKeep);
		// Get the loop name index from the last segment of loopLocation.
		// This is always a NameIndex (number) because loop entries are created
		// via appendName(), not appendLoopIteration().
		const loopSegment = loopLocation[loopLocation.length - 1];
		if (typeof loopSegment !== "number") {
			throw new Error("Expected loop location to end with a name index");
		}

		for (let i = 0; i < keepFrom; i++) {
			const iterationLocation: Location = [
				...loopLocation,
				{ loop: loopSegment, iteration: i },
			];
			await deleteEntriesWithPrefix(
				this.storage,
				this.driver,
				iterationLocation,
				this.historyNotifier,
			);
		}
	}

	// === Sleep ===

	async sleep(name: string, durationMs: number): Promise<void> {
		const deadline = Date.now() + durationMs;
		return this.sleepUntil(name, deadline);
	}

	async sleepUntil(name: string, timestampMs: number): Promise<void> {
		this.assertNotInProgress();
		this.checkEvicted();

		this.entryInProgress = true;
		try {
			await this.executeSleep(name, timestampMs);
		} finally {
			this.entryInProgress = false;
		}
	}

	private async executeSleep(name: string, deadline: number): Promise<void> {
		// Check for duplicate name in current execution
		this.checkDuplicateName(name);

		const location = appendName(this.storage, this.currentLocation, name);
		const key = locationToKey(this.storage, location);
		const existing = this.storage.history.entries.get(key);

		// Mark this entry as visited for validateComplete
		this.markVisited(key);

		let entry: Entry;

		if (existing) {
			if (existing.kind.type !== "sleep") {
				throw new HistoryDivergedError(
					`Expected sleep "${name}" at ${key}, found ${existing.kind.type}`,
				);
			}

			const sleepData = existing.kind.data;

			if (this.mode === "rollback") {
				this.stopRollbackIfIncomplete(sleepData.state === "pending");
				return;
			}

			// Already completed or interrupted
			if (sleepData.state !== "pending") {
				return;
			}

			// Use stored deadline
			deadline = sleepData.deadline;
			entry = existing;
		} else {
			this.stopRollbackIfIncomplete(true);

			entry = createEntry(location, {
				type: "sleep",
				data: { deadline, state: "pending" },
			});
			setEntry(this.storage, location, entry);
			entry.dirty = true;
			await this.flushStorage();
		}

		const now = Date.now();
		const remaining = deadline - now;

		if (remaining <= 0) {
			// Deadline passed
			if (entry.kind.type === "sleep") {
				entry.kind.data.state = "completed";
			}
			entry.dirty = true;
			await this.flushStorage();
			return;
		}

		// Short sleep: wait in memory
		if (remaining < this.driver.workerPollInterval) {
			await Promise.race([sleep(remaining), this.waitForEviction()]);

			this.checkEvicted();

			if (entry.kind.type === "sleep") {
				entry.kind.data.state = "completed";
			}
			entry.dirty = true;
			await this.flushStorage();
			return;
		}

		// Long sleep: yield to scheduler
		throw new SleepError(deadline);
	}

	// === Rollback Checkpoint ===

	async rollbackCheckpoint(name: string): Promise<void> {
		this.assertNotInProgress();
		this.checkEvicted();

		this.entryInProgress = true;
		try {
			await this.executeRollbackCheckpoint(name);
		} finally {
			this.entryInProgress = false;
		}
	}

	private async executeRollbackCheckpoint(name: string): Promise<void> {
		this.checkDuplicateName(name);

		const location = appendName(this.storage, this.currentLocation, name);
		const key = locationToKey(this.storage, location);
		const existing = this.storage.history.entries.get(key);

		this.markVisited(key);

		if (existing) {
			if (existing.kind.type !== "rollback_checkpoint") {
				throw new HistoryDivergedError(
					`Expected rollback checkpoint "${name}" at ${key}, found ${existing.kind.type}`,
				);
			}
			this.rollbackCheckpointSet = true;
			return;
		}

		if (this.mode === "rollback") {
			throw new HistoryDivergedError(
				`Missing rollback checkpoint "${name}" at ${key}`,
			);
		}

		const entry = createEntry(location, {
			type: "rollback_checkpoint",
			data: { name },
		});
		setEntry(this.storage, location, entry);
		entry.dirty = true;
		await this.flushStorage();

		this.rollbackCheckpointSet = true;
	}

	// === Listen ===
	//
	// IMPORTANT: Messages are loaded once at workflow start (in loadStorage).
	// If a message is sent via handle.message() DURING workflow execution,
	// it won't be visible until the next execution. The workflow will yield
	// (SleepError/MessageWaitError), then on the next run, loadStorage() will
	// pick up the new message. This is intentional - no polling during execution.

	async listen<T>(
		name: string,
		messageName: string | string[],
	): Promise<WorkflowListenMessage<T>> {
		this.assertNotInProgress();
		this.checkEvicted();

		this.entryInProgress = true;
		try {
			const messages = await this.executeListenN(name, messageName, 1);
			const message = messages[0];
			if (!message) {
				throw new HistoryDivergedError("Expected message for listen()");
			}
			return this.toListenMessage<T>(message);
		} finally {
			this.entryInProgress = false;
		}
	}

	async listenN<T>(
		name: string,
		messageName: string,
		limit: number,
	): Promise<T[]> {
		this.assertNotInProgress();
		this.checkEvicted();

		this.entryInProgress = true;
		try {
			const messages = await this.executeListenN(name, messageName, limit);
			await Promise.all(
				messages.map((message) => this.completeConsumedMessage(message)),
			);
			return messages.map((message) => message.data as T);
		} finally {
			this.entryInProgress = false;
		}
	}

	private async executeListenN(
		name: string,
		messageName: string | string[],
		limit: number,
	): Promise<Message[]> {
		const messageNames = this.normalizeMessageNames(messageName);
		const messageNameLabel = this.messageNamesLabel(messageNames);

		// Check for duplicate name in current execution
		this.checkDuplicateName(name);

		// Check for replay: first check if we have a count entry
		const countLocation = appendName(
			this.storage,
			this.currentLocation,
			`${name}:count`,
		);
		const countKey = locationToKey(this.storage, countLocation);
		const existingCount = this.storage.history.entries.get(countKey);

		// Mark count entry as visited
		this.markVisited(countKey);

		this.stopRollbackIfMissing(existingCount);

		if (existingCount && existingCount.kind.type === "message") {
			// Replay: read all recorded messages
			const count = existingCount.kind.data.data as number;
			const results: Message[] = [];

			for (let i = 0; i < count; i++) {
				const messageLocation = appendName(
					this.storage,
					this.currentLocation,
					`${name}:${i}`,
				);
				const messageKey = locationToKey(this.storage, messageLocation);

				// Mark each message entry as visited
				this.markVisited(messageKey);

				const existingMessage =
					this.storage.history.entries.get(messageKey);
				if (
					existingMessage &&
					existingMessage.kind.type === "message"
				) {
					results.push(
						this.fromHistoryListenMessage(
							existingMessage.kind.data.name,
							existingMessage.kind.data.data,
						),
					);
				}
			}

			return results;
		}

		// Try to consume messages immediately
		const messages = await consumeMessages(
			this.storage,
			this.messageDriver,
			messageNames,
			limit,
		);

		if (messages.length > 0) {
			// Record each message in history with indexed names
			for (let i = 0; i < messages.length; i++) {
				const messageLocation = appendName(
					this.storage,
					this.currentLocation,
					`${name}:${i}`,
				);
				const messageEntry = createEntry(messageLocation, {
					type: "message",
					data: {
						name: messages[i].name,
						data: this.toHistoryListenMessage(messages[i]),
					},
				});
				setEntry(this.storage, messageLocation, messageEntry);

				// Mark as visited
				this.markVisited(locationToKey(this.storage, messageLocation));
			}

			// Record the count for replay
			const countEntry = createEntry(countLocation, {
				type: "message",
				data: {
					name: `${messageNameLabel}:count`,
					data: messages.length,
				},
			});
			setEntry(this.storage, countLocation, countEntry);

			await this.flushStorage();

			return messages;
		}

		// No messages found, throw to yield to scheduler
		throw new MessageWaitError(messageNames);
	}

	private normalizeMessageNames(messageName: string | string[]): string[] {
		const names = Array.isArray(messageName) ? messageName : [messageName];
		const deduped: string[] = [];
		const seen = new Set<string>();

		for (const name of names) {
			if (seen.has(name)) {
				continue;
			}
			seen.add(name);
			deduped.push(name);
		}

		if (deduped.length === 0) {
			throw new Error("listen() requires at least one message name");
		}

		return deduped;
	}

	private messageNamesLabel(messageNames: string[]): string {
		return messageNames.length === 1
			? messageNames[0]
			: messageNames.join("|");
	}

	private toListenMessage<T>(message: Message): WorkflowListenMessage<T> {
		return {
			id: message.id,
			name: message.name,
			body: message.data as T,
			complete: async (response?: unknown) => {
				if (message.complete) {
					await message.complete(response);
					return;
				}
				if (this.messageDriver.completeMessage) {
					await this.messageDriver.completeMessage(message.id, response);
				}
			},
		};
	}

	private async completeConsumedMessage(message: Message): Promise<void> {
		if (message.complete) {
			await message.complete();
			return;
		}
		if (message.id && this.messageDriver.completeMessage) {
			await this.messageDriver.completeMessage(message.id);
		}
	}

	private toHistoryListenMessage(message: Message): unknown {
		return {
			[LISTEN_HISTORY_MESSAGE_MARKER]: 1,
			id: message.id,
			name: message.name,
			body: message.data,
		};
	}

	private fromHistoryListenMessage(name: string, value: unknown): Message {
		if (
			typeof value === "object" &&
			value !== null &&
			(value as Record<string, unknown>)[LISTEN_HISTORY_MESSAGE_MARKER] === 1
		) {
			const serialized = value as Record<string, unknown>;
			const id =
				typeof serialized.id === "string" ? serialized.id : "";
			const serializedName =
				typeof serialized.name === "string" ? serialized.name : name;
			const complete = async (response?: unknown) => {
				if (!id || !this.messageDriver.completeMessage) {
					return;
				}
				await this.messageDriver.completeMessage(id, response);
			};

			return {
				id,
				name: serializedName,
				data: serialized.body,
				sentAt: 0,
				complete,
			};
		}

		return {
			id: "",
			name,
			data: value,
			sentAt: 0,
		};
	}

	async listenWithTimeout<T>(
		name: string,
		messageName: string,
		timeoutMs: number,
	): Promise<T | null> {
		const deadline = Date.now() + timeoutMs;
		return this.listenUntil<T>(name, messageName, deadline);
	}

	async listenUntil<T>(
		name: string,
		messageName: string,
		timestampMs: number,
	): Promise<T | null> {
		this.assertNotInProgress();
		this.checkEvicted();

		this.entryInProgress = true;
		try {
			return await this.executeListenUntil<T>(
				name,
				messageName,
				timestampMs,
			);
		} finally {
			this.entryInProgress = false;
		}
	}

	private async executeListenUntil<T>(
		name: string,
		messageName: string,
		deadline: number,
	): Promise<T | null> {
		// Check for duplicate name in current execution
		this.checkDuplicateName(name);

		const sleepLocation = appendName(
			this.storage,
			this.currentLocation,
			name,
		);
		const messageLocation = appendName(
			this.storage,
			this.currentLocation,
			`${name}:message`,
		);
		const sleepKey = locationToKey(this.storage, sleepLocation);
		const messageKey = locationToKey(this.storage, messageLocation);

		// Mark entries as visited for validateComplete
		this.markVisited(sleepKey);
		this.markVisited(messageKey);

		const existingSleep = this.storage.history.entries.get(sleepKey);

		this.stopRollbackIfMissing(existingSleep);

		// Check for replay
		if (existingSleep && existingSleep.kind.type === "sleep") {
			const sleepData = existingSleep.kind.data;
			if (sleepData.state === "completed") {
				return null;
			}

			if (sleepData.state === "interrupted") {
				const existingMessage = this.storage.history.entries.get(messageKey);
				if (
					existingMessage &&
					existingMessage.kind.type === "message"
				) {
					const replayedMessage = this.fromHistoryListenMessage(
						existingMessage.kind.data.name,
						existingMessage.kind.data.data,
					);
					await this.completeConsumedMessage(replayedMessage);
					return replayedMessage.data as T;
				}
				throw new HistoryDivergedError(
					"Expected message entry after interrupted sleep",
				);
			}

			this.stopRollbackIfIncomplete(true);

			deadline = sleepData.deadline;
		} else {
			this.stopRollbackIfIncomplete(true);

			// Create sleep entry
			const sleepEntry = createEntry(sleepLocation, {
				type: "sleep",
				data: { deadline, state: "pending" },
			});
			setEntry(this.storage, sleepLocation, sleepEntry);
			sleepEntry.dirty = true;
			await this.flushStorage();
		}

		const now = Date.now();
		const remaining = deadline - now;

		// Deadline passed, check for message one more time
		if (remaining <= 0) {
			const message = await consumeMessage(
				this.storage,
				this.messageDriver,
				messageName,
			);
			const sleepEntry = getEntry(this.storage, sleepLocation)!;

			if (message) {
				if (sleepEntry.kind.type === "sleep") {
					sleepEntry.kind.data.state = "interrupted";
				}
				sleepEntry.dirty = true;

				const messageEntry = createEntry(messageLocation, {
					type: "message",
					data: {
						name: message.name,
						data: this.toHistoryListenMessage(message),
					},
				});
				setEntry(this.storage, messageLocation, messageEntry);
				await this.flushStorage();
				await this.completeConsumedMessage(message);

				return message.data as T;
			}

			if (sleepEntry.kind.type === "sleep") {
				sleepEntry.kind.data.state = "completed";
			}
			sleepEntry.dirty = true;
			await this.flushStorage();
			return null;
		}

		// Check for message (messages are loaded at workflow start, no polling needed)
		const message = await consumeMessage(
			this.storage,
			this.messageDriver,
			messageName,
		);
		if (message) {
			const sleepEntry = getEntry(this.storage, sleepLocation)!;
			if (sleepEntry.kind.type === "sleep") {
				sleepEntry.kind.data.state = "interrupted";
			}
			sleepEntry.dirty = true;

			const messageEntry = createEntry(messageLocation, {
				type: "message",
				data: {
					name: message.name,
					data: this.toHistoryListenMessage(message),
				},
			});
			setEntry(this.storage, messageLocation, messageEntry);
			await this.flushStorage();
			await this.completeConsumedMessage(message);

			return message.data as T;
		}

		// Message not available, yield to scheduler until deadline or message
		throw new SleepError(deadline, [messageName]);
	}

	async listenNWithTimeout<T>(
		name: string,
		messageName: string,
		limit: number,
		timeoutMs: number,
	): Promise<T[]> {
		this.assertNotInProgress();
		this.checkEvicted();

		this.entryInProgress = true;
		try {
			return await this.executeListenNWithTimeout<T>(
				name,
				messageName,
				limit,
				timeoutMs,
			);
		} finally {
			this.entryInProgress = false;
		}
	}

	private async executeListenNWithTimeout<T>(
		name: string,
		messageName: string,
		limit: number,
		timeoutMs: number,
	): Promise<T[]> {
		// Check for duplicate name in current execution
		this.checkDuplicateName(name);

		// Use a sleep entry to store the deadline for replay
		const sleepLocation = appendName(
			this.storage,
			this.currentLocation,
			`${name}:deadline`,
		);
		const sleepKey = locationToKey(this.storage, sleepLocation);
		const existingSleep = this.storage.history.entries.get(sleepKey);

		this.markVisited(sleepKey);

		this.stopRollbackIfMissing(existingSleep);

		let deadline: number;

		if (existingSleep && existingSleep.kind.type === "sleep") {
			// Replay: use stored deadline
			deadline = existingSleep.kind.data.deadline;
		} else {
			// New execution: calculate and store deadline
			deadline = Date.now() + timeoutMs;
			const sleepEntry = createEntry(sleepLocation, {
				type: "sleep",
				data: { deadline, state: "pending" },
			});
			setEntry(this.storage, sleepLocation, sleepEntry);
			sleepEntry.dirty = true;
			// Flush immediately to persist deadline before potential SleepError
			await this.flushStorage();
		}

		return this.executeListenNUntilImpl<T>(
			name,
			messageName,
			limit,
			deadline,
		);
	}

	async listenNUntil<T>(
		name: string,
		messageName: string,
		limit: number,
		timestampMs: number,
	): Promise<T[]> {
		this.assertNotInProgress();
		this.checkEvicted();

		// Check for duplicate name in current execution
		this.checkDuplicateName(name);

		this.entryInProgress = true;
		try {
			return await this.executeListenNUntilImpl<T>(
				name,
				messageName,
				limit,
				timestampMs,
			);
		} finally {
			this.entryInProgress = false;
		}
	}

	/**
	 * Internal implementation for listenNUntil with proper replay support.
	 * Stores the count and individual messages for deterministic replay.
	 */
	private async executeListenNUntilImpl<T>(
		name: string,
		messageName: string,
		limit: number,
		deadline: number,
	): Promise<T[]> {
		// Check for replay: look for count entry
		const countLocation = appendName(
			this.storage,
			this.currentLocation,
			`${name}:count`,
		);
		const countKey = locationToKey(this.storage, countLocation);
		const existingCount = this.storage.history.entries.get(countKey);

		this.markVisited(countKey);

		this.stopRollbackIfMissing(existingCount);

		if (existingCount && existingCount.kind.type === "message") {
			// Replay: read all recorded messages
			const count = existingCount.kind.data.data as number;
			const results: T[] = [];

			for (let i = 0; i < count; i++) {
				const messageLocation = appendName(
					this.storage,
					this.currentLocation,
					`${name}:${i}`,
				);
				const messageKey = locationToKey(this.storage, messageLocation);

				this.markVisited(messageKey);

				const existingMessage = this.storage.history.entries.get(messageKey);
				if (
					existingMessage &&
					existingMessage.kind.type === "message"
				) {
					const replayedMessage = this.fromHistoryListenMessage(
						existingMessage.kind.data.name,
						existingMessage.kind.data.data,
					);
					await this.completeConsumedMessage(replayedMessage);
					results.push(replayedMessage.data as T);
				}
			}

			return results;
		}

		// New execution: collect messages until timeout or limit reached
		const results: T[] = [];

		for (let i = 0; i < limit; i++) {
			const now = Date.now();
			if (now >= deadline) {
				break;
			}

			// Try to consume a message
			const message = await consumeMessage(
				this.storage,
				this.messageDriver,
				messageName,
			);
			if (!message) {
				// No message available - check if we should wait
				if (results.length === 0) {
					// No messages yet - yield to scheduler until deadline or message
					throw new SleepError(deadline, [messageName]);
				}
				// We have some messages - return what we have
				break;
			}

			// Record the message
			const messageLocation = appendName(
				this.storage,
				this.currentLocation,
				`${name}:${i}`,
			);
			const messageEntry = createEntry(messageLocation, {
				type: "message",
				data: {
					name: message.name,
					data: this.toHistoryListenMessage(message),
				},
			});
			setEntry(this.storage, messageLocation, messageEntry);
			this.markVisited(locationToKey(this.storage, messageLocation));
			await this.completeConsumedMessage(message);

			results.push(message.data as T);
		}

		// Record the count for replay
		const countEntry = createEntry(countLocation, {
			type: "message",
			data: { name: `${messageName}:count`, data: results.length },
		});
		setEntry(this.storage, countLocation, countEntry);

		await this.flushStorage();

		return results;
	}

	// === Join ===

	async join<T extends Record<string, BranchConfig<unknown>>>(
		name: string,
		branches: T,
	): Promise<{ [K in keyof T]: BranchOutput<T[K]> }> {
		this.assertNotInProgress();
		this.checkEvicted();

		this.entryInProgress = true;
		try {
			return await this.executeJoin(name, branches);
		} finally {
			this.entryInProgress = false;
		}
	}

	private async executeJoin<T extends Record<string, BranchConfig<unknown>>>(
		name: string,
		branches: T,
	): Promise<{ [K in keyof T]: BranchOutput<T[K]> }> {
		// Check for duplicate name in current execution
		this.checkDuplicateName(name);

		const location = appendName(this.storage, this.currentLocation, name);
		const key = locationToKey(this.storage, location);
		const existing = this.storage.history.entries.get(key);

		// Mark this entry as visited for validateComplete
		this.markVisited(key);

		this.stopRollbackIfMissing(existing);

		let entry: Entry;

		if (existing) {
			if (existing.kind.type !== "join") {
				throw new HistoryDivergedError(
					`Expected join "${name}" at ${key}, found ${existing.kind.type}`,
				);
			}
			entry = existing;
		} else {
			entry = createEntry(location, {
				type: "join",
				data: {
					branches: Object.fromEntries(
						Object.keys(branches).map((k) => [
							k,
							{ status: "pending" as const },
						]),
					),
				},
			});
			setEntry(this.storage, location, entry);
			entry.dirty = true;
			// Flush immediately to persist entry before branches execute
			await this.flushStorage();
		}

		if (entry.kind.type !== "join") {
			throw new HistoryDivergedError("Entry type mismatch");
		}

		this.stopRollbackIfIncomplete(
			Object.values(entry.kind.data.branches).some(
				(branch) => branch.status !== "completed",
			),
		);

		const joinData = entry.kind.data;
		const results: Record<string, unknown> = {};
		const errors: Record<string, Error> = {};

		// Execute all branches in parallel
		const branchPromises = Object.entries(branches).map(
			async ([branchName, config]) => {
				const branchStatus = joinData.branches[branchName];

				// Already completed
				if (branchStatus.status === "completed") {
					results[branchName] = branchStatus.output;
					return;
				}

				// Already failed
				if (branchStatus.status === "failed") {
					errors[branchName] = new Error(branchStatus.error);
					return;
				}

				// Execute branch
				const branchLocation = appendName(
					this.storage,
					location,
					branchName,
				);
				const branchCtx = this.createBranch(branchLocation);

				branchStatus.status = "running";
				entry.dirty = true;

				try {
					const output = await config.run(branchCtx);
					branchCtx.validateComplete();

					branchStatus.status = "completed";
					branchStatus.output = output;
					results[branchName] = output;
				} catch (error) {
					branchStatus.status = "failed";
					branchStatus.error = String(error);
					errors[branchName] = error as Error;
				}

				entry.dirty = true;
			},
		);

		// Wait for ALL branches (no short-circuit on error)
		await Promise.allSettled(branchPromises);
		await this.flushStorage();

		// Throw if any branches failed
		if (Object.keys(errors).length > 0) {
			throw new JoinError(errors);
		}

		return results as { [K in keyof T]: BranchOutput<T[K]> };
	}

	// === Race ===

	async race<T>(
		name: string,
		branches: Array<{
			name: string;
			run: (ctx: WorkflowContextInterface) => Promise<T>;
		}>,
	): Promise<{ winner: string; value: T }> {
		this.assertNotInProgress();
		this.checkEvicted();

		this.entryInProgress = true;
		try {
			return await this.executeRace(name, branches);
		} finally {
			this.entryInProgress = false;
		}
	}

	private async executeRace<T>(
		name: string,
		branches: Array<{
			name: string;
			run: (ctx: WorkflowContextInterface) => Promise<T>;
		}>,
	): Promise<{ winner: string; value: T }> {
		// Check for duplicate name in current execution
		this.checkDuplicateName(name);

		const location = appendName(this.storage, this.currentLocation, name);
		const key = locationToKey(this.storage, location);
		const existing = this.storage.history.entries.get(key);

		// Mark this entry as visited for validateComplete
		this.markVisited(key);

		this.stopRollbackIfMissing(existing);

		let entry: Entry;

		if (existing) {
			if (existing.kind.type !== "race") {
				throw new HistoryDivergedError(
					`Expected race "${name}" at ${key}, found ${existing.kind.type}`,
				);
			}
			entry = existing;

			// Check if we already have a winner
			const raceKind = existing.kind;
			if (raceKind.data.winner !== null) {
				const winnerStatus =
					raceKind.data.branches[raceKind.data.winner];
				return {
					winner: raceKind.data.winner,
					value: winnerStatus.output as T,
				};
			}

			this.stopRollbackIfIncomplete(true);
		} else {
			entry = createEntry(location, {
				type: "race",
				data: {
					winner: null,
					branches: Object.fromEntries(
						branches.map((b) => [
							b.name,
							{ status: "pending" as const },
						]),
					),
				},
			});
			setEntry(this.storage, location, entry);
			entry.dirty = true;
			// Flush immediately to persist entry before branches execute
			await this.flushStorage();
		}

		if (entry.kind.type !== "race") {
			throw new HistoryDivergedError("Entry type mismatch");
		}

		const raceData = entry.kind.data;

		// Create abort controller for cancellation
		const raceAbortController = new AbortController();

		// Track all branch promises to wait for cleanup
		const branchPromises: Promise<void>[] = [];

		// Track winner info
		let winnerName: string | null = null;
		let winnerValue: T | null = null;
		let settled = false;
		let pendingCount = branches.length;
		const errors: Record<string, Error> = {};
		const lateErrors: Array<{ name: string; error: string }> = [];
		// Track scheduler yield errors - we need to propagate these after allSettled
		let yieldError: SleepError | MessageWaitError | null = null;

		// Check for replay winners first
		for (const branch of branches) {
			const branchStatus = raceData.branches[branch.name];
			if (
				branchStatus.status !== "pending" &&
				branchStatus.status !== "running"
			) {
				pendingCount--;
				if (branchStatus.status === "completed" && !settled) {
					settled = true;
					winnerName = branch.name;
					winnerValue = branchStatus.output as T;
				}
			}
		}

		// If we found a replay winner, return immediately
		if (settled && winnerName !== null && winnerValue !== null) {
			return { winner: winnerName, value: winnerValue };
		}

		// Execute branches that need to run
		for (const branch of branches) {
			const branchStatus = raceData.branches[branch.name];

			// Skip already completed/cancelled
			if (
				branchStatus.status !== "pending" &&
				branchStatus.status !== "running"
			) {
				continue;
			}

			const branchLocation = appendName(
				this.storage,
				location,
				branch.name,
			);
			const branchCtx = this.createBranch(
				branchLocation,
				raceAbortController,
			);

			branchStatus.status = "running";
			entry.dirty = true;

			const branchPromise = branch.run(branchCtx).then(
				async (output) => {
					if (settled) {
						// This branch completed after a winner was determined
						// Still record the completion for observability
						branchStatus.status = "completed";
						branchStatus.output = output;
						entry.dirty = true;
						return;
					}
					settled = true;
					winnerName = branch.name;
					winnerValue = output;

					branchCtx.validateComplete();

					branchStatus.status = "completed";
					branchStatus.output = output;
					raceData.winner = branch.name;
					entry.dirty = true;

					// Cancel other branches
					raceAbortController.abort();
				},
				(error) => {
					pendingCount--;

					// Track sleep/message errors - they need to bubble up to the scheduler
					// We'll re-throw after allSettled to allow cleanup
					if (error instanceof SleepError) {
						// Track the earliest deadline
						if (
							!yieldError ||
							!(yieldError instanceof SleepError) ||
							error.deadline < yieldError.deadline
						) {
							yieldError = error;
						}
						branchStatus.status = "running"; // Keep as running since we'll resume
						entry.dirty = true;
						return;
					}
					if (error instanceof MessageWaitError) {
						// Track message wait errors, prefer sleep errors with deadlines
						if (!yieldError || !(yieldError instanceof SleepError)) {
							if (!yieldError) {
								yieldError = error;
							} else if (yieldError instanceof MessageWaitError) {
								// Merge message names
								yieldError = new MessageWaitError([
									...yieldError.messageNames,
									...error.messageNames,
								]);
							}
						}
						branchStatus.status = "running"; // Keep as running since we'll resume
						entry.dirty = true;
						return;
					}

					if (
						error instanceof CancelledError ||
						error instanceof EvictedError
					) {
						branchStatus.status = "cancelled";
					} else {
						branchStatus.status = "failed";
						branchStatus.error = String(error);

						if (settled) {
							// Track late errors for observability
							lateErrors.push({
								name: branch.name,
								error: String(error),
							});
						} else {
							errors[branch.name] = error;
						}
					}
					entry.dirty = true;

					// All branches failed (only if no winner yet)
					if (pendingCount === 0 && !settled) {
						settled = true;
					}
				},
			);

			branchPromises.push(branchPromise);
		}

		// Wait for all branches to complete or be cancelled
		await Promise.allSettled(branchPromises);

		// If any branch needs to yield to the scheduler (sleep/message wait),
		// save state and re-throw the error to exit the workflow execution
		if (yieldError && !settled) {
			await this.flushStorage();
			throw yieldError;
		}

		// Clean up entries from non-winning branches
		if (winnerName !== null) {
			for (const branch of branches) {
				if (branch.name !== winnerName) {
					const branchLocation = appendName(
						this.storage,
						location,
						branch.name,
					);
					await deleteEntriesWithPrefix(
						this.storage,
						this.driver,
						branchLocation,
						this.historyNotifier,
					);
				}
			}
		}

		// Flush final state
		await this.flushStorage();

		// Log late errors if any (these occurred after a winner was determined)
		if (lateErrors.length > 0) {
			console.warn(
				`Race "${name}" had ${lateErrors.length} branch(es) fail after winner was determined:`,
				lateErrors,
			);
		}

		// Return result or throw error
		if (winnerName !== null && winnerValue !== null) {
			return { winner: winnerName, value: winnerValue };
		}

		// All branches failed
		throw new RaceError(
			"All branches failed",
			Object.entries(errors).map(([name, error]) => ({
				name,
				error: String(error),
			})),
		);
	}

	// === Removed ===

	async removed(name: string, originalType: EntryKindType): Promise<void> {
		this.assertNotInProgress();
		this.checkEvicted();

		this.entryInProgress = true;
		try {
			await this.executeRemoved(name, originalType);
		} finally {
			this.entryInProgress = false;
		}
	}

	private async executeRemoved(
		name: string,
		originalType: EntryKindType,
	): Promise<void> {
		// Check for duplicate name in current execution
		this.checkDuplicateName(name);

		const location = appendName(this.storage, this.currentLocation, name);
		const key = locationToKey(this.storage, location);
		const existing = this.storage.history.entries.get(key);

		// Mark this entry as visited for validateComplete
		this.markVisited(key);

		this.stopRollbackIfMissing(existing);

		if (existing) {
			// Validate the existing entry matches what we expect
			if (
				existing.kind.type !== "removed" &&
				existing.kind.type !== originalType
			) {
				throw new HistoryDivergedError(
					`Expected ${originalType} or removed at ${key}, found ${existing.kind.type}`,
				);
			}

			// If it's not already marked as removed, we just skip it
			return;
		}

		// Create a removed entry placeholder
		const entry = createEntry(location, {
			type: "removed",
			data: { originalType, originalName: name },
		});
		setEntry(this.storage, location, entry);
		await this.flushStorage();
	}
}

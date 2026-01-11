import type { EngineDriver } from "./driver.js";
import {
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
import {
	appendLoopIteration,
	appendName,
	emptyLocation,
	locationToKey,
	registerName,
} from "./location.js";
import {
	consumeSignal,
	consumeSignals,
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
	Location,
	LoopConfig,
	LoopResult,
	Signal,
	StepConfig,
	Storage,
	WorkflowContextInterface,
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
export const DEFAULT_STEP_TIMEOUT = 30000; // 30 seconds

/**
 * Calculate backoff delay with exponential backoff.
 * Uses deterministic calculation (no jitter) for replay consistency.
 */
function calculateBackoff(
	attempts: number,
	base: number,
	max: number,
): number {
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
 * Internal implementation of WorkflowContext.
 */
export class WorkflowContextImpl implements WorkflowContextInterface {
	private entryInProgress = false;
	private abortController: AbortController;
	private currentLocation: Location;
	private visitedKeys = new Set<string>();
	/** Track names used in current execution to detect duplicates */
	private usedNamesInExecution = new Set<string>();

	constructor(
		public readonly workflowId: string,
		private storage: Storage,
		private driver: EngineDriver,
		location: Location = emptyLocation(),
		abortController?: AbortController,
	) {
		this.currentLocation = location;
		this.abortController = abortController ?? new AbortController();
	}

	get signal(): AbortSignal {
		return this.abortController.signal;
	}

	isEvicted(): boolean {
		return this.signal.aborted;
	}

	private assertNotInProgress(): void {
		if (this.entryInProgress) {
			throw new EntryInProgressError();
		}
	}

	private checkEvicted(): void {
		if (this.signal.aborted) {
			throw new EvictedError();
		}
	}

	/**
	 * Create a new branch context for parallel/nested execution.
	 */
	createBranch(location: Location, abortController?: AbortController): WorkflowContextImpl {
		return new WorkflowContextImpl(
			this.workflowId,
			this.storage,
			this.driver,
			location,
			abortController ?? this.abortController,
		);
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
		const fullKey = locationToKey(this.storage, this.currentLocation) + "/" + name;
		if (this.usedNamesInExecution.has(fullKey)) {
			throw new HistoryDivergedError(
				`Duplicate entry name "${name}" at location "${locationToKey(this.storage, this.currentLocation)}". ` +
					`Each step/loop/sleep/listen/join/race must have a unique name within its scope.`,
			);
		}
		this.usedNamesInExecution.add(fullKey);
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
	 * Wait for eviction signal.
	 *
	 * The event listener uses { once: true } to auto-remove after firing,
	 * preventing memory leaks if this method is called multiple times.
	 */
	waitForEviction(): Promise<never> {
		return new Promise((_, reject) => {
			if (this.signal.aborted) {
				reject(new EvictedError());
				return;
			}
			this.signal.addEventListener(
				"abort",
				() => {
					reject(new EvictedError());
				},
				{ once: true },
			);
		});
	}

	// === Step ===

	async step<T>(nameOrConfig: string | StepConfig<T>, run?: () => Promise<T>): Promise<T> {
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
		// Check for duplicate name in current execution
		this.checkDuplicateName(config.name);

		const location = appendName(this.storage, this.currentLocation, config.name);
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
				return stepData.output as T;
			}

			// Check if we should retry
			const metadata = await loadMetadata(this.storage, this.driver, existing.id);
			const maxRetries = config.maxRetries ?? DEFAULT_MAX_RETRIES;

			if (metadata.attempts >= maxRetries) {
				throw new StepExhaustedError(config.name, stepData.error);
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
			existing ??
			createEntry(location, { type: "step", data: {} });
		if (!existing) {
			setEntry(this.storage, location, entry);
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
			metadata.completedAt = Date.now();

			// Ephemeral steps don't trigger an immediate flush. This avoids the
			// synchronous write overhead for transient operations. Note that the
			// step's entry is still marked dirty and WILL be persisted on the
			// next flush from a non-ephemeral operation. The purpose of ephemeral
			// is to batch writes, not to avoid persistence entirely.
			if (!config.ephemeral) {
				await flush(this.storage, this.driver);
			}

			return output;
		} catch (error) {
			// Timeout errors are treated as critical (no retry)
			if (error instanceof StepTimeoutError) {
				if (entry.kind.type === "step") {
					entry.kind.data.error = String(error);
				}
				entry.dirty = true;
				metadata.status = "exhausted";
				await flush(this.storage, this.driver);
				throw new CriticalError(error.message);
			}

			if (error instanceof CriticalError) {
				if (entry.kind.type === "step") {
					entry.kind.data.error = String(error);
				}
				entry.dirty = true;
				metadata.status = "exhausted";
				await flush(this.storage, this.driver);
				throw error;
			}

			if (entry.kind.type === "step") {
				entry.kind.data.error = String(error);
			}
			entry.dirty = true;
			metadata.status = "failed";

			await flush(this.storage, this.driver);

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
	 * For cancellable operations, pass ctx.signal to APIs that support AbortSignal:
	 *
	 *   await ctx.step("fetch", async () => {
	 *     return fetch(url, { signal: ctx.signal });
	 *   });
	 *
	 * Or check ctx.isEvicted() periodically in long-running loops.
	 */
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
		run?: (ctx: WorkflowContextInterface) => Promise<LoopResult<undefined, T>>,
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

		const location = appendName(this.storage, this.currentLocation, config.name);
		const key = locationToKey(this.storage, location);
		const existing = this.storage.history.entries.get(key);

		// Mark this entry as visited for validateComplete
		this.markVisited(key);

		let entry: Entry;
		let state: S;
		let iteration: number;

		if (existing) {
			if (existing.kind.type !== "loop") {
				throw new HistoryDivergedError(
					`Expected loop "${config.name}" at ${key}, found ${existing.kind.type}`,
				);
			}

			const loopData = existing.kind.data;

			// Loop already completed
			if (loopData.output !== undefined) {
				return loopData.output as T;
			}

			// Resume from saved state
			entry = existing;
			state = loopData.state as S;
			iteration = loopData.iteration;
		} else {
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
		const commitInterval = config.commitInterval ?? DEFAULT_LOOP_COMMIT_INTERVAL;

		// Execute loop iterations
		while (true) {
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

				await flush(this.storage, this.driver);
				await this.forgetOldIterations(location, iteration, commitInterval);

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

				await flush(this.storage, this.driver);
				await this.forgetOldIterations(location, iteration, commitInterval);
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
	 * This function removes iterations older than (currentIteration - commitInterval).
	 */
	private async forgetOldIterations(
		loopLocation: Location,
		currentIteration: number,
		commitInterval: number,
	): Promise<void> {
		const keepFrom = Math.max(0, currentIteration - commitInterval);
		// Get the loop name index from the last segment of loopLocation.
		// This is always a NameIndex (number) because loop entries are created
		// via appendName(), not appendLoopIteration().
		const loopSegment = loopLocation[loopLocation.length - 1];
		if (typeof loopSegment !== "number") {
			throw new Error("Expected loop location to end with a name index");
		}

		for (let i = 0; i < keepFrom; i++) {
			// Build location prefix for this iteration.
			// We replace the last segment (the loop's name index) with an
			// iteration marker to target all entries under that iteration.
			const iterationLocation: Location = [
				...loopLocation.slice(0, -1),
				{ loop: loopSegment, iteration: i },
			];
			await deleteEntriesWithPrefix(this.storage, this.driver, iterationLocation);
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

			// Already completed or interrupted
			if (sleepData.state !== "pending") {
				return;
			}

			// Use stored deadline
			deadline = sleepData.deadline;
			entry = existing;
		} else {
			entry = createEntry(location, {
				type: "sleep",
				data: { deadline, state: "pending" },
			});
			setEntry(this.storage, location, entry);
			entry.dirty = true;
			await flush(this.storage, this.driver);
		}

		const now = Date.now();
		const remaining = deadline - now;

		if (remaining <= 0) {
			// Deadline passed
			if (entry.kind.type === "sleep") {
				entry.kind.data.state = "completed";
			}
			entry.dirty = true;
			await flush(this.storage, this.driver);
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
			return;
		}

		// Long sleep: yield to scheduler
		throw new SleepError(deadline);
	}

	// === Listen ===
	//
	// IMPORTANT: Signals are loaded once at workflow start (in loadStorage).
	// If a signal is sent via handle.signal() DURING workflow execution,
	// it won't be visible until the next execution. The workflow will yield
	// (SleepError/SignalWaitError), then on the next run, loadStorage() will
	// pick up the new signal. This is intentional - no polling during execution.

	async listen<T>(name: string, signalName: string): Promise<T> {
		const signals = await this.listenN<T>(name, signalName, 1);
		return signals[0];
	}

	async listenN<T>(name: string, signalName: string, limit: number): Promise<T[]> {
		this.assertNotInProgress();
		this.checkEvicted();

		this.entryInProgress = true;
		try {
			return await this.executeListenN<T>(name, signalName, limit);
		} finally {
			this.entryInProgress = false;
		}
	}

	private async executeListenN<T>(
		name: string,
		signalName: string,
		limit: number,
	): Promise<T[]> {
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

		if (existingCount && existingCount.kind.type === "signal") {
			// Replay: read all recorded signals
			const count = existingCount.kind.data.data as number;
			const results: T[] = [];

			for (let i = 0; i < count; i++) {
				const signalLocation = appendName(
					this.storage,
					this.currentLocation,
					`${name}:${i}`,
				);
				const signalKey = locationToKey(this.storage, signalLocation);

				// Mark each signal entry as visited
				this.markVisited(signalKey);

				const existingSignal = this.storage.history.entries.get(signalKey);
				if (existingSignal && existingSignal.kind.type === "signal") {
					results.push(existingSignal.kind.data.data as T);
				}
			}

			return results;
		}

		// Try to consume signals immediately
		const signals = await consumeSignals(
			this.storage,
			this.driver,
			signalName,
			limit,
		);

		if (signals.length > 0) {
			// Record each signal in history with indexed names
			for (let i = 0; i < signals.length; i++) {
				const signalLocation = appendName(
					this.storage,
					this.currentLocation,
					`${name}:${i}`,
				);
				const signalEntry = createEntry(signalLocation, {
					type: "signal",
					data: { name: signalName, data: signals[i].data },
				});
				setEntry(this.storage, signalLocation, signalEntry);

				// Mark as visited
				this.markVisited(locationToKey(this.storage, signalLocation));
			}

			// Record the count for replay
			const countEntry = createEntry(countLocation, {
				type: "signal",
				data: { name: `${signalName}:count`, data: signals.length },
			});
			setEntry(this.storage, countLocation, countEntry);

			await flush(this.storage, this.driver);

			return signals.map((s) => s.data as T);
		}

		// No signals found, throw to yield to scheduler
		throw new SignalWaitError([signalName]);
	}

	async listenWithTimeout<T>(
		name: string,
		signalName: string,
		timeoutMs: number,
	): Promise<T | null> {
		const deadline = Date.now() + timeoutMs;
		return this.listenUntil<T>(name, signalName, deadline);
	}

	async listenUntil<T>(
		name: string,
		signalName: string,
		timestampMs: number,
	): Promise<T | null> {
		this.assertNotInProgress();
		this.checkEvicted();

		this.entryInProgress = true;
		try {
			return await this.executeListenUntil<T>(name, signalName, timestampMs);
		} finally {
			this.entryInProgress = false;
		}
	}

	private async executeListenUntil<T>(
		name: string,
		signalName: string,
		deadline: number,
	): Promise<T | null> {
		// Check for duplicate name in current execution
		this.checkDuplicateName(name);

		const sleepLocation = appendName(this.storage, this.currentLocation, name);
		const signalLocation = appendName(
			this.storage,
			this.currentLocation,
			`${name}:signal`,
		);
		const sleepKey = locationToKey(this.storage, sleepLocation);
		const signalKey = locationToKey(this.storage, signalLocation);

		// Mark entries as visited for validateComplete
		this.markVisited(sleepKey);
		this.markVisited(signalKey);

		const existingSleep = this.storage.history.entries.get(sleepKey);

		// Check for replay
		if (existingSleep && existingSleep.kind.type === "sleep") {
			const sleepData = existingSleep.kind.data;

			if (sleepData.state === "completed") {
				return null;
			}

			if (sleepData.state === "interrupted") {
				const existingSignal = this.storage.history.entries.get(signalKey);
				if (existingSignal && existingSignal.kind.type === "signal") {
					return existingSignal.kind.data.data as T;
				}
				throw new HistoryDivergedError(
					"Expected signal entry after interrupted sleep",
				);
			}

			deadline = sleepData.deadline;
		} else {
			// Create sleep entry
			const sleepEntry = createEntry(sleepLocation, {
				type: "sleep",
				data: { deadline, state: "pending" },
			});
			setEntry(this.storage, sleepLocation, sleepEntry);
			sleepEntry.dirty = true;
			await flush(this.storage, this.driver);
		}

		const now = Date.now();
		const remaining = deadline - now;

		// Deadline passed, check for signal one more time
		if (remaining <= 0) {
			const signal = await consumeSignal(this.storage, this.driver, signalName);
			const sleepEntry = getEntry(this.storage, sleepLocation)!;

			if (signal) {
				if (sleepEntry.kind.type === "sleep") {
					sleepEntry.kind.data.state = "interrupted";
				}
				sleepEntry.dirty = true;

				const signalEntry = createEntry(signalLocation, {
					type: "signal",
					data: { name: signalName, data: signal.data },
				});
				setEntry(this.storage, signalLocation, signalEntry);
				await flush(this.storage, this.driver);

				return signal.data as T;
			}

			if (sleepEntry.kind.type === "sleep") {
				sleepEntry.kind.data.state = "completed";
			}
			sleepEntry.dirty = true;
			await flush(this.storage, this.driver);
			return null;
		}

		// Check for signal (signals are loaded at workflow start, no polling needed)
		const signal = await consumeSignal(this.storage, this.driver, signalName);
		if (signal) {
			const sleepEntry = getEntry(this.storage, sleepLocation)!;
			if (sleepEntry.kind.type === "sleep") {
				sleepEntry.kind.data.state = "interrupted";
			}
			sleepEntry.dirty = true;

			const signalEntry = createEntry(signalLocation, {
				type: "signal",
				data: { name: signalName, data: signal.data },
			});
			setEntry(this.storage, signalLocation, signalEntry);
			await flush(this.storage, this.driver);

			return signal.data as T;
		}

		// Signal not available, yield to scheduler until deadline
		throw new SleepError(deadline);
	}

	async listenNWithTimeout<T>(
		name: string,
		signalName: string,
		limit: number,
		timeoutMs: number,
	): Promise<T[]> {
		this.assertNotInProgress();
		this.checkEvicted();

		this.entryInProgress = true;
		try {
			return await this.executeListenNWithTimeout<T>(
				name,
				signalName,
				limit,
				timeoutMs,
			);
		} finally {
			this.entryInProgress = false;
		}
	}

	private async executeListenNWithTimeout<T>(
		name: string,
		signalName: string,
		limit: number,
		timeoutMs: number,
	): Promise<T[]> {
		// Check for duplicate name in current execution
		this.checkDuplicateName(name);

		// Use a sleep entry to store the deadline for replay
		const sleepLocation = appendName(this.storage, this.currentLocation, `${name}:deadline`);
		const sleepKey = locationToKey(this.storage, sleepLocation);
		const existingSleep = this.storage.history.entries.get(sleepKey);

		this.markVisited(sleepKey);

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
		}

		return this.executeListenNUntilImpl<T>(name, signalName, limit, deadline);
	}

	async listenNUntil<T>(
		name: string,
		signalName: string,
		limit: number,
		timestampMs: number,
	): Promise<T[]> {
		this.assertNotInProgress();
		this.checkEvicted();

		// Check for duplicate name in current execution
		this.checkDuplicateName(name);

		this.entryInProgress = true;
		try {
			return await this.executeListenNUntilImpl<T>(name, signalName, limit, timestampMs);
		} finally {
			this.entryInProgress = false;
		}
	}

	/**
	 * Internal implementation for listenNUntil with proper replay support.
	 * Stores the count and individual signals for deterministic replay.
	 */
	private async executeListenNUntilImpl<T>(
		name: string,
		signalName: string,
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

		if (existingCount && existingCount.kind.type === "signal") {
			// Replay: read all recorded signals
			const count = existingCount.kind.data.data as number;
			const results: T[] = [];

			for (let i = 0; i < count; i++) {
				const signalLocation = appendName(
					this.storage,
					this.currentLocation,
					`${name}:${i}`,
				);
				const signalKey = locationToKey(this.storage, signalLocation);

				this.markVisited(signalKey);

				const existingSignal = this.storage.history.entries.get(signalKey);
				if (existingSignal && existingSignal.kind.type === "signal") {
					results.push(existingSignal.kind.data.data as T);
				}
			}

			return results;
		}

		// New execution: collect signals until timeout or limit reached
		const results: T[] = [];

		for (let i = 0; i < limit; i++) {
			const now = Date.now();
			if (now >= deadline) {
				break;
			}

			// Try to consume a signal
			const signal = await consumeSignal(this.storage, this.driver, signalName);
			if (!signal) {
				// No signal available - check if we should wait
				if (results.length === 0) {
					// No signals yet - yield to scheduler until deadline
					throw new SleepError(deadline);
				}
				// We have some signals - return what we have
				break;
			}

			// Record the signal
			const signalLocation = appendName(
				this.storage,
				this.currentLocation,
				`${name}:${i}`,
			);
			const signalEntry = createEntry(signalLocation, {
				type: "signal",
				data: { name: signalName, data: signal.data },
			});
			setEntry(this.storage, signalLocation, signalEntry);
			this.markVisited(locationToKey(this.storage, signalLocation));

			results.push(signal.data as T);
		}

		// Record the count for replay
		const countEntry = createEntry(countLocation, {
			type: "signal",
			data: { name: `${signalName}:count`, data: results.length },
		});
		setEntry(this.storage, countLocation, countEntry);

		await flush(this.storage, this.driver);

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
						Object.keys(branches).map((k) => [k, { status: "pending" as const }]),
					),
				},
			});
			setEntry(this.storage, location, entry);
			entry.dirty = true;
		}

		if (entry.kind.type !== "join") {
			throw new HistoryDivergedError("Entry type mismatch");
		}

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
				const branchLocation = appendName(this.storage, location, branchName);
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
		await flush(this.storage, this.driver);

		// Throw if any branches failed
		if (Object.keys(errors).length > 0) {
			throw new JoinError(errors);
		}

		return results as { [K in keyof T]: BranchOutput<T[K]> };
	}

	// === Race ===

	async race<T>(
		name: string,
		branches: Array<{ name: string; run: (ctx: WorkflowContextInterface) => Promise<T> }>,
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
		branches: Array<{ name: string; run: (ctx: WorkflowContextInterface) => Promise<T> }>,
	): Promise<{ winner: string; value: T }> {
		// Check for duplicate name in current execution
		this.checkDuplicateName(name);

		const location = appendName(this.storage, this.currentLocation, name);
		const key = locationToKey(this.storage, location);
		const existing = this.storage.history.entries.get(key);

		// Mark this entry as visited for validateComplete
		this.markVisited(key);

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
				const winnerStatus = raceKind.data.branches[raceKind.data.winner];
				return {
					winner: raceKind.data.winner,
					value: winnerStatus.output as T,
				};
			}
		} else {
			entry = createEntry(location, {
				type: "race",
				data: {
					winner: null,
					branches: Object.fromEntries(
						branches.map((b) => [b.name, { status: "pending" as const }]),
					),
				},
			});
			setEntry(this.storage, location, entry);
			entry.dirty = true;
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

			const branchLocation = appendName(this.storage, location, branch.name);
			const branchCtx = this.createBranch(branchLocation, raceAbortController);

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

					if (error instanceof CancelledError || error instanceof EvictedError) {
						branchStatus.status = "cancelled";
					} else {
						branchStatus.status = "failed";
						branchStatus.error = String(error);

						if (settled) {
							// Track late errors for observability
							lateErrors.push({ name: branch.name, error: String(error) });
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

		// Clean up entries from non-winning branches
		if (winnerName !== null) {
			for (const branch of branches) {
				if (branch.name !== winnerName) {
					const branchLocation = appendName(this.storage, location, branch.name);
					await deleteEntriesWithPrefix(this.storage, this.driver, branchLocation);
				}
			}
		}

		// Flush final state
		await flush(this.storage, this.driver);

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
		await flush(this.storage, this.driver);
	}
}

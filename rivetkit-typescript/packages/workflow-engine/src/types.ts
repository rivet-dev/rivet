import type { Logger } from "pino";

/**
 * Index into the entry name registry.
 * Names are stored once and referenced by this index to avoid repetition.
 */
export type NameIndex = number;

/**
 * A segment in a location path.
 * Either a name index (for named entries) or a loop iteration marker.
 */
export type PathSegment = NameIndex | LoopIterationMarker;

/**
 * Marker for a loop iteration in a location path.
 */
export interface LoopIterationMarker {
	loop: NameIndex;
	iteration: number;
}

/**
 * Location identifies where an entry exists in the workflow execution tree.
 * It forms a path from the root through loops, joins, and branches.
 */
export type Location = PathSegment[];

/**
 * Current state of a sleep entry.
 */
export type SleepState = "pending" | "completed" | "interrupted";

/**
 * Status of an entry in the workflow.
 */
export type EntryStatus =
	| "pending"
	| "running"
	| "completed"
	| "failed"
	| "exhausted";

/**
 * Status of a branch in join/race.
 */
export type BranchStatusType =
	| "pending"
	| "running"
	| "completed"
	| "failed"
	| "cancelled";

/**
 * Current state of the workflow.
 */
export type WorkflowState =
	| "pending"
	| "running"
	| "sleeping"
	| "failed"
	| "completed"
	| "cancelled"
	| "rolling_back";

/**
 * Step entry data.
 */
export interface StepEntry {
	output?: unknown;
	error?: string;
}

/**
 * Loop entry data.
 */
export interface LoopEntry {
	state: unknown;
	iteration: number;
	output?: unknown;
}

/**
 * Sleep entry data.
 */
export interface SleepEntry {
	deadline: number;
	state: SleepState;
}

/**
 * Message entry data.
 */
export interface MessageEntry {
	name: string;
	data: unknown;
}

/**
 * Rollback checkpoint entry data.
 */
export interface RollbackCheckpointEntry {
	name: string;
}

/**
 * Branch status for join/race entries.
 */
export interface BranchStatus {
	status: BranchStatusType;
	output?: unknown;
	error?: string;
}

/**
 * Join entry data.
 */
export interface JoinEntry {
	branches: Record<string, BranchStatus>;
}

/**
 * Race entry data.
 */
export interface RaceEntry {
	winner: string | null;
	branches: Record<string, BranchStatus>;
}

/**
 * Removed entry data - placeholder for removed steps in workflow migrations.
 */
export interface RemovedEntry {
	originalType: EntryKindType;
	originalName?: string;
}

/**
 * All possible entry kind types.
 */
export type EntryKindType =
	| "step"
	| "loop"
	| "sleep"
	| "message"
	| "rollback_checkpoint"
	| "join"
	| "race"
	| "removed";

/**
 * Type-specific entry data.
 */
export type EntryKind =
	| { type: "step"; data: StepEntry }
	| { type: "loop"; data: LoopEntry }
	| { type: "sleep"; data: SleepEntry }
	| { type: "message"; data: MessageEntry }
	| { type: "rollback_checkpoint"; data: RollbackCheckpointEntry }
	| { type: "join"; data: JoinEntry }
	| { type: "race"; data: RaceEntry }
	| { type: "removed"; data: RemovedEntry };

/**
 * An entry in the workflow history.
 */
export interface Entry {
	id: string;
	location: Location;
	kind: EntryKind;
	dirty: boolean;
}

/**
 * Metadata for an entry (stored separately, lazily loaded).
 */
export interface EntryMetadata {
	status: EntryStatus;
	error?: string;
	attempts: number;
	lastAttemptAt: number;
	createdAt: number;
	completedAt?: number;
	rollbackCompletedAt?: number;
	rollbackError?: string;
	dirty: boolean;
}

/**
 * A message in the queue.
 */
export interface Message {
	/** Unique message ID (used as KV key). */
	id: string;
	name: string;
	data: unknown;
	sentAt: number;
}

/**
 * Workflow history - maps location keys to entries.
 */
export interface History {
	entries: Map<string, Entry>;
}

/**
 * History entry snapshot without internal dirty flags.
 */
export type WorkflowHistoryEntry = Omit<Entry, "dirty">;

/**
 * Entry metadata snapshot without internal dirty flags.
 */
export type WorkflowEntryMetadataSnapshot = Omit<EntryMetadata, "dirty">;

/**
 * Snapshot of workflow history for observers.
 */
export interface WorkflowHistorySnapshot {
	nameRegistry: string[];
	entries: WorkflowHistoryEntry[];
	entryMetadata: ReadonlyMap<string, WorkflowEntryMetadataSnapshot>;
}

/**
 * Structured error information for workflow failures.
 */
export interface WorkflowError {
	/** Error name/type (e.g., "TypeError", "CriticalError") */
	name: string;
	/** Error message */
	message: string;
	/** Stack trace if available */
	stack?: string;
	/** Custom error properties (for structured errors) */
	metadata?: Record<string, unknown>;
}

/**
 * Complete storage state for a workflow.
 */
export interface Storage {
	nameRegistry: string[];
	flushedNameCount: number;
	history: History;
	entryMetadata: Map<string, EntryMetadata>;
	messages: Message[];
	output?: unknown;
	state: WorkflowState;
	flushedState?: WorkflowState;
	error?: WorkflowError;
	flushedError?: WorkflowError;
	flushedOutput?: unknown;
}

/**
 * Driver interface for workflow message persistence.
 */
export interface WorkflowMessageDriver {
	loadMessages(): Promise<Message[]>;
	addMessage(message: Message): Promise<void>;
	/**
	 * Delete the specified messages and return the IDs that were successfully removed.
	 */
	deleteMessages(messageIds: string[]): Promise<string[]>;
}

/**
 * Context available to rollback handlers.
 */
export interface RollbackContextInterface {
	readonly workflowId: string;
	readonly abortSignal: AbortSignal;
	isEvicted(): boolean;
}

/**
 * Configuration for a step.
 */
export interface StepConfig<T> {
	name: string;
	run: () => Promise<T>;
	rollback?: (ctx: RollbackContextInterface, output: T) => Promise<void>;
	/** If true, step result is not persisted (use for idempotent operations). */
	ephemeral?: boolean;
	/** Maximum number of retry attempts (default: 3). */
	maxRetries?: number;
	/** Base delay in ms for exponential backoff (default: 100). */
	retryBackoffBase?: number;
	/** Maximum delay in ms for exponential backoff (default: 30000). */
	retryBackoffMax?: number;
	/** Timeout in ms for step execution (default: 30000). Set to 0 to disable. */
	timeout?: number;
}

/**
 * Result from a loop iteration.
 */
export type LoopResult<S, T> =
	| { continue: true; state: S }
	| { break: true; value: T };

/**
 * Configuration for a loop.
 */
export interface LoopConfig<S, T> {
	name: string;
	state?: S;
	run: (ctx: WorkflowContextInterface, state: S) => Promise<LoopResult<S, T>>;
	commitInterval?: number;
	/** Trim loop history every N iterations. Defaults to commitInterval or 20. */
	historyEvery?: number;
	/** Retain the last N iterations of history. Defaults to commitInterval or 20. */
	historyKeep?: number;
}

/**
 * Configuration for a branch in join/race.
 */
export interface BranchConfig<T> {
	run: (ctx: WorkflowContextInterface) => Promise<T>;
}

/**
 * Extract the output type from a BranchConfig.
 */
export type BranchOutput<T> = T extends BranchConfig<infer O> ? O : never;

/**
 * The workflow context interface exposed to workflow functions.
 */
export interface WorkflowContextInterface {
	readonly workflowId: string;
	readonly abortSignal: AbortSignal;

	step<T>(name: string, run: () => Promise<T>): Promise<T>;
	step<T>(config: StepConfig<T>): Promise<T>;

	loop<T>(
		name: string,
		run: (
			ctx: WorkflowContextInterface,
		) => Promise<LoopResult<undefined, T>>,
	): Promise<T>;
	loop<S, T>(config: LoopConfig<S, T>): Promise<T>;

	sleep(name: string, durationMs: number): Promise<void>;
	sleepUntil(name: string, timestampMs: number): Promise<void>;

	listen<T>(name: string, messageName: string): Promise<T>;
	listenN<T>(name: string, messageName: string, limit: number): Promise<T[]>;
	listenWithTimeout<T>(
		name: string,
		messageName: string,
		timeoutMs: number,
	): Promise<T | null>;
	listenUntil<T>(
		name: string,
		messageName: string,
		timestampMs: number,
	): Promise<T | null>;
	listenNWithTimeout<T>(
		name: string,
		messageName: string,
		limit: number,
		timeoutMs: number,
	): Promise<T[]>;
	listenNUntil<T>(
		name: string,
		messageName: string,
		limit: number,
		timestampMs: number,
	): Promise<T[]>;

	rollbackCheckpoint(name: string): Promise<void>;

	join<T extends Record<string, BranchConfig<unknown>>>(
		name: string,
		branches: T,
	): Promise<{ [K in keyof T]: BranchOutput<T[K]> }>;

	race<T>(
		name: string,
		branches: Array<{
			name: string;
			run: (ctx: WorkflowContextInterface) => Promise<T>;
		}>,
	): Promise<{ winner: string; value: T }>;

	removed(name: string, originalType: EntryKindType): Promise<void>;

	isEvicted(): boolean;
}

/**
 * Workflow function type.
 */
export type WorkflowRunMode = "yield" | "live";

export interface RunWorkflowOptions {
	mode?: WorkflowRunMode;
	logger?: Logger;
	onHistoryUpdated?: (history: WorkflowHistorySnapshot) => void;
}

export type WorkflowFunction<TInput = unknown, TOutput = unknown> = (
	ctx: WorkflowContextInterface,
	input: TInput,
) => Promise<TOutput>;

/**
 * Result returned when a workflow run completes or yields.
 */
export interface WorkflowResult<TOutput = unknown> {
	state: WorkflowState;
	output?: TOutput;
	sleepUntil?: number;
	waitingForMessages?: string[];
}

/**
 * Handle for managing a running workflow.
 *
 * Returned by `runWorkflow()`. The workflow starts executing immediately.
 * Use `.result` to await completion, and other methods to interact with
 * the running workflow.
 */
export interface WorkflowHandle<TOutput = unknown> {
	readonly workflowId: string;

	/**
	 * Promise that resolves when the workflow completes or yields.
	 */
	readonly result: Promise<WorkflowResult<TOutput>>;

	/**
	 * Send a message to the workflow.
	 * The message is persisted and will be available on the next run.
	 * In live mode, this also wakes the workflow if it's waiting.
	 */
	message(name: string, data: unknown): Promise<void>;

	/**
	 * Wake the workflow immediately by setting an alarm for now.
	 */
	wake(): Promise<void>;

	/**
	 * Reset exhausted retries and schedule the workflow to run again.
	 */
	recover(): Promise<void>;

	/**
	 * Request the workflow to stop gracefully.
	 * The workflow will throw EvictedError at its next yield point,
	 * flush its state, and resolve the result promise.
	 */
	evict(): void;

	/**
	 * Cancel the workflow permanently.
	 * Sets the workflow state to "cancelled" and clears any pending alarms.
	 * Unlike evict(), this marks the workflow as permanently stopped.
	 */
	cancel(): Promise<void>;

	/**
	 * Get the workflow output if completed.
	 */
	getOutput(): Promise<TOutput | undefined>;

	/**
	 * Get the current workflow state.
	 */
	getState(): Promise<WorkflowState>;
}

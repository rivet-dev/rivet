/**
 * Vercel Workflow SDK `World` types.
 *
 * These mirror the public contract from `@workflow/world`. We redeclare them
 * locally so this package does not hard-depend on the SDK. The shapes are
 * derived from https://useworkflow.dev/docs/deploying/building-a-world and
 * the Postgres and Local reference worlds.
 *
 * When the upstream types drift, update this file to keep parity.
 */

// ---------------------------------------------------------------------------
// Primitive types
// ---------------------------------------------------------------------------

export type MessageId = string;

/** Queue name, always prefixed with `__wkf_workflow_` or `__wkf_step_`. */
export type ValidQueueName = `__wkf_workflow_${string}` | `__wkf_step_${string}`;

/** Queue prefix used when registering a handler. */
export type QueuePrefix = "__wkf_workflow_" | "__wkf_step_";

export interface PaginatedResponse<T> {
	data: T[];
	cursor: string | null;
	hasMore: boolean;
}

// ---------------------------------------------------------------------------
// Run / Step / Hook state
// ---------------------------------------------------------------------------

export type WorkflowRunStatus =
	| "pending"
	| "running"
	| "completed"
	| "failed"
	| "cancelled";

export type StepStatus =
	| "pending"
	| "running"
	| "completed"
	| "failed"
	| "cancelled";

export type HookStatus = "pending" | "triggered" | "disposed";

export interface WorkflowRun {
	id: string;
	workflowName: string;
	status: WorkflowRunStatus;
	input?: unknown;
	output?: unknown;
	error?: WorkflowRunError;
	createdAt: Date;
	updatedAt: Date;
	startedAt?: Date;
	finishedAt?: Date;
	deploymentId?: string;
	parentRunId?: string;
	parentStepId?: string;
	traceCarrier?: Record<string, string>;
	metadata?: Record<string, unknown>;
}

export interface WorkflowRunError {
	message: string;
	stack?: string;
	name?: string;
	cause?: unknown;
}

export interface Step {
	id: string;
	runId: string;
	name: string;
	status: StepStatus;
	input?: unknown;
	output?: unknown;
	error?: WorkflowRunError;
	createdAt: Date;
	updatedAt: Date;
	startedAt?: Date;
	finishedAt?: Date;
	attempt: number;
	parentStepId?: string;
}

export interface Hook {
	id: string;
	runId: string;
	token: string;
	name: string;
	status: HookStatus;
	createdAt: Date;
	updatedAt: Date;
	disposedAt?: Date;
	triggeredAt?: Date;
	metadata?: Record<string, unknown>;
}

// ---------------------------------------------------------------------------
// Events (append-only log)
// ---------------------------------------------------------------------------

export type EventType =
	| "run_created"
	| "run_started"
	| "run_completed"
	| "run_failed"
	| "run_cancelled"
	| "run_updated"
	| "step_created"
	| "step_started"
	| "step_completed"
	| "step_failed"
	| "step_cancelled"
	| "step_retrying"
	| "hook_created"
	| "hook_triggered"
	| "hook_disposed"
	| "hook_conflict"
	| "stream_chunk"
	| "stream_closed";

export interface Event {
	id: string;
	type: EventType;
	runId: string;
	stepId?: string;
	hookId?: string;
	correlationId?: string;
	data: unknown;
	createdAt: Date;
}

export interface EventResult {
	event: Event;
	/** Present when an event results in a newly created run. */
	run?: WorkflowRun;
}

export interface RunCreatedEventRequest {
	type: "run_created";
	workflowName: string;
	input?: unknown;
	deploymentId?: string;
	parentRunId?: string;
	parentStepId?: string;
	traceCarrier?: Record<string, string>;
	metadata?: Record<string, unknown>;
}

export interface CreateEventRequest {
	type: Exclude<EventType, "run_created">;
	stepId?: string;
	hookId?: string;
	correlationId?: string;
	data?: unknown;
}

// ---------------------------------------------------------------------------
// Query parameters
// ---------------------------------------------------------------------------

export interface GetWorkflowRunParams {
	includeEvents?: boolean;
}

export interface ListWorkflowRunsParams {
	cursor?: string;
	limit?: number;
	workflowName?: string;
	status?: WorkflowRunStatus;
	deploymentId?: string;
	parentRunId?: string;
	createdAfter?: Date;
	createdBefore?: Date;
}

export interface GetStepParams {
	includeEvents?: boolean;
}

export interface ListWorkflowRunStepsParams {
	runId: string;
	cursor?: string;
	limit?: number;
	status?: StepStatus;
}

export interface ListEventsParams {
	runId: string;
	cursor?: string;
	limit?: number;
	types?: EventType[];
	stepId?: string;
	hookId?: string;
}

export interface ListEventsByCorrelationIdParams {
	correlationId: string;
	cursor?: string;
	limit?: number;
}

export interface CreateEventParams {
	idempotencyKey?: string;
}

export interface GetHookParams {
	includeEvents?: boolean;
}

export interface ListHooksParams {
	runId: string;
	cursor?: string;
	limit?: number;
	status?: HookStatus;
}

// ---------------------------------------------------------------------------
// Queue payloads
// ---------------------------------------------------------------------------

export interface WorkflowInvokePayload {
	runId: string;
	traceCarrier?: Record<string, string>;
	requestedAt?: Date;
}

export interface StepInvokePayload {
	workflowName: string;
	workflowRunId: string;
	workflowStartedAt: number;
	stepId: string;
	traceCarrier?: Record<string, string>;
	requestedAt?: Date;
}

export type QueuePayload = WorkflowInvokePayload | StepInvokePayload;

export interface QueueRetryPolicy {
	maxAttempts?: number;
	initialBackoffMs?: number;
	maxBackoffMs?: number;
	backoffMultiplier?: number;
}

export interface QueueOptions {
	idempotencyKey?: string;
	delay?: number;
	retryPolicy?: QueueRetryPolicy;
}

export interface QueueMessageMeta {
	attempt: number;
	queueName: ValidQueueName;
	messageId: MessageId;
}

export type QueueHandler = (
	message: unknown,
	meta: QueueMessageMeta,
) => Promise<void | { timeoutSeconds: number }>;

// ---------------------------------------------------------------------------
// World interface
// ---------------------------------------------------------------------------

export interface Storage {
	runs: {
		get(id: string, params?: GetWorkflowRunParams): Promise<WorkflowRun>;
		list(
			params?: ListWorkflowRunsParams,
		): Promise<PaginatedResponse<WorkflowRun>>;
	};
	steps: {
		get(
			runId: string | undefined,
			stepId: string,
			params?: GetStepParams,
		): Promise<Step>;
		list(
			params: ListWorkflowRunStepsParams,
		): Promise<PaginatedResponse<Step>>;
	};
	events: {
		create(
			runId: string | null,
			data: RunCreatedEventRequest,
			params?: CreateEventParams,
		): Promise<EventResult>;
		create(
			runId: string,
			data: CreateEventRequest,
			params?: CreateEventParams,
		): Promise<EventResult>;
		list(params: ListEventsParams): Promise<PaginatedResponse<Event>>;
		listByCorrelationId(
			params: ListEventsByCorrelationIdParams,
		): Promise<PaginatedResponse<Event>>;
	};
	hooks: {
		get(hookId: string, params?: GetHookParams): Promise<Hook>;
		getByToken(token: string, params?: GetHookParams): Promise<Hook>;
		list(params: ListHooksParams): Promise<PaginatedResponse<Hook>>;
	};
}

export interface Queue {
	getDeploymentId(): Promise<string>;
	queue(
		queueName: ValidQueueName,
		message: QueuePayload,
		opts?: QueueOptions,
	): Promise<{ messageId: MessageId }>;
	createQueueHandler(
		queueNamePrefix: QueuePrefix,
		handler: QueueHandler,
	): (req: Request) => Promise<Response>;
}

export interface StreamChunk {
	index: number;
	data: Uint8Array;
}

export interface StreamInfo {
	tailIndex: number;
	done: boolean;
}

export interface StreamChunksResult {
	data: StreamChunk[];
	cursor: string | null;
	hasMore: boolean;
	done: boolean;
}

export interface Streamer {
	writeToStream(
		name: string,
		runId: string,
		chunk: string | Uint8Array,
	): Promise<void>;
	writeToStreamMulti?(
		name: string,
		runId: string,
		chunks: (string | Uint8Array)[],
	): Promise<void>;
	closeStream(name: string, runId: string): Promise<void>;
	readFromStream(
		name: string,
		startIndex?: number,
	): Promise<ReadableStream<Uint8Array>>;
	listStreamsByRunId(runId: string): Promise<string[]>;
	getStreamChunks(
		name: string,
		runId: string,
		options?: { limit?: number; cursor?: string },
	): Promise<StreamChunksResult>;
	getStreamInfo(name: string, runId: string): Promise<StreamInfo>;
}

export interface World extends Storage, Queue, Streamer {
	start?(): Promise<void>;
	close?(): Promise<void>;
	getEncryptionKeyForRun?(run: WorkflowRun): Promise<Uint8Array | undefined>;
}

// ---------------------------------------------------------------------------
// Error shapes
// ---------------------------------------------------------------------------

export class WorldError extends Error {
	code: string;
	constructor(code: string, message: string) {
		super(message);
		this.name = "WorldError";
		this.code = code;
	}
}

export class NotFoundError extends WorldError {
	constructor(entity: string, id: string) {
		super("not_found", `${entity} ${id} not found`);
		this.name = "NotFoundError";
	}
}

export class HookConflictError extends WorldError {
	constructor(token: string) {
		super("hook_conflict", `hook token ${token} already registered`);
		this.name = "HookConflictError";
	}
}

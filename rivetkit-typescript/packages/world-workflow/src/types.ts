/**
 * Vercel Workflow SDK `World` types.
 *
 * These mirror the public contract from `@workflow/world` (packages/world/src/).
 * When the upstream types drift, update this file to keep parity.
 *
 * Source: https://github.com/vercel/workflow/tree/main/packages/world/src
 */

// ---------------------------------------------------------------------------
// Shared primitives
// ---------------------------------------------------------------------------

export type SerializedData = Uint8Array | unknown;

export type ResolveData = "none" | "all";

export interface PaginationOptions {
	limit?: number;
	cursor?: string;
	sortOrder?: "asc" | "desc";
}

export interface PaginatedResponse<T> {
	data: T[];
	cursor: string | null;
	hasMore: boolean;
}

export interface StructuredError {
	message: string;
	stack?: string;
	code?: string;
}

export type MessageId = string;

export type ValidQueueName =
	| `__wkf_workflow_${string}`
	| `__wkf_step_${string}`;

export type QueuePrefix = "__wkf_workflow_" | "__wkf_step_";

export type TraceCarrier = Record<string, string>;

// ---------------------------------------------------------------------------
// WorkflowRun
// ---------------------------------------------------------------------------

export type WorkflowRunStatus =
	| "pending"
	| "running"
	| "completed"
	| "failed"
	| "cancelled";

export interface WorkflowRun {
	runId: string;
	status: WorkflowRunStatus;
	deploymentId: string;
	workflowName: string;
	specVersion?: number;
	executionContext?: Record<string, unknown>;
	input: SerializedData;
	output?: SerializedData;
	error?: StructuredError;
	expiredAt?: Date;
	startedAt?: Date;
	completedAt?: Date;
	createdAt: Date;
	updatedAt: Date;
}

export type WorkflowRunWithoutData = Omit<WorkflowRun, "input" | "output"> & {
	input: undefined;
	output: undefined;
};

// ---------------------------------------------------------------------------
// Step
// ---------------------------------------------------------------------------

export type StepStatus =
	| "pending"
	| "running"
	| "completed"
	| "failed"
	| "cancelled";

export interface Step {
	runId: string;
	stepId: string;
	stepName: string;
	status: StepStatus;
	input: SerializedData;
	output?: SerializedData;
	error?: StructuredError;
	attempt: number;
	startedAt?: Date;
	completedAt?: Date;
	createdAt: Date;
	updatedAt: Date;
	retryAfter?: Date;
	specVersion?: number;
}

export type StepWithoutData = Omit<Step, "input" | "output"> & {
	input: undefined;
	output: undefined;
};

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

export interface Hook {
	runId: string;
	hookId: string;
	token: string;
	ownerId: string;
	projectId: string;
	environment: string;
	metadata?: SerializedData;
	createdAt: Date;
	specVersion?: number;
	isWebhook?: boolean;
}

// ---------------------------------------------------------------------------
// Wait
// ---------------------------------------------------------------------------

export type WaitStatus = "waiting" | "completed";

export interface Wait {
	waitId: string;
	runId: string;
	status: WaitStatus;
	resumeAt?: Date;
	completedAt?: Date;
	createdAt: Date;
	updatedAt: Date;
	specVersion?: number;
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

export type EventType =
	| "run_created"
	| "run_started"
	| "run_completed"
	| "run_failed"
	| "run_cancelled"
	| "step_created"
	| "step_started"
	| "step_completed"
	| "step_failed"
	| "step_retrying"
	| "hook_created"
	| "hook_received"
	| "hook_disposed"
	| "hook_conflict"
	| "wait_created"
	| "wait_completed";

export interface Event {
	runId: string;
	eventId: string;
	eventType: EventType;
	correlationId?: string;
	eventData?: unknown;
	createdAt: Date;
	specVersion?: number;
}

export interface EventResult {
	event?: Event;
	run?: WorkflowRun;
	step?: Step;
	hook?: Hook;
	wait?: Wait;
	events?: Event[];
}

// ---------------------------------------------------------------------------
// Event request types
// ---------------------------------------------------------------------------

export interface RunCreatedEventRequest {
	eventType: "run_created";
	eventData: {
		deploymentId: string;
		workflowName: string;
		input: SerializedData;
		executionContext?: Record<string, unknown>;
	};
}

export interface CreateEventRequest {
	eventType: Exclude<EventType, "run_created">;
	correlationId?: string;
	eventData?: unknown;
}

// ---------------------------------------------------------------------------
// Query parameters
// ---------------------------------------------------------------------------

export interface GetWorkflowRunParams {
	resolveData?: ResolveData;
}

export interface ListWorkflowRunsParams {
	workflowName?: string;
	status?: WorkflowRunStatus;
	pagination?: PaginationOptions;
	resolveData?: ResolveData;
}

export interface CreateWorkflowRunRequest {
	deploymentId: string;
	workflowName: string;
	input: SerializedData;
	executionContext?: SerializedData;
	specVersion?: number;
}

export interface GetStepParams {
	resolveData?: ResolveData;
}

export interface ListWorkflowRunStepsParams {
	runId: string;
	pagination?: PaginationOptions;
	resolveData?: ResolveData;
}

export interface CreateEventParams {
	v1Compat?: boolean;
	resolveData?: ResolveData;
	requestId?: string;
}

export interface GetEventParams {
	resolveData?: ResolveData;
}

export interface ListEventsParams {
	runId: string;
	pagination?: PaginationOptions;
	resolveData?: ResolveData;
}

export interface ListEventsByCorrelationIdParams {
	correlationId: string;
	pagination?: PaginationOptions;
	resolveData?: ResolveData;
}

export interface GetHookParams {
	resolveData?: ResolveData;
}

export interface ListHooksParams {
	runId?: string;
	pagination?: PaginationOptions;
	resolveData?: ResolveData;
}

// ---------------------------------------------------------------------------
// Queue payloads
// ---------------------------------------------------------------------------

export interface RunInput {
	input: unknown;
	deploymentId: string;
	workflowName: string;
	specVersion: number;
	executionContext?: Record<string, unknown>;
}

export interface WorkflowInvokePayload {
	runId: string;
	traceCarrier?: TraceCarrier;
	requestedAt?: Date;
	serverErrorRetryCount?: number;
	runInput?: RunInput;
}

export interface StepInvokePayload {
	workflowName: string;
	workflowRunId: string;
	workflowStartedAt: number;
	stepId: string;
	traceCarrier?: TraceCarrier;
	requestedAt?: Date;
}

export interface HealthCheckPayload {
	__healthCheck: true;
	correlationId: string;
}

export type QueuePayload =
	| WorkflowInvokePayload
	| StepInvokePayload
	| HealthCheckPayload;

export interface QueueOptions {
	deploymentId?: string;
	idempotencyKey?: string;
	headers?: Record<string, string>;
	delaySeconds?: number;
	specVersion?: number;
}

export interface QueueMessageMeta {
	attempt: number;
	queueName: ValidQueueName;
	messageId: MessageId;
	requestId?: string;
}

export type QueueHandler = (
	message: unknown,
	meta: QueueMessageMeta,
) => Promise<void | { timeoutSeconds: number }>;

// ---------------------------------------------------------------------------
// Stream types
// ---------------------------------------------------------------------------

export interface StreamChunk {
	index: number;
	data: Uint8Array;
}

export interface GetChunksOptions {
	limit?: number;
	cursor?: string;
}

export interface StreamInfoResponse {
	tailIndex: number;
	done: boolean;
}

export interface StreamChunksResponse {
	data: StreamChunk[];
	cursor: string | null;
	hasMore: boolean;
	done: boolean;
}

// ---------------------------------------------------------------------------
// World interface
// ---------------------------------------------------------------------------

export interface Storage {
	runs: {
		get(
			id: string,
			params?: GetWorkflowRunParams,
		): Promise<WorkflowRun | WorkflowRunWithoutData>;
		list(
			params?: ListWorkflowRunsParams,
		): Promise<PaginatedResponse<WorkflowRun | WorkflowRunWithoutData>>;
	};
	steps: {
		get(
			runId: string,
			stepId: string,
			params?: GetStepParams,
		): Promise<Step | StepWithoutData>;
		list(
			params: ListWorkflowRunStepsParams,
		): Promise<PaginatedResponse<Step | StepWithoutData>>;
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
		get(
			runId: string,
			eventId: string,
			params?: GetEventParams,
		): Promise<Event>;
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
	): Promise<{ messageId: MessageId | null }>;
	createQueueHandler(
		queueNamePrefix: QueuePrefix,
		handler: QueueHandler,
	): (req: Request) => Promise<Response>;
}

export interface Streamer {
	streamFlushIntervalMs?: number;
	streams: {
		write(
			runId: string,
			name: string,
			chunk: string | Uint8Array,
		): Promise<void>;
		writeMulti?(
			runId: string,
			name: string,
			chunks: (string | Uint8Array)[],
		): Promise<void>;
		close(runId: string, name: string): Promise<void>;
		get(
			runId: string,
			name: string,
			startIndex?: number,
		): Promise<ReadableStream<Uint8Array>>;
		list(runId: string): Promise<string[]>;
		getChunks(
			runId: string,
			name: string,
			options?: GetChunksOptions,
		): Promise<StreamChunksResponse>;
		getInfo(
			runId: string,
			name: string,
		): Promise<StreamInfoResponse>;
	};
}

export interface World extends Storage, Queue, Streamer {
	specVersion?: number;
	start?(): Promise<void>;
	close?(): Promise<void>;
	resolveLatestDeploymentId?(): Promise<string>;
	getEncryptionKeyForRun?(
		run: WorkflowRun,
	): Promise<Uint8Array | undefined>;
	getEncryptionKeyForRun?(
		runId: string,
		context?: Record<string, unknown>,
	): Promise<Uint8Array | undefined>;
}

// ---------------------------------------------------------------------------
// Spec versions
// ---------------------------------------------------------------------------

export const SPEC_VERSION_LEGACY = 1;
export const SPEC_VERSION_SUPPORTS_EVENT_SOURCING = 2;
export const SPEC_VERSION_SUPPORTS_CBOR_QUEUE_TRANSPORT = 3;
export const SPEC_VERSION_CURRENT = SPEC_VERSION_SUPPORTS_CBOR_QUEUE_TRANSPORT;

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

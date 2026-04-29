/**
 * WorkflowRun actor.
 *
 * One actor per workflow run, keyed by `[runId]`. Owns:
 *
 * - Append-only event log for this run
 * - Materialized run, steps, and hooks state
 * - Named streams keyed by stream name
 *
 * All mutations go through `createEvent` so the event log remains the source
 * of truth.
 */

import { actor, event } from "rivetkit";
import { v4 as uuidv4 } from "uuid";
import type {
	CreateEventRequest,
	Event as WorldEvent,
	EventResult,
	EventType,
	Hook,
	RunCreatedEventRequest,
	Step,
	StepStatus,
	Wait,
	WaitStatus,
	WorkflowRun,
	WorkflowRunStatus,
} from "../types";
import { encodeBinary, nowMs } from "./shared";

// ---------------------------------------------------------------------------
// Persisted state shape
// ---------------------------------------------------------------------------

interface PersistedRun {
	runId: string;
	workflowName: string;
	status: WorkflowRunStatus;
	deploymentId: string;
	input?: unknown;
	output?: unknown;
	error?: unknown;
	executionContext?: Record<string, unknown>;
	specVersion?: number;
	createdAt: number;
	updatedAt: number;
	startedAt?: number;
	completedAt?: number;
	expiredAt?: number;
}

interface PersistedStep {
	stepId: string;
	runId: string;
	stepName: string;
	status: StepStatus;
	input?: unknown;
	output?: unknown;
	error?: unknown;
	createdAt: number;
	updatedAt: number;
	startedAt?: number;
	completedAt?: number;
	retryAfter?: number;
	attempt: number;
	specVersion?: number;
}

interface PersistedHook {
	hookId: string;
	runId: string;
	token: string;
	ownerId: string;
	projectId: string;
	environment: string;
	metadata?: unknown;
	createdAt: number;
	specVersion?: number;
	isWebhook?: boolean;
}

interface PersistedWait {
	waitId: string;
	runId: string;
	status: WaitStatus;
	resumeAt?: number;
	completedAt?: number;
	createdAt: number;
	updatedAt: number;
	specVersion?: number;
}

interface PersistedEvent {
	eventId: string;
	eventType: EventType;
	runId: string;
	correlationId?: string;
	eventData?: unknown;
	createdAt: number;
	specVersion?: number;
}

interface PersistedStreamChunk {
	index: number;
	data: string;
}

interface PersistedStream {
	name: string;
	chunks: PersistedStreamChunk[];
	tailIndex: number;
	done: boolean;
}

interface WorkflowRunState {
	initialized: boolean;
	run?: PersistedRun;
	steps: Record<string, PersistedStep>;
	hooks: Record<string, PersistedHook>;
	waits: Record<string, PersistedWait>;
	events: PersistedEvent[];
	idempotencyKeys: Record<string, string>;
	streams: Record<string, PersistedStream>;
}

// ---------------------------------------------------------------------------
// Terminal state helper
// ---------------------------------------------------------------------------

const TERMINAL_RUN_STATUSES: readonly WorkflowRunStatus[] = [
	"completed",
	"failed",
	"cancelled",
];

function isTerminalRunStatus(status: WorkflowRunStatus): boolean {
	return TERMINAL_RUN_STATUSES.includes(status);
}

// ---------------------------------------------------------------------------
// Mapping helpers
// ---------------------------------------------------------------------------

function runToPublic(run: PersistedRun): WorkflowRun {
	return {
		runId: run.runId,
		workflowName: run.workflowName,
		status: run.status,
		deploymentId: run.deploymentId,
		input: run.input,
		output: run.output,
		error: run.error as WorkflowRun["error"],
		executionContext: run.executionContext,
		specVersion: run.specVersion,
		createdAt: new Date(run.createdAt),
		updatedAt: new Date(run.updatedAt),
		startedAt: run.startedAt ? new Date(run.startedAt) : undefined,
		completedAt: run.completedAt ? new Date(run.completedAt) : undefined,
		expiredAt: run.expiredAt ? new Date(run.expiredAt) : undefined,
	};
}

function stepToPublic(step: PersistedStep): Step {
	return {
		stepId: step.stepId,
		runId: step.runId,
		stepName: step.stepName,
		status: step.status,
		input: step.input,
		output: step.output,
		error: step.error as Step["error"],
		createdAt: new Date(step.createdAt),
		updatedAt: new Date(step.updatedAt),
		startedAt: step.startedAt ? new Date(step.startedAt) : undefined,
		completedAt: step.completedAt
			? new Date(step.completedAt)
			: undefined,
		retryAfter: step.retryAfter ? new Date(step.retryAfter) : undefined,
		attempt: step.attempt,
		specVersion: step.specVersion,
	};
}

function hookToPublic(hook: PersistedHook): Hook {
	return {
		hookId: hook.hookId,
		runId: hook.runId,
		token: hook.token,
		ownerId: hook.ownerId,
		projectId: hook.projectId,
		environment: hook.environment,
		metadata: hook.metadata,
		createdAt: new Date(hook.createdAt),
		specVersion: hook.specVersion,
		isWebhook: hook.isWebhook,
	};
}

function waitToPublic(wait: PersistedWait): Wait {
	return {
		waitId: wait.waitId,
		runId: wait.runId,
		status: wait.status,
		resumeAt: wait.resumeAt ? new Date(wait.resumeAt) : undefined,
		completedAt: wait.completedAt
			? new Date(wait.completedAt)
			: undefined,
		createdAt: new Date(wait.createdAt),
		updatedAt: new Date(wait.updatedAt),
		specVersion: wait.specVersion,
	};
}

function eventToPublic(e: PersistedEvent): WorldEvent {
	return {
		eventId: e.eventId,
		eventType: e.eventType,
		runId: e.runId,
		correlationId: e.correlationId,
		eventData: e.eventData,
		createdAt: new Date(e.createdAt),
		specVersion: e.specVersion,
	};
}

// ---------------------------------------------------------------------------
// Event materialization
// ---------------------------------------------------------------------------

interface MaterializeArgs {
	state: WorkflowRunState;
	event: PersistedEvent;
	data: CreateEventRequest | RunCreatedEventRequest;
}

function materializeEvent(args: MaterializeArgs): void {
	const { state, event: ev, data } = args;
	const now = ev.createdAt;

	if (data.eventType === "run_created") {
		if (state.run) return;
		const ed = data.eventData as {
			deploymentId: string;
			workflowName: string;
			input?: unknown;
			executionContext?: Record<string, unknown>;
		};
		state.run = {
			runId: ev.runId,
			workflowName: ed.workflowName,
			deploymentId: ed.deploymentId,
			status: "pending",
			input: ed.input,
			executionContext: ed.executionContext,
			specVersion: ev.specVersion,
			createdAt: now,
			updatedAt: now,
		};
		return;
	}

	if (!state.run) return;

	switch (data.eventType) {
		case "run_started": {
			state.run.status = "running";
			state.run.startedAt ??= now;
			state.run.updatedAt = now;
			if (data.eventData) {
				const ed = data.eventData as {
					deploymentId?: string;
					input?: unknown;
					executionContext?: Record<string, unknown>;
				};
				if (ed.deploymentId) state.run.deploymentId = ed.deploymentId;
				if (ed.input !== undefined) state.run.input = ed.input;
				if (ed.executionContext)
					state.run.executionContext = ed.executionContext;
			}
			break;
		}
		case "run_completed":
			state.run.status = "completed";
			state.run.completedAt = now;
			state.run.updatedAt = now;
			if (data.eventData) {
				const ed = data.eventData as { output?: unknown };
				if (ed.output !== undefined) state.run.output = ed.output;
			}
			break;
		case "run_failed":
			state.run.status = "failed";
			state.run.completedAt = now;
			state.run.updatedAt = now;
			if (data.eventData) {
				state.run.error = data.eventData;
			}
			break;
		case "run_cancelled":
			state.run.status = "cancelled";
			state.run.completedAt = now;
			state.run.updatedAt = now;
			break;
		case "step_created": {
			const corrId = data.correlationId;
			if (!corrId) break;
			const ed = (data.eventData ?? {}) as {
				stepName?: string;
				input?: unknown;
			};
			state.steps[corrId] = {
				stepId: corrId,
				runId: ev.runId,
				stepName: ed.stepName ?? "step",
				status: "pending",
				input: ed.input,
				createdAt: now,
				updatedAt: now,
				attempt: 0,
				specVersion: ev.specVersion,
			};
			break;
		}
		case "step_started": {
			const step = data.correlationId
				? state.steps[data.correlationId]
				: undefined;
			if (!step) break;
			step.status = "running";
			step.startedAt ??= now;
			step.updatedAt = now;
			if (data.eventData) {
				const ed = data.eventData as { attempt?: number };
				if (ed.attempt !== undefined) step.attempt = ed.attempt;
			} else {
				step.attempt += 1;
			}
			break;
		}
		case "step_completed": {
			const step = data.correlationId
				? state.steps[data.correlationId]
				: undefined;
			if (!step) break;
			step.status = "completed";
			step.completedAt = now;
			step.updatedAt = now;
			if (data.eventData) {
				const ed = data.eventData as { result?: unknown };
				step.output = ed.result;
			}
			break;
		}
		case "step_failed": {
			const step = data.correlationId
				? state.steps[data.correlationId]
				: undefined;
			if (!step) break;
			step.status = "failed";
			step.completedAt = now;
			step.updatedAt = now;
			step.error = data.eventData;
			break;
		}
		case "step_retrying": {
			const step = data.correlationId
				? state.steps[data.correlationId]
				: undefined;
			if (!step) break;
			step.status = "pending";
			step.updatedAt = now;
			step.error = data.eventData;
			if (data.eventData) {
				const ed = data.eventData as { retryAfter?: string | number };
				if (ed.retryAfter)
					step.retryAfter = new Date(ed.retryAfter).getTime();
			}
			break;
		}
		case "hook_created": {
			const corrId = data.correlationId;
			if (!corrId) break;
			const ed = (data.eventData ?? {}) as {
				token?: string;
				metadata?: unknown;
				isWebhook?: boolean;
			};
			state.hooks[corrId] = {
				hookId: corrId,
				runId: ev.runId,
				token: ed.token ?? corrId,
				ownerId: "",
				projectId: "",
				environment: "",
				metadata: ed.metadata,
				createdAt: now,
				specVersion: ev.specVersion,
				isWebhook: ed.isWebhook,
			};
			break;
		}
		case "hook_received":
		case "hook_disposed":
		case "hook_conflict":
			break;
		case "wait_created": {
			const corrId = data.correlationId;
			if (!corrId) break;
			const ed = (data.eventData ?? {}) as {
				resumeAt?: string | number;
			};
			state.waits[corrId] = {
				waitId: corrId,
				runId: ev.runId,
				status: "waiting",
				resumeAt: ed.resumeAt
					? new Date(ed.resumeAt).getTime()
					: undefined,
				createdAt: now,
				updatedAt: now,
				specVersion: ev.specVersion,
			};
			break;
		}
		case "wait_completed": {
			const wait = data.correlationId
				? state.waits[data.correlationId]
				: undefined;
			if (!wait) break;
			wait.status = "completed";
			wait.completedAt = now;
			wait.updatedAt = now;
			break;
		}
	}

	// Auto-dispose hooks when the run enters a terminal status.
	if (state.run && isTerminalRunStatus(state.run.status)) {
		// Hook disposal is tracked via hook_disposed events; we do not
		// delete them from state here so they remain queryable.
	}
}

// ---------------------------------------------------------------------------
// Actor definition
// ---------------------------------------------------------------------------

export const workflowRunActor = actor({
	state: {
		initialized: false,
		steps: {},
		hooks: {},
		waits: {},
		events: [],
		idempotencyKeys: {},
		streams: {},
	} as WorkflowRunState,
	events: {
		streamAppended: event<{
			streamName: string;
			chunks: PersistedStreamChunk[];
			done: boolean;
		}>(),
	},
	actions: {
		ensureRun: (c, runId: string) => {
			if (!c.state.initialized) {
				c.state.initialized = true;
			}
			return runId;
		},

		createEvent: (
			c,
			runId: string,
			data: RunCreatedEventRequest | CreateEventRequest,
			opts?: { requestId?: string },
		): EventResult => {
			if (opts?.requestId) {
				const existingId = c.state.idempotencyKeys[opts.requestId];
				if (existingId) {
					const existing = c.state.events.find(
						(e) => e.eventId === existingId,
					);
					if (existing) {
						return {
							event: eventToPublic(existing),
							run: c.state.run
								? runToPublic(c.state.run)
								: undefined,
						};
					}
				}
			}

			const ev: PersistedEvent = {
				eventId: uuidv4(),
				eventType: data.eventType,
				runId,
				correlationId:
					"correlationId" in data ? data.correlationId : undefined,
				eventData: data.eventData,
				createdAt: nowMs(),
			};

			materializeEvent({ state: c.state, event: ev, data });
			c.state.events.push(ev);

			if (opts?.requestId) {
				c.state.idempotencyKeys[opts.requestId] = ev.eventId;
			}

			const corrId =
				"correlationId" in data ? data.correlationId : undefined;

			return {
				event: eventToPublic(ev),
				run: c.state.run ? runToPublic(c.state.run) : undefined,
				step: corrId && c.state.steps[corrId]
					? stepToPublic(c.state.steps[corrId])
					: undefined,
				hook: corrId && c.state.hooks[corrId]
					? hookToPublic(c.state.hooks[corrId])
					: undefined,
				wait: corrId && c.state.waits[corrId]
					? waitToPublic(c.state.waits[corrId])
					: undefined,
			};
		},

		getEvent: (c, eventId: string): WorldEvent | null => {
			const ev = c.state.events.find((e) => e.eventId === eventId);
			return ev ? eventToPublic(ev) : null;
		},

		getRun: (c): WorkflowRun | null => {
			return c.state.run ? runToPublic(c.state.run) : null;
		},

		getStep: (c, stepId: string): Step | null => {
			const step = c.state.steps[stepId];
			return step ? stepToPublic(step) : null;
		},

		listSteps: (
			c,
			opts?: {
				cursor?: string;
				limit?: number;
				sortOrder?: "asc" | "desc";
			},
		) => {
			const limit = opts?.limit ?? 50;
			let items = Object.values(c.state.steps);
			const desc = opts?.sortOrder !== "asc";
			items.sort((a, b) =>
				desc ? b.createdAt - a.createdAt : a.createdAt - b.createdAt,
			);

			const startIdx = opts?.cursor
				? Number.parseInt(opts.cursor, 10)
				: 0;
			const slice = items.slice(startIdx, startIdx + limit);
			const nextIdx = startIdx + slice.length;
			return {
				data: slice.map(stepToPublic),
				cursor: nextIdx < items.length ? String(nextIdx) : null,
				hasMore: nextIdx < items.length,
			};
		},

		listEvents: (
			c,
			opts?: {
				cursor?: string;
				limit?: number;
				sortOrder?: "asc" | "desc";
			},
		) => {
			const limit = opts?.limit ?? 100;
			let items = [...c.state.events];
			if (opts?.sortOrder === "desc") {
				items.reverse();
			}

			const startIdx = opts?.cursor
				? Number.parseInt(opts.cursor, 10)
				: 0;
			const slice = items.slice(startIdx, startIdx + limit);
			const nextIdx = startIdx + slice.length;
			return {
				data: slice.map(eventToPublic),
				cursor: nextIdx < items.length ? String(nextIdx) : null,
				hasMore: nextIdx < items.length,
			};
		},

		listEventsByCorrelationId: (
			c,
			correlationId: string,
			opts?: {
				cursor?: string;
				limit?: number;
				sortOrder?: "asc" | "desc";
			},
		) => {
			const limit = opts?.limit ?? 100;
			let items = c.state.events.filter(
				(e) => e.correlationId === correlationId,
			);
			if (opts?.sortOrder === "desc") {
				items = [...items].reverse();
			}
			const startIdx = opts?.cursor
				? Number.parseInt(opts.cursor, 10)
				: 0;
			const slice = items.slice(startIdx, startIdx + limit);
			const nextIdx = startIdx + slice.length;
			return {
				data: slice.map(eventToPublic),
				cursor: nextIdx < items.length ? String(nextIdx) : null,
				hasMore: nextIdx < items.length,
			};
		},

		getHook: (c, hookId: string): Hook | null => {
			const hook = c.state.hooks[hookId];
			return hook ? hookToPublic(hook) : null;
		},

		listHooks: (
			c,
			opts?: {
				cursor?: string;
				limit?: number;
				sortOrder?: "asc" | "desc";
			},
		) => {
			const limit = opts?.limit ?? 50;
			let items = Object.values(c.state.hooks);
			const desc = opts?.sortOrder !== "asc";
			items.sort((a, b) =>
				desc ? b.createdAt - a.createdAt : a.createdAt - b.createdAt,
			);
			const startIdx = opts?.cursor
				? Number.parseInt(opts.cursor, 10)
				: 0;
			const slice = items.slice(startIdx, startIdx + limit);
			const nextIdx = startIdx + slice.length;
			return {
				data: slice.map(hookToPublic),
				cursor: nextIdx < items.length ? String(nextIdx) : null,
				hasMore: nextIdx < items.length,
			};
		},

		// ------------------------------------------------------------------
		// Stream operations
		// ------------------------------------------------------------------

		writeStream: (
			c,
			streamName: string,
			chunks: (string | Uint8Array)[],
		) => {
			let stream = c.state.streams[streamName];
			if (!stream) {
				stream = {
					name: streamName,
					chunks: [],
					tailIndex: -1,
					done: false,
				};
				c.state.streams[streamName] = stream;
			}
			if (stream.done) {
				throw new Error(`stream ${streamName} is closed`);
			}
			const appended: PersistedStreamChunk[] = [];
			for (const chunk of chunks) {
				stream.tailIndex += 1;
				const encoded: PersistedStreamChunk = {
					index: stream.tailIndex,
					data: encodeBinary(chunk),
				};
				stream.chunks.push(encoded);
				appended.push(encoded);
			}
			c.broadcast("streamAppended", {
				streamName,
				chunks: appended,
				done: false,
			});
		},

		closeStream: (c, streamName: string) => {
			const stream = c.state.streams[streamName];
			if (!stream) {
				c.state.streams[streamName] = {
					name: streamName,
					chunks: [],
					tailIndex: -1,
					done: true,
				};
				c.broadcast("streamAppended", {
					streamName,
					chunks: [],
					done: true,
				});
				return;
			}
			stream.done = true;
			c.broadcast("streamAppended", {
				streamName,
				chunks: [],
				done: true,
			});
		},

		getStreamInfo: (c, streamName: string) => {
			const stream = c.state.streams[streamName];
			if (!stream) {
				return { tailIndex: -1, done: false };
			}
			return { tailIndex: stream.tailIndex, done: stream.done };
		},

		getStreamChunks: (
			c,
			streamName: string,
			opts?: { limit?: number; cursor?: string },
		) => {
			const stream = c.state.streams[streamName];
			if (!stream) {
				return {
					data: [] as PersistedStreamChunk[],
					cursor: null as string | null,
					hasMore: false,
					done: false,
				};
			}
			const limit = opts?.limit ?? 100;
			const startIdx = opts?.cursor
				? Number.parseInt(opts.cursor, 10)
				: 0;
			const slice = stream.chunks.slice(startIdx, startIdx + limit);
			const nextIdx = startIdx + slice.length;
			const hasMore = nextIdx < stream.chunks.length;
			return {
				data: slice,
				cursor: hasMore ? String(nextIdx) : null,
				hasMore,
				done: stream.done && !hasMore,
			};
		},

		listStreams: (c): string[] => {
			return Object.keys(c.state.streams);
		},
	},
});

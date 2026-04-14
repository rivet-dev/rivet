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
 * of truth. This is the Postgres-World reference pattern, mapped onto Rivet
 * actor state.
 */

import { actor, event } from "rivetkit";
import { v4 as uuidv4 } from "uuid";
import type {
	CreateEventRequest,
	Event as WorldEvent,
	EventResult,
	EventType,
	Hook,
	HookStatus,
	RunCreatedEventRequest,
	Step,
	StepStatus,
	WorkflowRun,
	WorkflowRunStatus,
} from "../types";
import { encodeBinary, nowMs } from "./shared";

// ---------------------------------------------------------------------------
// Persisted state shape
// ---------------------------------------------------------------------------

interface PersistedRun {
	id: string;
	workflowName: string;
	status: WorkflowRunStatus;
	input?: unknown;
	output?: unknown;
	error?: unknown;
	createdAt: number;
	updatedAt: number;
	startedAt?: number;
	finishedAt?: number;
	deploymentId?: string;
	parentRunId?: string;
	parentStepId?: string;
	traceCarrier?: Record<string, string>;
	metadata?: Record<string, unknown>;
}

interface PersistedStep {
	id: string;
	runId: string;
	name: string;
	status: StepStatus;
	input?: unknown;
	output?: unknown;
	error?: unknown;
	createdAt: number;
	updatedAt: number;
	startedAt?: number;
	finishedAt?: number;
	attempt: number;
	parentStepId?: string;
}

interface PersistedHook {
	id: string;
	runId: string;
	token: string;
	name: string;
	status: HookStatus;
	createdAt: number;
	updatedAt: number;
	disposedAt?: number;
	triggeredAt?: number;
	metadata?: Record<string, unknown>;
}

interface PersistedEvent {
	id: string;
	type: EventType;
	runId: string;
	stepId?: string;
	hookId?: string;
	correlationId?: string;
	data?: unknown;
	createdAt: number;
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
		id: run.id,
		workflowName: run.workflowName,
		status: run.status,
		input: run.input,
		output: run.output,
		error: run.error as WorkflowRun["error"],
		createdAt: new Date(run.createdAt),
		updatedAt: new Date(run.updatedAt),
		startedAt: run.startedAt ? new Date(run.startedAt) : undefined,
		finishedAt: run.finishedAt ? new Date(run.finishedAt) : undefined,
		deploymentId: run.deploymentId,
		parentRunId: run.parentRunId,
		parentStepId: run.parentStepId,
		traceCarrier: run.traceCarrier,
		metadata: run.metadata,
	};
}

function stepToPublic(step: PersistedStep): Step {
	return {
		id: step.id,
		runId: step.runId,
		name: step.name,
		status: step.status,
		input: step.input,
		output: step.output,
		error: step.error as Step["error"],
		createdAt: new Date(step.createdAt),
		updatedAt: new Date(step.updatedAt),
		startedAt: step.startedAt ? new Date(step.startedAt) : undefined,
		finishedAt: step.finishedAt ? new Date(step.finishedAt) : undefined,
		attempt: step.attempt,
		parentStepId: step.parentStepId,
	};
}

function hookToPublic(hook: PersistedHook): Hook {
	return {
		id: hook.id,
		runId: hook.runId,
		token: hook.token,
		name: hook.name,
		status: hook.status,
		createdAt: new Date(hook.createdAt),
		updatedAt: new Date(hook.updatedAt),
		disposedAt: hook.disposedAt ? new Date(hook.disposedAt) : undefined,
		triggeredAt: hook.triggeredAt ? new Date(hook.triggeredAt) : undefined,
		metadata: hook.metadata,
	};
}

function eventToPublic(e: PersistedEvent): WorldEvent {
	return {
		id: e.id,
		type: e.type,
		runId: e.runId,
		stepId: e.stepId,
		hookId: e.hookId,
		correlationId: e.correlationId,
		data: e.data,
		createdAt: new Date(e.createdAt),
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

	if (data.type === "run_created") {
		if (state.run) {
			// Idempotency: treat as no-op so replays do not clobber state.
			return;
		}
		state.run = {
			id: ev.runId,
			workflowName: data.workflowName,
			status: "pending",
			input: data.input,
			createdAt: now,
			updatedAt: now,
			deploymentId: data.deploymentId,
			parentRunId: data.parentRunId,
			parentStepId: data.parentStepId,
			traceCarrier: data.traceCarrier,
			metadata: data.metadata,
		};
		return;
	}

	if (!state.run) {
		// Ignore events that arrive before run_created. This keeps the actor
		// defensive; the coordinator should prevent this in practice.
		return;
	}

	switch (data.type) {
		case "run_started":
			state.run.status = "running";
			state.run.startedAt ??= now;
			state.run.updatedAt = now;
			break;
		case "run_completed":
			state.run.status = "completed";
			state.run.finishedAt = now;
			state.run.updatedAt = now;
			if (data.data !== undefined) {
				state.run.output = data.data;
			}
			break;
		case "run_failed":
			state.run.status = "failed";
			state.run.finishedAt = now;
			state.run.updatedAt = now;
			state.run.error = data.data;
			break;
		case "run_cancelled":
			state.run.status = "cancelled";
			state.run.finishedAt = now;
			state.run.updatedAt = now;
			break;
		case "run_updated":
			state.run.updatedAt = now;
			if (typeof data.data === "object" && data.data !== null) {
				Object.assign(state.run, data.data);
			}
			break;
		case "step_created": {
			const stepId = data.stepId;
			if (!stepId) break;
			const stepData = (data.data ?? {}) as Partial<PersistedStep>;
			state.steps[stepId] = {
				id: stepId,
				runId: ev.runId,
				name: stepData.name ?? "step",
				status: "pending",
				input: stepData.input,
				createdAt: now,
				updatedAt: now,
				attempt: 0,
				parentStepId: stepData.parentStepId,
			};
			break;
		}
		case "step_started": {
			const step = data.stepId ? state.steps[data.stepId] : undefined;
			if (!step) break;
			step.status = "running";
			step.startedAt ??= now;
			step.updatedAt = now;
			step.attempt += 1;
			break;
		}
		case "step_completed": {
			const step = data.stepId ? state.steps[data.stepId] : undefined;
			if (!step) break;
			step.status = "completed";
			step.finishedAt = now;
			step.updatedAt = now;
			step.output = data.data;
			break;
		}
		case "step_failed": {
			const step = data.stepId ? state.steps[data.stepId] : undefined;
			if (!step) break;
			step.status = "failed";
			step.finishedAt = now;
			step.updatedAt = now;
			step.error = data.data;
			break;
		}
		case "step_cancelled": {
			const step = data.stepId ? state.steps[data.stepId] : undefined;
			if (!step) break;
			step.status = "cancelled";
			step.finishedAt = now;
			step.updatedAt = now;
			break;
		}
		case "step_retrying": {
			const step = data.stepId ? state.steps[data.stepId] : undefined;
			if (!step) break;
			step.status = "pending";
			step.updatedAt = now;
			break;
		}
		case "hook_created": {
			const hookId = data.hookId;
			if (!hookId) break;
			const hookData = (data.data ?? {}) as Partial<PersistedHook>;
			state.hooks[hookId] = {
				id: hookId,
				runId: ev.runId,
				token: hookData.token ?? hookId,
				name: hookData.name ?? "hook",
				status: "pending",
				createdAt: now,
				updatedAt: now,
				metadata: hookData.metadata,
			};
			break;
		}
		case "hook_triggered": {
			const hook = data.hookId ? state.hooks[data.hookId] : undefined;
			if (!hook) break;
			hook.status = "triggered";
			hook.triggeredAt = now;
			hook.updatedAt = now;
			break;
		}
		case "hook_disposed": {
			const hook = data.hookId ? state.hooks[data.hookId] : undefined;
			if (!hook) break;
			hook.status = "disposed";
			hook.disposedAt = now;
			hook.updatedAt = now;
			break;
		}
		case "hook_conflict":
		case "stream_chunk":
		case "stream_closed":
			// No materialization needed. Streams are mutated directly; hook
			// conflicts are surfaced through the event itself.
			break;
	}

	// Auto-dispose hooks when the run enters a terminal status.
	if (state.run && isTerminalRunStatus(state.run.status)) {
		for (const hook of Object.values(state.hooks)) {
			if (hook.status === "pending") {
				hook.status = "disposed";
				hook.disposedAt = now;
				hook.updatedAt = now;
			}
		}
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
		/** Initialize the run identity. Called implicitly by `createEvent`. */
		ensureRun: (c, runId: string) => {
			if (!c.state.initialized) {
				c.state.initialized = true;
			}
			return runId;
		},

		/** Append an event. Atomically updates materialized state. */
		createEvent: (
			c,
			runId: string,
			data: RunCreatedEventRequest | CreateEventRequest,
			opts?: { idempotencyKey?: string },
		): EventResult => {
			if (opts?.idempotencyKey) {
				const existingId = c.state.idempotencyKeys[opts.idempotencyKey];
				if (existingId) {
					const existing = c.state.events.find(
						(e) => e.id === existingId,
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
				id: uuidv4(),
				type: data.type,
				runId,
				stepId: "stepId" in data ? data.stepId : undefined,
				hookId: "hookId" in data ? data.hookId : undefined,
				correlationId:
					"correlationId" in data ? data.correlationId : undefined,
				data: "data" in data ? data.data : undefined,
				createdAt: nowMs(),
			};

			materializeEvent({ state: c.state, event: ev, data });
			c.state.events.push(ev);

			if (opts?.idempotencyKey) {
				c.state.idempotencyKeys[opts.idempotencyKey] = ev.id;
			}

			return {
				event: eventToPublic(ev),
				run: c.state.run ? runToPublic(c.state.run) : undefined,
			};
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
			opts?: { status?: StepStatus; cursor?: string; limit?: number },
		) => {
			const limit = opts?.limit ?? 50;
			let items = Object.values(c.state.steps);
			if (opts?.status) {
				items = items.filter((s) => s.status === opts.status);
			}
			items.sort((a, b) => a.createdAt - b.createdAt);

			const startIdx = opts?.cursor ? Number.parseInt(opts.cursor, 10) : 0;
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
				types?: EventType[];
				stepId?: string;
				hookId?: string;
				cursor?: string;
				limit?: number;
			},
		) => {
			const limit = opts?.limit ?? 100;
			let items = c.state.events;
			if (opts?.types && opts.types.length > 0) {
				const set = new Set(opts.types);
				items = items.filter((e) => set.has(e.type));
			}
			if (opts?.stepId) {
				items = items.filter((e) => e.stepId === opts.stepId);
			}
			if (opts?.hookId) {
				items = items.filter((e) => e.hookId === opts.hookId);
			}

			const startIdx = opts?.cursor ? Number.parseInt(opts.cursor, 10) : 0;
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
			opts?: { cursor?: string; limit?: number },
		) => {
			const limit = opts?.limit ?? 100;
			const items = c.state.events.filter(
				(e) => e.correlationId === correlationId,
			);
			const startIdx = opts?.cursor ? Number.parseInt(opts.cursor, 10) : 0;
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
			opts?: { status?: HookStatus; cursor?: string; limit?: number },
		) => {
			const limit = opts?.limit ?? 50;
			let items = Object.values(c.state.hooks);
			if (opts?.status) {
				items = items.filter((h) => h.status === opts.status);
			}
			items.sort((a, b) => a.createdAt - b.createdAt);
			const startIdx = opts?.cursor ? Number.parseInt(opts.cursor, 10) : 0;
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

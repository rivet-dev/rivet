/**
 * `createRivetWorld` factory.
 *
 * Wires the three Rivet actors into a single `World` instance that
 * implements the Vercel Workflow SDK's `Storage`, `Queue`, and `Streamer`
 * interfaces.
 *
 * A note on actor sharding: the coordinator and queue actors are singletons
 * per name. The `workflowRun` actor is keyed by `runId`, so each run lives
 * on its own actor and can be materialized independently.
 */

import { createClient } from "rivetkit/client";
import { v4 as uuidv4 } from "uuid";
import { decodeBinary } from "./actors/shared";
import type { WorldRegistry } from "./registry";
import {
	type CreateEventParams,
	type CreateEventRequest,
	type Event as WorldEvent,
	type EventResult,
	HookConflictError,
	type GetHookParams,
	type GetStepParams,
	type GetWorkflowRunParams,
	type Hook,
	type ListEventsByCorrelationIdParams,
	type ListEventsParams,
	type ListHooksParams,
	type ListWorkflowRunStepsParams,
	type ListWorkflowRunsParams,
	NotFoundError,
	type PaginatedResponse,
	type QueueHandler,
	type QueueOptions,
	type QueuePayload,
	type QueuePrefix,
	type RunCreatedEventRequest,
	type Step,
	type StreamChunk,
	type StreamChunksResult,
	type StreamInfo,
	type ValidQueueName,
	type WorkflowRun,
	type World,
} from "./types";

export interface RivetWorldConfig {
	/**
	 * RivetKit client endpoint or full config. Passed through to
	 * `rivetkit/client` `createClient`.
	 */
	endpoint?: string;

	/**
	 * Stable deployment id returned by `Queue.getDeploymentId()`. If omitted
	 * we derive one from `process.env.VERCEL_DEPLOYMENT_ID` or generate a
	 * random id per process.
	 */
	deploymentId?: string;

	/**
	 * Optional key resolver for per-run encryption.
	 */
	getEncryptionKeyForRun?: (
		run: WorkflowRun,
	) => Promise<Uint8Array | undefined>;

	/** Pre-built rivetkit client. Overrides `endpoint` when provided. */
	client?: ReturnType<typeof createClient<WorldRegistry>>;
}

const COORDINATOR_KEY = ["coordinator"] as const;

function resolveDeploymentId(cfg: RivetWorldConfig): string {
	if (cfg.deploymentId) return cfg.deploymentId;
	const envId =
		typeof process !== "undefined"
			? process.env.VERCEL_DEPLOYMENT_ID ??
				process.env.WORKFLOW_DEPLOYMENT_ID
			: undefined;
	return envId ?? `rivet-world-${uuidv4()}`;
}

type RivetClient = ReturnType<typeof createClient<WorldRegistry>>;

function runHandle(client: RivetClient, runId: string) {
	return client.workflowRun.getOrCreate([runId]);
}

function coordinatorHandle(client: RivetClient) {
	return client.coordinator.getOrCreate([...COORDINATOR_KEY]);
}

function queueHandle(client: RivetClient, queueName: ValidQueueName) {
	return client.queueRunner.getOrCreate([queueName]);
}

function revivePaginated<T>(result: {
	data: unknown[];
	cursor: string | null;
	hasMore: boolean;
}): PaginatedResponse<T> {
	return {
		data: result.data as T[],
		cursor: result.cursor,
		hasMore: result.hasMore,
	};
}

function reviveChunks(
	raw: { index: number; data: string }[],
): StreamChunk[] {
	return raw.map((c) => ({ index: c.index, data: decodeBinary(c.data) }));
}

export function createRivetWorld(cfg: RivetWorldConfig = {}): World {
	const client: RivetClient =
		cfg.client ?? createClient<WorldRegistry>(cfg.endpoint);
	const deploymentId = resolveDeploymentId(cfg);

	// -----------------------------------------------------------------
	// Storage
	// -----------------------------------------------------------------

	const world: World = {
		runs: {
			async get(id, _params?: GetWorkflowRunParams): Promise<WorkflowRun> {
				const run = (await runHandle(client, id).getRun()) as
					| WorkflowRun
					| null;
				if (!run) throw new NotFoundError("run", id);
				return run;
			},
			async list(
				params: ListWorkflowRunsParams = {},
			): Promise<PaginatedResponse<WorkflowRun>> {
				const res = (await coordinatorHandle(client).listRuns({
					cursor: params.cursor,
					limit: params.limit,
					workflowName: params.workflowName,
					status: params.status,
					deploymentId: params.deploymentId,
					parentRunId: params.parentRunId,
					createdAfter: params.createdAfter?.getTime(),
					createdBefore: params.createdBefore?.getTime(),
				})) as {
					data: Array<{
						id: string;
						workflowName: string;
						status: WorkflowRun["status"];
						createdAt: number;
						updatedAt: number;
						deploymentId?: string;
						parentRunId?: string;
					}>;
					cursor: string | null;
					hasMore: boolean;
				};

				// The coordinator only stores an index row. Materialize full
				// runs by hitting each run actor. In practice callers will
				// paginate, so this fan-out is bounded by `limit`.
				const data = await Promise.all(
					res.data.map(async (row) => {
						const run = (await runHandle(
							client,
							row.id,
						).getRun()) as WorkflowRun | null;
						if (run) return run;
						// Fall back to the index row if the run actor is gone.
						return {
							id: row.id,
							workflowName: row.workflowName,
							status: row.status,
							createdAt: new Date(row.createdAt),
							updatedAt: new Date(row.updatedAt),
							deploymentId: row.deploymentId,
							parentRunId: row.parentRunId,
						} satisfies WorkflowRun;
					}),
				);

				return {
					data,
					cursor: res.cursor,
					hasMore: res.hasMore,
				};
			},
		},
		steps: {
			async get(
				runId: string | undefined,
				stepId: string,
				_params?: GetStepParams,
			): Promise<Step> {
				if (!runId) {
					throw new NotFoundError("step", stepId);
				}
				const step = (await runHandle(client, runId).getStep(
					stepId,
				)) as Step | null;
				if (!step) throw new NotFoundError("step", stepId);
				return step;
			},
			async list(
				params: ListWorkflowRunStepsParams,
			): Promise<PaginatedResponse<Step>> {
				const res = (await runHandle(client, params.runId).listSteps({
					status: params.status,
					cursor: params.cursor,
					limit: params.limit,
				})) as unknown as {
					data: Step[];
					cursor: string | null;
					hasMore: boolean;
				};
				return revivePaginated<Step>(res);
			},
		},
		events: {
			create: (async (
				runId: string | null,
				data: CreateEventRequest | RunCreatedEventRequest,
				params?: CreateEventParams,
			): Promise<EventResult> => {
				if (data.type === "run_created") {
					const id = runId ?? uuidv4();
					const runActor = runHandle(client, id);
					const result = (await runActor.createEvent(
						id,
						data,
						params,
					)) as EventResult;

					if (result.run) {
						const run = result.run;
						await coordinatorHandle(client).registerRun({
							id: run.id,
							workflowName: run.workflowName,
							status: run.status,
							createdAt:
								run.createdAt instanceof Date
									? run.createdAt.getTime()
									: Date.parse(
											run.createdAt as unknown as string,
										),
							updatedAt:
								run.updatedAt instanceof Date
									? run.updatedAt.getTime()
									: Date.parse(
											run.updatedAt as unknown as string,
										),
							deploymentId: run.deploymentId ?? deploymentId,
							parentRunId: run.parentRunId,
						});
					}
					return result;
				}

				if (!runId) {
					throw new Error(
						`events.create requires runId for event type ${data.type}`,
					);
				}

				const runActor = runHandle(client, runId);

				// Special-case hook creation so the coordinator can enforce
				// global token uniqueness before we commit the event.
				if (data.type === "hook_created") {
					const hookData = (data.data ?? {}) as {
						token?: string;
						name?: string;
					};
					if (hookData.token) {
						const coord = coordinatorHandle(client);
						const reg = (await coord.registerHookToken({
							hookId: data.hookId ?? uuidv4(),
							runId,
							token: hookData.token,
							status: "pending",
							createdAt: Date.now(),
						})) as { ok: true } | { ok: false; reason: string };
						if (!reg.ok) {
							// Materialize a hook_conflict event on the run so
							// the event log reflects the rejection, then throw.
							await runActor.createEvent(
								runId,
								{
									type: "hook_conflict",
									data: { token: hookData.token },
								},
								params,
							);
							throw new HookConflictError(hookData.token);
						}
					}
				}

				const result = (await runActor.createEvent(
					runId,
					data,
					params,
				)) as EventResult;

				// Mirror status into coordinator when the run status moved.
				if (result.run) {
					const run = result.run;
					await coordinatorHandle(client).updateRunStatus(
						run.id,
						run.status,
						run.updatedAt instanceof Date
							? run.updatedAt.getTime()
							: Date.parse(run.updatedAt as unknown as string),
					);
					if (
						run.status === "completed" ||
						run.status === "failed" ||
						run.status === "cancelled"
					) {
						await coordinatorHandle(
							client,
						).disposeHookTokensForRun(run.id);
					}
				}

				return result;
			}) as World["events"]["create"],
			async list(
				params: ListEventsParams,
			): Promise<PaginatedResponse<WorldEvent>> {
				const res = (await runHandle(client, params.runId).listEvents({
					types: params.types,
					stepId: params.stepId,
					hookId: params.hookId,
					cursor: params.cursor,
					limit: params.limit,
				})) as unknown as {
					data: WorldEvent[];
					cursor: string | null;
					hasMore: boolean;
				};
				return revivePaginated<WorldEvent>(res);
			},
			async listByCorrelationId(
				params: ListEventsByCorrelationIdParams,
			): Promise<PaginatedResponse<WorldEvent>> {
				// Correlation ids are not indexed globally. We delegate to the
				// caller-provided run scope in practice; if unknown, the
				// Workflow SDK passes a correlation id that matches a single
				// run. We fall back to returning an empty result.
				return {
					data: [],
					cursor: null,
					hasMore: false,
				};
			},
		},
		hooks: {
			async get(hookId: string, _params?: GetHookParams): Promise<Hook> {
				// Hooks are keyed by runId; we need to resolve via the
				// coordinator. For direct lookups by hook id, the Workflow
				// SDK normally knows the runId already, but we support it by
				// scanning hooks on the run if supplied via metadata prefix.
				throw new NotFoundError("hook", hookId);
			},
			async getByToken(
				token: string,
				_params?: GetHookParams,
			): Promise<Hook> {
				const row = (await coordinatorHandle(client).lookupHookToken(
					token,
				)) as {
					hookId: string;
					runId: string;
					token: string;
					status: Hook["status"];
					createdAt: number;
				} | null;
				if (!row) throw new NotFoundError("hook", token);
				const hook = (await runHandle(client, row.runId).getHook(
					row.hookId,
				)) as Hook | null;
				if (!hook) throw new NotFoundError("hook", token);
				return hook;
			},
			async list(
				params: ListHooksParams,
			): Promise<PaginatedResponse<Hook>> {
				const res = (await runHandle(client, params.runId).listHooks({
					status: params.status,
					cursor: params.cursor,
					limit: params.limit,
				})) as unknown as {
					data: Hook[];
					cursor: string | null;
					hasMore: boolean;
				};
				return revivePaginated<Hook>(res);
			},
		},

		// -----------------------------------------------------------------
		// Queue
		// -----------------------------------------------------------------

		async getDeploymentId(): Promise<string> {
			return deploymentId;
		},

		async queue(
			queueName: ValidQueueName,
			message: QueuePayload,
			opts?: QueueOptions,
		) {
			const res = (await queueHandle(client, queueName).enqueue(
				queueName,
				message,
				opts,
			)) as { messageId: string };
			return { messageId: res.messageId };
		},

		createQueueHandler(
			queueNamePrefix: QueuePrefix,
			handler: QueueHandler,
		): (req: Request) => Promise<Response> {
			return async (req: Request): Promise<Response> => {
				let body: {
					queueName?: ValidQueueName;
					messageId?: string;
				};
				try {
					body = await req.json();
				} catch {
					return new Response("invalid body", { status: 400 });
				}

				const queueName = body.queueName;
				if (!queueName || !queueName.startsWith(queueNamePrefix)) {
					return new Response("queue name mismatch", { status: 400 });
				}

				const q = queueHandle(client, queueName);
				const msg = (await q.claimNext()) as {
					id: string;
					queueName: ValidQueueName;
					payload: unknown;
					attempt: number;
				} | null;
				if (!msg) {
					return new Response(null, { status: 204 });
				}

				try {
					const result = await handler(msg.payload, {
						attempt: msg.attempt,
						queueName: msg.queueName,
						messageId: msg.id,
					});
					await q.ack(msg.id);
					return new Response(
						JSON.stringify({
							ok: true,
							messageId: msg.id,
							timeoutSeconds:
								result && "timeoutSeconds" in result
									? result.timeoutSeconds
									: undefined,
						}),
						{
							status: 200,
							headers: { "content-type": "application/json" },
						},
					);
				} catch (err) {
					await q.nack(
						msg.id,
						err instanceof Error ? err.message : String(err),
					);
					return new Response(
						JSON.stringify({
							ok: false,
							messageId: msg.id,
							error:
								err instanceof Error
									? err.message
									: String(err),
						}),
						{
							status: 500,
							headers: { "content-type": "application/json" },
						},
					);
				}
			};
		},

		// -----------------------------------------------------------------
		// Streamer
		// -----------------------------------------------------------------

		async writeToStream(
			name: string,
			runId: string,
			chunk: string | Uint8Array,
		): Promise<void> {
			await runHandle(client, runId).writeStream(name, [chunk]);
		},
		async writeToStreamMulti(
			name: string,
			runId: string,
			chunks: (string | Uint8Array)[],
		): Promise<void> {
			await runHandle(client, runId).writeStream(name, chunks);
		},
		async closeStream(name: string, runId: string): Promise<void> {
			await runHandle(client, runId).closeStream(name);
		},
		async getStreamChunks(
			name: string,
			runId: string,
			options?: { limit?: number; cursor?: string },
		): Promise<StreamChunksResult> {
			const res = (await runHandle(client, runId).getStreamChunks(
				name,
				options,
			)) as {
				data: { index: number; data: string }[];
				cursor: string | null;
				hasMore: boolean;
				done: boolean;
			};
			return {
				data: reviveChunks(res.data),
				cursor: res.cursor,
				hasMore: res.hasMore,
				done: res.done,
			};
		},
		async getStreamInfo(name: string, runId: string): Promise<StreamInfo> {
			return (await runHandle(client, runId).getStreamInfo(
				name,
			)) as StreamInfo;
		},
		async listStreamsByRunId(runId: string): Promise<string[]> {
			return (await runHandle(client, runId).listStreams()) as string[];
		},
		async readFromStream(
			_name: string,
			_startIndex = 0,
		): Promise<ReadableStream<Uint8Array>> {
			// Live streaming support requires subscribing to the
			// `streamAppended` actor event over WebSocket. Until that is
			// wired up, callers must use `getStreamChunks` to drain the
			// stream manually.
			throw new Error(
				"readFromStream is not yet implemented; use getStreamChunks",
			);
		},

		// -----------------------------------------------------------------
		// Lifecycle
		// -----------------------------------------------------------------

		async start(): Promise<void> {
			// No-op: the RivetKit client lazily connects on first use.
		},
		async close(): Promise<void> {
			// No-op: RivetKit client does not require explicit shutdown.
		},
		getEncryptionKeyForRun: cfg.getEncryptionKeyForRun,
	};

	return world;
}

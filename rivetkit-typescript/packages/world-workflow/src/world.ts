/**
 * `createRivetWorld` factory.
 *
 * Wires the three Rivet actors into a single `World` instance that
 * implements the Vercel Workflow SDK's `Storage`, `Queue`, and `Streamer`
 * interfaces.
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
	type GetChunksOptions,
	type GetEventParams,
	type GetHookParams,
	type GetStepParams,
	type GetWorkflowRunParams,
	type Hook,
	HookConflictError,
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
	SPEC_VERSION_CURRENT,
	type Step,
	type StepWithoutData,
	type StreamChunksResponse,
	type StreamInfoResponse,
	type ValidQueueName,
	type WorkflowRun,
	type WorkflowRunWithoutData,
	type World,
} from "./types";

export interface RivetWorldConfig {
	endpoint?: string;
	deploymentId?: string;
	getEncryptionKeyForRun?: (
		run: WorkflowRun,
	) => Promise<Uint8Array | undefined>;
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
): { index: number; data: Uint8Array }[] {
	return raw.map((c) => ({ index: c.index, data: decodeBinary(c.data) }));
}

export function createRivetWorld(cfg: RivetWorldConfig = {}): World {
	const client: RivetClient =
		cfg.client ?? createClient<WorldRegistry>(cfg.endpoint);
	const deploymentId = resolveDeploymentId(cfg);

	const queueHandlers = new Map<QueuePrefix, QueueHandler>();

	async function dispatchMessage(
		queueName: ValidQueueName,
	): Promise<void> {
		const prefix = (
			queueName.startsWith("__wkf_workflow_")
				? "__wkf_workflow_"
				: "__wkf_step_"
		) as QueuePrefix;
		const handler = queueHandlers.get(prefix);
		if (!handler) return;

		const q = queueHandle(client, queueName);
		const msg = (await q.claimNext()) as {
			id: string;
			queueName: ValidQueueName;
			payload: unknown;
			attempt: number;
		} | null;
		if (!msg) return;

		try {
			await handler(msg.payload, {
				attempt: msg.attempt,
				queueName: msg.queueName,
				messageId: msg.id,
			});
			await q.ack(msg.id);
		} catch (err) {
			await q.nack(
				msg.id,
				err instanceof Error ? err.message : String(err),
			);
		}
	}

	// -----------------------------------------------------------------
	// Build world
	// -----------------------------------------------------------------

	const world: World = {
		specVersion: SPEC_VERSION_CURRENT,

		// =============================================================
		// Storage
		// =============================================================

		runs: {
			async get(
				id: string,
				_params?: GetWorkflowRunParams,
			): Promise<WorkflowRun | WorkflowRunWithoutData> {
				const run = (await runHandle(client, id).getRun()) as
					| WorkflowRun
					| null;
				if (!run) throw new NotFoundError("run", id);
				return run;
			},
			async list(
				params: ListWorkflowRunsParams = {},
			): Promise<
				PaginatedResponse<WorkflowRun | WorkflowRunWithoutData>
			> {
				const pagination = params.pagination ?? {};
				const res = (await coordinatorHandle(client).listRuns({
					cursor: pagination.cursor,
					limit: pagination.limit,
					workflowName: params.workflowName,
					status: params.status,
				})) as {
					data: Array<{
						id: string;
						workflowName: string;
						status: WorkflowRun["status"];
						createdAt: number;
						updatedAt: number;
						deploymentId?: string;
					}>;
					cursor: string | null;
					hasMore: boolean;
				};

				const data = await Promise.all(
					res.data.map(async (row) => {
						const run = (await runHandle(
							client,
							row.id,
						).getRun()) as WorkflowRun | null;
						if (run) return run;
						return {
							runId: row.id,
							workflowName: row.workflowName,
							status: row.status,
							deploymentId: row.deploymentId ?? deploymentId,
							input: undefined,
							output: undefined,
							createdAt: new Date(row.createdAt),
							updatedAt: new Date(row.updatedAt),
						} satisfies WorkflowRunWithoutData;
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
				runId: string,
				stepId: string,
				_params?: GetStepParams,
			): Promise<Step | StepWithoutData> {
				const step = (await runHandle(client, runId).getStep(
					stepId,
				)) as Step | null;
				if (!step) throw new NotFoundError("step", stepId);
				return step;
			},
			async list(
				params: ListWorkflowRunStepsParams,
			): Promise<PaginatedResponse<Step | StepWithoutData>> {
				const pagination = params.pagination ?? {};
				const res = (await runHandle(
					client,
					params.runId,
				).listSteps({
					cursor: pagination.cursor,
					limit: pagination.limit,
					sortOrder: pagination.sortOrder,
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
				if (data.eventType === "run_created") {
					const id = runId ?? uuidv4();
					const runActor = runHandle(client, id);
					const result = (await runActor.createEvent(
						id,
						data,
						params ? { requestId: params.requestId } : undefined,
					)) as EventResult;

					if (result.run) {
						const run = result.run;
						await coordinatorHandle(client).registerRun({
							id: run.runId,
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
						});
					}
					return result;
				}

				if (!runId) {
					throw new Error(
						`events.create requires runId for event type ${data.eventType}`,
					);
				}

				const runActor = runHandle(client, runId);

				if (data.eventType === "hook_created") {
					const hookData = (data.eventData ?? {}) as {
						token?: string;
					};
					if (hookData.token) {
						const hookId =
							data.correlationId ?? uuidv4();
						const coord = coordinatorHandle(client);
						const reg = (await coord.registerHookToken({
							hookId,
							runId,
							token: hookData.token,
							status: "pending",
							createdAt: Date.now(),
						})) as
							| { ok: true }
							| { ok: false; reason: string };
						if (!reg.ok) {
							await runActor.createEvent(
								runId,
								{
									eventType: "hook_conflict",
									correlationId: data.correlationId,
									eventData: { token: hookData.token },
								},
								params
									? { requestId: params.requestId }
									: undefined,
							);
							throw new HookConflictError(hookData.token);
						}
					}
				}

				const result = (await runActor.createEvent(
					runId,
					data,
					params ? { requestId: params.requestId } : undefined,
				)) as EventResult;

				if (result.run) {
					const run = result.run;
					await coordinatorHandle(client).updateRunStatus(
						run.runId,
						run.status,
						run.updatedAt instanceof Date
							? run.updatedAt.getTime()
							: Date.parse(
									run.updatedAt as unknown as string,
								),
					);
					if (
						run.status === "completed" ||
						run.status === "failed" ||
						run.status === "cancelled"
					) {
						await coordinatorHandle(
							client,
						).disposeHookTokensForRun(run.runId);
					}
				}

				return result;
			}) as World["events"]["create"],

			async get(
				runId: string,
				eventId: string,
				_params?: GetEventParams,
			): Promise<WorldEvent> {
				const ev = (await runHandle(
					client,
					runId,
				).getEvent(eventId)) as WorldEvent | null;
				if (!ev) throw new NotFoundError("event", eventId);
				return ev;
			},

			async list(
				params: ListEventsParams,
			): Promise<PaginatedResponse<WorldEvent>> {
				const pagination = params.pagination ?? {};
				const res = (await runHandle(
					client,
					params.runId,
				).listEvents({
					cursor: pagination.cursor,
					limit: pagination.limit,
					sortOrder: pagination.sortOrder,
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
				// Correlation ids are scoped to a single run. Without a
				// global correlation index we cannot resolve the runId.
				// Callers that need this must use events.list on the known
				// run. Return empty for now.
				return {
					data: [],
					cursor: null,
					hasMore: false,
				};
			},
		},
		hooks: {
			async get(hookId: string, _params?: GetHookParams): Promise<Hook> {
				const lookup = (await coordinatorHandle(
					client,
				).lookupHookId(hookId)) as {
					runId: string;
					token: string;
				} | null;
				if (!lookup) throw new NotFoundError("hook", hookId);
				const hook = (await runHandle(
					client,
					lookup.runId,
				).getHook(hookId)) as Hook | null;
				if (!hook) throw new NotFoundError("hook", hookId);
				return hook;
			},
			async getByToken(
				token: string,
				_params?: GetHookParams,
			): Promise<Hook> {
				const row = (await coordinatorHandle(
					client,
				).lookupHookToken(token)) as {
					hookId: string;
					runId: string;
					token: string;
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
				if (!params.runId) {
					return { data: [], cursor: null, hasMore: false };
				}
				const pagination = params.pagination ?? {};
				const res = (await runHandle(
					client,
					params.runId,
				).listHooks({
					cursor: pagination.cursor,
					limit: pagination.limit,
					sortOrder: pagination.sortOrder,
				})) as unknown as {
					data: Hook[];
					cursor: string | null;
					hasMore: boolean;
				};
				return revivePaginated<Hook>(res);
			},
		},

		// =============================================================
		// Queue
		// =============================================================

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
				opts
					? {
							idempotencyKey: opts.idempotencyKey,
							delay: opts.delaySeconds
								? opts.delaySeconds * 1000
								: undefined,
						}
					: undefined,
			)) as { messageId: string };

			const delayMs = opts?.delaySeconds
				? opts.delaySeconds * 1000
				: 0;
			if (delayMs > 0) {
				setTimeout(() => {
					dispatchMessage(queueName).catch(() => {});
				}, delayMs);
			} else {
				dispatchMessage(queueName).catch(() => {});
			}

			return { messageId: res.messageId };
		},

		createQueueHandler(
			queueNamePrefix: QueuePrefix,
			handler: QueueHandler,
		): (req: Request) => Promise<Response> {
			queueHandlers.set(queueNamePrefix, handler);

			return async (req: Request): Promise<Response> => {
				let body: { queueName?: ValidQueueName };
				try {
					body = (await req.json()) as typeof body;
				} catch {
					return new Response("invalid body", { status: 400 });
				}

				const queueName = body.queueName;
				if (
					!queueName ||
					!queueName.startsWith(queueNamePrefix)
				) {
					return new Response("queue name mismatch", {
						status: 400,
					});
				}

				try {
					await dispatchMessage(queueName);
					return new Response(
						JSON.stringify({ ok: true }),
						{
							status: 200,
							headers: {
								"content-type": "application/json",
							},
						},
					);
				} catch (err) {
					return new Response(
						JSON.stringify({
							ok: false,
							error:
								err instanceof Error
									? err.message
									: String(err),
						}),
						{
							status: 500,
							headers: {
								"content-type": "application/json",
							},
						},
					);
				}
			};
		},

		// =============================================================
		// Streamer
		// =============================================================

		streams: {
			async write(
				runId: string,
				name: string,
				chunk: string | Uint8Array,
			): Promise<void> {
				await runHandle(client, runId).writeStream(name, [chunk]);
			},
			async writeMulti(
				runId: string,
				name: string,
				chunks: (string | Uint8Array)[],
			): Promise<void> {
				await runHandle(client, runId).writeStream(name, chunks);
			},
			async close(runId: string, name: string): Promise<void> {
				await runHandle(client, runId).closeStream(name);
			},
			async getChunks(
				runId: string,
				name: string,
				options?: GetChunksOptions,
			): Promise<StreamChunksResponse> {
				const res = (await runHandle(
					client,
					runId,
				).getStreamChunks(name, options)) as {
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
			async getInfo(
				runId: string,
				name: string,
			): Promise<StreamInfoResponse> {
				return (await runHandle(client, runId).getStreamInfo(
					name,
				)) as StreamInfoResponse;
			},
			async list(runId: string): Promise<string[]> {
				return (await runHandle(
					client,
					runId,
				).listStreams()) as string[];
			},
			async get(
				runId: string,
				name: string,
				startIndex = 0,
			): Promise<ReadableStream<Uint8Array>> {
				return new ReadableStream<Uint8Array>({
					async start(controller) {
						let cursor: string | undefined;
						while (true) {
							const res = (await runHandle(
								client,
								runId,
							).getStreamChunks(name, {
								cursor,
							})) as {
								data: {
									index: number;
									data: string;
								}[];
								cursor: string | null;
								hasMore: boolean;
								done: boolean;
							};

							for (const chunk of res.data) {
								if (chunk.index >= startIndex) {
									controller.enqueue(
										decodeBinary(chunk.data),
									);
								}
							}

							if (!res.hasMore) {
								if (res.done) {
									controller.close();
									return;
								}
								break;
							}
							cursor = res.cursor ?? undefined;
						}

						// Subscribe to live updates.
						const handle = runHandle(client, runId);
						const conn = handle.connect();
						conn.on(
							"streamAppended",
							(
								payload: {
									streamName: string;
									chunks: {
										index: number;
										data: string;
									}[];
									done: boolean;
								},
							) => {
								if (payload.streamName !== name) return;
								for (const chunk of payload.chunks) {
									if (chunk.index >= startIndex) {
										controller.enqueue(
											decodeBinary(chunk.data),
										);
									}
								}
								if (payload.done) {
									controller.close();
									conn.dispose();
								}
							},
						);
					},
				});
			},
		},

		// =============================================================
		// Lifecycle
		// =============================================================

		async start(): Promise<void> {},
		async close(): Promise<void> {},
		getEncryptionKeyForRun: cfg.getEncryptionKeyForRun as
			| World["getEncryptionKeyForRun"]
			| undefined,
	};

	return world;
}

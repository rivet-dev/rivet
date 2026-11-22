/**
 * Runs Effect Cluster entities on Rivet Actors.
 *
 * Every entity instance is one Rivet Actor: an actor named after the entity
 * type, keyed by the entity id. The actor's SQLite database is the entity's
 * mailbox and RivetKit schedules provide its wake time, so there is no shard
 * routing, runner fleet, or external message storage. {@link layer} provides
 * the cluster `Sharding` service for any process that calls entities;
 * {@link serve} also hosts the entity actors on a Rivet node.
 *
 * Portions adapted from Effect (MIT License,
 * Copyright (c) 2023 Effectful Technologies Inc). See `LICENSE-effect`.
 *
 * @since 3.1.2
 */
import * as Cause from "effect/Cause";
import { Clock } from "effect/Clock";
import * as Context from "effect/Context";
import { MailboxFull, PersistenceError } from "effect/cluster/ClusterError";
import {
	isUninterruptibleForClient,
	Persisted,
	Uninterruptible,
} from "effect/cluster/ClusterSchema";
import * as DeliverAt from "effect/cluster/DeliverAt";
import type * as Entity from "effect/cluster/Entity";
import * as EntityAddress from "effect/cluster/EntityAddress";
import * as EntityId from "effect/cluster/EntityId";
import * as Envelope from "effect/cluster/Envelope";
import * as ShardId from "effect/cluster/ShardId";
import { Sharding } from "effect/cluster/Sharding";
import * as Deferred from "effect/Deferred";
import * as Duration from "effect/Duration";
import * as Effect from "effect/Effect";
import * as Exit from "effect/Exit";
import * as Layer from "effect/Layer";
import * as Option from "effect/Option";
import * as PersistedQueue from "effect/persistence/PersistedQueue";
import type * as Rpc from "effect/rpc/Rpc";
import * as RpcClient from "effect/rpc/RpcClient";
import { type FromClient, RequestId } from "effect/rpc/RpcMessage";
import * as RpcSchema from "effect/rpc/RpcSchema";
import * as Schedule from "effect/Schedule";
import * as Schema from "effect/Schema";
import * as Scope from "effect/Scope";
import * as Stream from "effect/Stream";
import * as Workflow from "effect/workflow/Workflow";
import * as WorkflowEngine from "effect/workflow/WorkflowEngine";
import {
	type AnyActorDefinition,
	type RegistryConfigInput,
	setup,
} from "rivetkit";
import {
	type Client,
	type ClientConfigInput,
	createClient,
} from "rivetkit/client";
import { setWithEviction } from "./internal/boundedMap.ts";
import { decodeName } from "./internal/clusterName.ts";
import {
	type EntityActorHost,
	makeEntityActor,
} from "./internal/entityActor.ts";
import { maximumEncodedSize } from "./internal/entityMailbox.ts";
import {
	type EntityRegistration,
	type EntityRegistry,
	makeEntityRegistry,
} from "./internal/entityRegistry.ts";
import {
	CurrentEntityName,
	CurrentReplyRegistry,
} from "./internal/entityReply.ts";
import {
	decodeInvokeResult,
	decodeReplyFor,
	encodeRequest,
} from "./internal/entityWire.ts";
import { makeQueueActor, queueActorName } from "./internal/queueActor.ts";
import type { QueueItem } from "./internal/queueStorage.ts";
import {
	makeSingletonActor,
	singletonActorName,
} from "./internal/singletonActor.ts";
import {
	makeSingletonRegistry,
	type SingletonRegistration,
	type SingletonRegistry,
} from "./internal/singletonRegistry.ts";
import { minimumHeartbeatMillis } from "./internal/timer.ts";
import { boundedRetry, callActor } from "./internal/transport.ts";
import type { EntityDeliveryOptions } from "./internal/types.ts";
import {
	makeWorkflowActor,
	workflowActorName,
} from "./internal/workflowActor.ts";
import {
	CurrentExecutionHandle,
	makeWorkflowRegistry,
	type WorkflowRegistration,
	type WorkflowRegistry,
	type WorkflowStub,
} from "./internal/workflowRegistry.ts";
import { InterruptSignalName } from "./internal/workflowRuntime.ts";
import {
	decodeExit,
	decodeResult,
	encodeExit,
	encodePayload,
} from "./internal/workflowWire.ts";

/**
 * Options shared by {@link layer} and {@link serve}.
 *
 * @since 3.1.2
 */
export interface Options {
	/**
	 * Every entity type this process calls or hosts. A client or handler
	 * registration for any other type dies before an actor is contacted.
	 */
	readonly entities: ReadonlyArray<Entity.Entity<any, any>>;
	/**
	 * RivetKit client configuration used to reach entity actors. Unset fields
	 * follow RivetKit's standard resolution, such as `RIVET_ENDPOINT`.
	 */
	readonly client?: ClientConfigInput | undefined;
	/**
	 * Interval of the wake an entity actor keeps scheduled while it holds
	 * `Entity.keepAlive` or runs a persisted request. It bounds how long an
	 * entity stays down after its node is lost. Defaults to 30 seconds.
	 */
	readonly keepAliveHeartbeat?: Duration.Input | undefined;
	/**
	 * Upper bound of one entity action. An ask waits inside the action for its
	 * reply, so this caps how long an ask may run. Defaults to 15 minutes.
	 */
	readonly actionTimeout?: Duration.Input | undefined;
}

const defaultKeepAliveHeartbeat = Duration.seconds(30);

const defaultActionTimeout = Duration.minutes(15);

// Room for the action envelope around a maximum-size encoded message.
const messageOverheadBytes = 64 * 1024;

const maxActorNameBytes = 64;

const textEncoder = new TextEncoder();

const singletonWakeRetry = Schedule.max([
	Schedule.exponential(200, 2),
	Schedule.recurs(8),
]);

/** The actions of an entity actor, as a client calls them. */
interface EntityStub {
	readonly invoke: (
		envelope: string,
		discard: boolean,
		delivery?: EntityDeliveryOptions,
	) => Promise<unknown>;
	readonly acknowledge: (
		requestId: string,
		replyId: string,
	) => Promise<ReadonlyArray<string>>;
	readonly interrupt: (
		storageRequestId: string,
		clientRequestId?: string,
	) => Promise<void>;
	readonly reset: (requestId: string) => Promise<void>;
	readonly deliverReply: (
		requestId: string,
		reply: string,
	) => Promise<boolean>;
}

/** The part of a RivetKit actor handle the stub uses. */
interface ActionHandle {
	action(options: {
		readonly name: string;
		readonly args: Array<unknown>;
	}): Promise<unknown>;
}

const makeEntityStub = (handle: ActionHandle): EntityStub => {
	const call = (name: string, args: Array<unknown>) =>
		handle.action({ name, args }) as Promise<any>;
	return {
		invoke: (envelope, discard, delivery) =>
			call("invoke", [envelope, discard, delivery ?? null]),
		acknowledge: (requestId, replyId) =>
			call("acknowledge", [requestId, replyId]),
		interrupt: (storageRequestId, clientRequestId) =>
			call("interrupt", [storageRequestId, clientRequestId ?? null]),
		reset: (requestId) => call("reset", [requestId]),
		deliverReply: (requestId, reply) =>
			call("deliverReply", [requestId, reply]),
	};
};

interface ClientTargetValue {
	readonly address: EntityAddress.EntityAddress;
	// A stub whose call threw is not reused, so every call resolves a fresh one.
	readonly resolve: () => EntityStub;
}

class ClientTarget extends Context.Service<ClientTarget, ClientTargetValue>()(
	"@rivetkit/effect-cluster/RivetCluster/ClientTarget",
) {}

const uuidV7 = (timestamp: number): string => {
	const bytes = crypto.getRandomValues(new Uint8Array(16));
	bytes[0] = Math.floor(timestamp / 2 ** 40);
	bytes[1] = Math.floor(timestamp / 2 ** 32) & 0xff;
	bytes[2] = Math.floor(timestamp / 2 ** 24) & 0xff;
	bytes[3] = Math.floor(timestamp / 2 ** 16) & 0xff;
	bytes[4] = Math.floor(timestamp / 2 ** 8) & 0xff;
	bytes[5] = timestamp & 0xff;
	bytes[6] = (bytes[6] & 0x0f) | 0x70;
	bytes[8] = (bytes[8] & 0x3f) | 0x80;
	const hex = (byte: number) => byte.toString(16).padStart(2, "0");
	return [
		bytes.subarray(0, 4),
		bytes.subarray(4, 6),
		bytes.subarray(6, 8),
		bytes.subarray(8, 10),
		bytes.subarray(10),
	]
		.map((part) => Array.from(part, hex).join(""))
		.join("-");
};

const requestTargetCapacity = 4096;

// ClusterCron owns this entity inside its layer, so callers cannot include it
// in `entities`. Its reserved shape is the only implicit entity.
const isClusterCronEntity = (entity: Entity.Entity<any, any>): boolean => {
	if (
		!entity.type.startsWith("ClusterCron/") ||
		entity.protocol.requests.size !== 2
	)
		return false;
	const run = entity.protocol.requests.get("run");
	const start = entity.protocol.requests.get("start");
	return (
		run !== undefined &&
		start !== undefined &&
		Context.get(run.annotations, Persisted) &&
		Context.get(run.annotations, Uninterruptible) === true &&
		Context.get(start.annotations, Persisted)
	);
};

const notSupported = (method: string) =>
	Effect.die(
		new Error(`RivetCluster: ${method} is not supported on Rivet Actors`),
	);

/**
 * The state one cluster layer shares with the actors it hosts.
 *
 * @internal
 */
interface ClusterRuntime {
	readonly entities: ReadonlyMap<string, Entity.Entity<any, any>>;
	readonly registry: EntityRegistry;
	readonly singletons: SingletonRegistry;
	readonly workflows: WorkflowRegistry;
	readonly deferredState: ReturnType<typeof WorkflowEngine.makeDeferredState>;
	readonly getWorkflowStub: (
		workflowName: string,
		executionId: string,
	) => WorkflowStub;
	readonly host: EntityActorHost;
	readonly heartbeatMillis: number;
	/** Asks the singleton actor of `name` to run its registered effect. */
	readonly wakeSingleton: (name: string) => Promise<unknown>;
	readonly actionTimeoutMillis: number;
	/** Marks registration complete, which lets entity actors start. */
	readonly markReady: () => void;
}

class RivetClusterRuntime extends Context.Service<
	RivetClusterRuntime,
	ClusterRuntime
>()("@rivetkit/effect-cluster/RivetCluster/Runtime") {}

const makeWorkflowStub = (handle: ActionHandle): WorkflowStub => {
	const call = (name: string, args: Array<unknown>) =>
		handle.action({ name, args }) as Promise<any>;
	return {
		run: (payload, options) =>
			call("run", [
				payload,
				{ discard: options.discard, parent: options.parent ?? null },
			]),
		poll: () =>
			call("poll", []).then(
				(result: string | null) => result ?? undefined,
			),
		resume: (mode) => call("resume", [mode ?? null]),
		interrupt: () => call("interrupt", []),
		interruptUnsafe: () => call("interruptUnsafe", []),
		deferredDone: (name, exit) => call("deferredDone", [name, exit]),
		scheduleClock: (name, deferredName, wakeUp) =>
			call("scheduleClock", [name, deferredName, wakeUp]),
	};
};

// A transport failure is retried on an exponential schedule capped at one
// minute. Every workflow call is idempotent by key, and a permanently
// unreachable actor stalls the caller instead of failing a workflow for good.
const workflowCallRetry = Schedule.min([
	Schedule.exponential(200, 1.5),
	Schedule.spaced("1 minute"),
]);

/**
 * The `WorkflowEngine` backed by workflow actors. Inside a workflow actor the
 * engine uses the execution's own handle, so activities and deferred reads
 * never leave the actor. Everywhere else it calls the execution's actor.
 */
const makeWorkflowEngine = (options: {
	readonly workflows: WorkflowRegistry;
	readonly getWorkflowStub: (
		workflowName: string,
		executionId: string,
	) => WorkflowStub;
	readonly currentTimeMillis: () => number;
}): WorkflowEngine.WorkflowEngine["Service"] => {
	const callExecution = <A>(
		workflowName: string,
		executionId: string,
		call: (stub: WorkflowStub) => Promise<A>,
	): Effect.Effect<A> =>
		Effect.flatMap(
			Effect.serviceOption(CurrentExecutionHandle),
			(handle) => {
				if (
					Option.isSome(handle) &&
					handle.value.executionId === executionId
				) {
					const local: WorkflowStub = handle.value;
					return Effect.promise(() => call(local));
				}
				return callActor({
					resolve: () =>
						options.getWorkflowStub(workflowName, executionId),
					call,
					retry: workflowCallRetry,
				}).pipe(Effect.orDie);
			},
		);

	const localHandle = Effect.fnUntraced(function* (operation: string) {
		const instance = yield* WorkflowEngine.WorkflowInstance;
		const handle = yield* Effect.serviceOption(CurrentExecutionHandle);
		if (Option.isNone(handle)) {
			return yield* Effect.die(
				`RivetWorkflowEngine: ${operation} is only available inside a workflow actor execution`,
			);
		}
		return { instance, handle: handle.value } as const;
	});

	return WorkflowEngine.makeUnsafe({
		register: Effect.fnUntraced(function* (workflow, execute) {
			const context = yield* Effect.context<never>();
			const registration: WorkflowRegistration = {
				workflow,
				execute,
				context,
			};
			if (!options.workflows.register(workflow._tag, registration))
				return;
			yield* Effect.addFinalizer(() =>
				Effect.sync(() => {
					options.workflows.unregister(workflow._tag, registration);
				}),
			);
		}),

		execute: Effect.fnUntraced(function* (workflow, opts) {
			const context = yield* Effect.context<never>();
			const payload = yield* encodePayload(
				workflow,
				opts.payload,
				context,
			);
			// A detached child keeps no parent, so neither its completion nor a
			// failed run wakes the parent.
			const parent =
				opts.parent === undefined || opts.detached
					? undefined
					: {
							workflowName: opts.parent.workflow._tag,
							executionId: opts.parent.executionId,
						};
			const text = yield* callExecution(
				workflow._tag,
				opts.executionId,
				(stub) => stub.run(payload, { discard: opts.discard, parent }),
			);
			if (opts.discard) return undefined;
			return yield* decodeResult(workflow, text, context);
		}) as WorkflowEngine.Encoded["execute"],

		poll: Effect.fnUntraced(function* (workflow, executionId) {
			const context = yield* Effect.context<never>();
			const text = yield* callExecution(
				workflow._tag,
				executionId,
				(stub) => stub.poll(),
			);
			if (text === undefined) return Option.none();
			return Option.some(yield* decodeResult(workflow, text, context));
		}),

		interrupt: (workflow, executionId) =>
			callExecution(workflow._tag, executionId, (stub) =>
				stub.interrupt(),
			),

		interruptUnsafe: (workflow, executionId) =>
			callExecution(workflow._tag, executionId, (stub) =>
				stub.interruptUnsafe(),
			),

		resume: (workflow, executionId) =>
			callExecution(workflow._tag, executionId, (stub) => stub.resume()),

		activityExecute: Effect.fnUntraced(function* (activity, attempt) {
			const { handle, instance } =
				yield* localHandle("Activity execution");
			const key = `${activity.name}/${attempt}`;
			const stored = yield* handle.loadActivity(key);
			if (stored !== undefined) {
				return new Workflow.Complete({
					exit: yield* decodeExit(stored, Context.empty()),
				});
			}
			// A pending interrupt replays only to rebuild compensations, so it
			// must not start new activities. Suspending lets the run apply it.
			const interruptSignal =
				yield* handle.loadDeferred(InterruptSignalName);
			if (!instance.interrupted && interruptSignal !== undefined) {
				instance.suspended = true;
				return new Workflow.Suspended({});
			}
			const activityInstance = WorkflowEngine.WorkflowInstance.initial(
				instance.workflow,
				instance.executionId,
			);
			activityInstance.interrupted = instance.interrupted;
			const result = yield* activity.executeEncoded.pipe(
				Workflow.intoResult,
				Effect.provideService(
					WorkflowEngine.WorkflowInstance,
					activityInstance,
				),
			);
			if (result._tag === "Complete") {
				yield* handle.saveActivity(
					key,
					yield* encodeExit(result.exit, Context.empty()),
				);
			}
			return result;
		}),

		deferredResult: Effect.fnUntraced(function* (deferred) {
			const { handle } = yield* localHandle("DurableDeferred reads");
			const stored = yield* handle.loadDeferred(deferred.name);
			if (stored === undefined) return Option.none();
			return Option.some(yield* decodeExit(stored, Context.empty()));
		}),

		deferredDone: Effect.fnUntraced(function* ({
			deferredName,
			executionId,
			exit,
			workflowName,
		}) {
			const text = yield* encodeExit(exit, Context.empty());
			const handle = yield* Effect.serviceOption(CurrentExecutionHandle);
			if (
				Option.isSome(handle) &&
				handle.value.executionId === executionId
			) {
				return yield* handle.value.completeOwnDeferred(
					deferredName,
					text,
				);
			}
			yield* callExecution(workflowName, executionId, (stub) =>
				stub.deferredDone(deferredName, text),
			);
		}),

		scheduleClock: Effect.fnUntraced(function* (workflow, opts) {
			const wakeUp =
				options.currentTimeMillis() +
				Math.ceil(Duration.toMillis(opts.clock.duration));
			yield* callExecution(workflow._tag, opts.executionId, (stub) =>
				stub.scheduleClock(
					opts.clock.name,
					opts.clock.deferred.name,
					wakeUp,
				),
			);
		}),
	});
};

/** The actions of a queue actor, as the queue store calls them. */
interface QueueStub {
	readonly offer: (
		id: string,
		element: string,
		delayMillis: number,
		deduplicate: "untilExecuted" | "untilRemoved" | undefined,
	) => Promise<void>;
	readonly take: (
		takerId: string,
		maxAttempts: number,
		leaseMillis: number,
	) => Promise<QueueItem | null>;
	readonly cancelTake: (takerId: string) => Promise<void>;
	readonly complete: (id: string, takerId: string) => Promise<void>;
	readonly fail: (
		id: string,
		lastFailure: string,
		takerId: string,
		terminal: boolean,
		retryDelayMillis: number,
	) => Promise<void>;
	readonly release: (id: string, takerId: string) => Promise<void>;
	readonly extend: (
		id: string,
		leaseMillis: number,
		takerId: string,
	) => Promise<void>;
}

const makeQueueStub = (handle: ActionHandle): QueueStub => {
	const call = (name: string, args: Array<unknown>) =>
		handle.action({ name, args }) as Promise<any>;
	return {
		offer: (id, element, delayMillis, deduplicate) =>
			call("offer", [id, element, delayMillis, deduplicate ?? null]),
		take: (takerId, maxAttempts, leaseMillis) =>
			call("take", [takerId, maxAttempts, leaseMillis]),
		cancelTake: (takerId) => call("cancelTake", [takerId]),
		complete: (id, takerId) => call("complete", [id, takerId]),
		fail: (id, lastFailure, takerId, terminal, retryDelayMillis) =>
			call("fail", [
				id,
				lastFailure,
				takerId,
				terminal,
				retryDelayMillis,
			]),
		release: (id, takerId) => call("release", [id, takerId]),
		extend: (id, leaseMillis, takerId) =>
			call("extend", [id, leaseMillis, takerId]),
	};
};

const queueLeaseMillis = 120_000;

const queueLeaseRefreshMillis = 30_000;

const queueAttempt = (run: () => Promise<void>) =>
	Effect.promise(run).pipe(
		Effect.sandbox,
		Effect.retry({ times: 5, schedule: Schedule.exponential(100, 1.5) }),
	);

const queueFinalize = (run: () => Promise<void>): Effect.Effect<void> =>
	Effect.orDie(queueAttempt(run));

/** The `PersistedQueueStore` backed by queue actors, one per queue name. */
const makePersistedQueueStore = (
	getQueueStub: (name: string) => QueueStub,
): PersistedQueue.PersistedQueueStore["Service"] =>
	PersistedQueue.PersistedQueueStore.of({
		cleanup: () =>
			Effect.fail(
				new PersistedQueue.PersistedQueueError({
					message:
						"RivetCluster persisted queue cleanup is not supported yet",
				}),
			),
		offer: ({ deduplicate, delay, element, id, isCustomId, name }) =>
			Effect.tryPromise({
				try: () =>
					getQueueStub(name).offer(
						id,
						JSON.stringify(element),
						delay === undefined ? 0 : Duration.toMillis(delay),
						// Generated ids are never offered twice.
						isCustomId ? deduplicate : undefined,
					),
				catch: (cause) =>
					new PersistedQueue.PersistedQueueError({
						message: "Failed to offer element to persisted queue",
						cause,
					}),
			}),
		take: ({ maxAttempts, name, retryDelay }) =>
			// Uninterruptible outside `restore` so the release finalizer is always
			// registered once an item is leased. An interrupt while still waiting
			// cancels the take by taker id, which releases an item already
			// leased to it.
			Effect.uninterruptibleMask((restore) =>
				Effect.gen(function* () {
					const takerId = crypto.randomUUID();
					// Each call waits a bounded time and returns nothing when the
					// queue stays empty; a failed call is retried with the same
					// taker id, so nothing is lost.
					const takeOnce = Effect.promise(() =>
						getQueueStub(name).take(
							takerId,
							maxAttempts,
							queueLeaseMillis,
						),
					).pipe(
						Effect.sandbox,
						Effect.tapCause((cause) =>
							Effect.logWarning(
								"PersistedQueue take failed, retrying",
								cause,
							),
						),
						Effect.retry(Schedule.spaced(500)),
						Effect.orDie,
					);
					const item = yield* restore(
						takeOnce.pipe(
							Effect.repeat({ until: (taken) => taken !== null }),
							Effect.map((taken) => taken as QueueItem),
						),
					).pipe(
						Effect.onInterrupt(() =>
							Effect.ignore(
								queueAttempt(() =>
									getQueueStub(name).cancelTake(takerId),
								),
							),
						),
					);
					yield* Effect.addFinalizer(
						Exit.match({
							onFailure: (cause) => {
								if (Cause.hasInterruptsOnly(cause)) {
									return queueFinalize(() =>
										getQueueStub(name).release(
											item.id,
											takerId,
										),
									);
								}
								if (PersistedQueue.isDeadLetterCause(cause)) {
									return queueFinalize(() =>
										getQueueStub(name).fail(
											item.id,
											Cause.pretty(cause),
											takerId,
											true,
											0,
										),
									);
								}
								// The delay follows the attempt count including this one.
								return Effect.flatMap(
									retryDelay(item.attempts + 1),
									(delay) =>
										queueFinalize(() =>
											getQueueStub(name).fail(
												item.id,
												Cause.pretty(cause),
												takerId,
												false,
												Duration.toMillis(delay),
											),
										),
								);
							},
							onSuccess: () =>
								queueFinalize(() =>
									getQueueStub(name).complete(
										item.id,
										takerId,
									),
								),
						}),
					);
					yield* Effect.promise(() =>
						getQueueStub(name).extend(
							item.id,
							queueLeaseMillis,
							takerId,
						),
					).pipe(
						Effect.sandbox,
						Effect.ignore,
						Effect.schedule(
							Schedule.spaced(queueLeaseRefreshMillis),
						),
						Effect.forkScoped,
						Effect.interruptible,
					);
					return {
						id: item.id,
						attempts: item.attempts + 1,
						element: JSON.parse(item.element),
					};
				}),
			),
	});

const make = Effect.fnUntraced(function* (options: Options) {
	const layerScope = yield* Effect.scope;
	const layerContext = yield* Effect.context<never>();
	const entities = new Map<string, Entity.Entity<any, any>>();
	for (const entity of options.entities) {
		entities.set(entity.type, entity);
	}
	const clock = yield* Clock;
	const heartbeatMillis = Duration.toMillis(
		Duration.fromInputUnsafe(
			options.keepAliveHeartbeat ?? defaultKeepAliveHeartbeat,
		),
	);
	if (heartbeatMillis < minimumHeartbeatMillis) {
		return yield* Effect.die(
			new Error(
				`RivetCluster: keepAliveHeartbeat must be at least ${minimumHeartbeatMillis} ms`,
			),
		);
	}
	const actionTimeoutMillis = Duration.toMillis(
		Duration.fromInputUnsafe(options.actionTimeout ?? defaultActionTimeout),
	);
	const registry = makeEntityRegistry();
	const singletons = makeSingletonRegistry();
	const workflows = makeWorkflowRegistry();
	const deferredState = WorkflowEngine.makeDeferredState();
	const rivetClient: Client<any> = yield* Effect.acquireRelease(
		Effect.sync(() => createClient(options.client ?? {})),
		(client) => Effect.promise(() => client.dispose()),
	);
	const requestTargets = new Map<
		string,
		{ readonly resolve: () => EntityStub; storageRequestId: string }
	>();

	const resolveEntity = (entityType: string, entityId: string): EntityStub =>
		makeEntityStub(rivetClient.getOrCreate(entityType, [entityId]));

	const ready = Promise.withResolvers<void>();
	const host: EntityActorHost = {
		registry,
		heartbeatMillis,
		ready: ready.promise,
		deliverReply: (replyTo, requestId, reply) => {
			const name = decodeName(replyTo);
			if (name === undefined) return Promise.resolve(false);
			return resolveEntity(name.type, name.id).deliverReply(
				requestId,
				reply,
			);
		},
	};

	const wakeSingleton = (name: string): Promise<unknown> =>
		rivetClient
			.getOrCreate(singletonActorName, [name])
			.action({ name: "wake", args: [] });

	const unknownEntity = (entity: Entity.Entity<any, any>) =>
		Effect.die(
			new Error(
				`RivetCluster: entity type "${entity.type}" is not part of the cluster's entities`,
			),
		);

	const buildClient = Effect.fnUntraced(function* (
		entity: Entity.Entity<any, any>,
	) {
		type ClientEntry = {
			readonly rpc: Rpc.AnyWithProps;
			readonly context: Context.Context<never>;
			readonly clientRequestId: string;
			storageRequestId: string;
			lastChunkId?: string;
		};
		const entries = new Map<string, ClientEntry>();

		// `handleFromClient` is hoisted and only invoked once requests are made,
		// after `client` is constructed.
		const client = yield* RpcClient.makeNoSerialization<
			any,
			MailboxFull | PersistenceError,
			true
		>(entity.protocol, {
			spanPrefix: `${entity.type}.client`,
			supportsAck: true,
			generateRequestId: () =>
				RequestId(uuidV7(clock.currentTimeMillisUnsafe())),
			flatten: true,
			onFromClient: (options) => handleFromClient(options),
		});

		const deliverReplies = (
			entry: ClientEntry,
			replyTexts: ReadonlyArray<string>,
		): Effect.Effect<void> =>
			Effect.forEach(
				replyTexts,
				(replyText) =>
					Effect.flatMap(
						decodeReplyFor(entry.rpc, entry.context, replyText),
						(reply) => {
							if (reply._tag === "Chunk") {
								entry.lastChunkId = String(reply.id);
								return client.write({
									_tag: "Chunk",
									clientId: 0,
									requestId: RequestId(entry.clientRequestId),
									values: reply.values,
								});
							}
							entries.delete(entry.clientRequestId);
							return client.write({
								_tag: "Exit",
								clientId: 0,
								requestId: RequestId(entry.clientRequestId),
								exit: reply.exit,
							});
						},
					),
				{ discard: true },
			);

		function handleFromClient({
			context,
			message,
		}: {
			readonly message: FromClient<any>;
			readonly context: Context.Context<never>;
		}): Effect.Effect<void, MailboxFull | PersistenceError> {
			const target = Context.getUnsafe(context, ClientTarget);
			switch (message._tag) {
				case "Request": {
					const discard = message.discard === true;
					const rpc = entity.protocol.requests.get(
						message.tag,
					)! as Rpc.AnyWithProps;
					const clientRequestId = String(message.id);
					const encode = Schema.encodeUnknownEffect(
						Schema.toCodecJson(rpc.payloadSchema),
					)(message.payload).pipe(
						Effect.provideContext(context as any),
						Effect.orDie,
					) as unknown as Effect.Effect<unknown>;
					return Effect.flatMap(encode, (payload) => {
						const envelope = encodeRequest({
							requestId: clientRequestId,
							address: target.address,
							tag: message.tag,
							payload,
							headers: message.headers,
							...(message.traceId === undefined
								? undefined
								: {
										traceId: message.traceId,
										spanId: message.spanId,
										sampled: message.sampled,
									}),
						});
						const entry: ClientEntry = {
							rpc,
							context,
							clientRequestId,
							storageRequestId: clientRequestId,
						};
						if (!discard) entries.set(clientRequestId, entry);
						const deliverAt = DeliverAt.toMillis(message.payload);
						const delayed =
							deliverAt !== null &&
							deliverAt > clock.currentTimeMillisUnsafe();
						const persisted = Context.get(
							rpc.annotations,
							Persisted,
						);
						const primaryKey = Envelope.primaryKey({
							...message,
							requestId: message.id,
							address: target.address,
							headers: message.headers,
						} as any);
						const replyTo = discard
							? undefined
							: Option.getOrUndefined(
									Context.getOption(
										context,
										CurrentEntityName,
									),
								);
						const replyRegistry = discard
							? undefined
							: Option.getOrUndefined(
									Context.getOption(
										context,
										CurrentReplyRegistry,
									),
								);
						if (
							delayed &&
							(!persisted || (!discard && primaryKey === null))
						) {
							entries.delete(clientRequestId);
							return Effect.fail(
								new PersistenceError({
									cause: new Error(
										!persisted
											? "Future DeliverAt requests must be persisted"
											: "Future DeliverAt asks must define a PrimaryKey",
									),
								}),
							);
						}
						if (
							delayed &&
							RpcSchema.isStreamSchema(rpc.successSchema)
						) {
							entries.delete(clientRequestId);
							return Effect.fail(
								new PersistenceError({
									cause: new Error(
										"Stream asks with a future DeliverAt are not supported",
									),
								}),
							);
						}
						const delivery = delayed
							? {
									deliverAt: deliverAt!,
									primaryKey,
									...(replyTo === undefined
										? undefined
										: { replyTo }),
								}
							: replyTo === undefined
								? undefined
								: { replyTo };
						// A pinned caller receives the scheduled reply pushed to its own
						// entity actor's `deliverReply` action, which completes this waiter.
						// The destination stores this request id with the reply target and
						// pushes under it, so registering before the send closes the race.
						const pushedReply =
							delivery?.replyTo !== undefined &&
							replyRegistry !== undefined
								? Deferred.makeUnsafe<string>()
								: undefined;
						if (pushedReply !== undefined)
							replyRegistry!.register(
								clientRequestId,
								pushedReply,
							);
						// The mailbox dedupes on the request id, so a persisted request is
						// safe to retry. A non-persisted one may already be running.
						const send = callActor({
							resolve: target.resolve,
							call: (stub) =>
								stub.invoke(envelope, discard, delivery),
							retry: persisted ? boundedRetry : undefined,
						}).pipe(
							Effect.flatMap(decodeInvokeResult),
							// biome-ignore lint/suspicious/useIterableCallbackReturn: this is Effect.flatMap, and every case returns.
							Effect.flatMap(
								(
									result,
								): Effect.Effect<
									void,
									MailboxFull | PersistenceError
								> => {
									switch (result._tag) {
										case "MailboxFull":
											return Effect.fail(
												new MailboxFull({
													address: target.address,
												}),
											);
										case "EncodedMessageTooLarge":
											return Effect.fail(
												new PersistenceError({
													cause: new Error(
														"Encoded entity message exceeds 2 MB",
													),
												}),
											);
										case "AskDeduplicatedToTell":
											return Effect.fail(
												new PersistenceError({
													cause: new Error(
														"Cannot deduplicate an ask onto a tell with the same PrimaryKey",
													),
												}),
											);
										case "Success": {
											entry.storageRequestId =
												result.requestId;
											if (!discard && persisted) {
												setWithEviction(
													requestTargets,
													clientRequestId,
													{
														resolve: target.resolve,
														storageRequestId:
															result.requestId,
													},
													requestTargetCapacity,
												);
											}
											// A discarded call is acknowledged once its message is delivered.
											if (discard) {
												return client.write({
													_tag: "Exit",
													clientId: 0,
													requestId:
														RequestId(
															clientRequestId,
														),
													exit: Exit.void as any,
												});
											}
											if (
												pushedReply !== undefined &&
												result.replies.length === 0
											) {
												return Effect.flatMap(
													Deferred.await(pushedReply),
													(replyText) =>
														deliverReplies(entry, [
															replyText,
														]),
												);
											}
											return deliverReplies(
												entry,
												result.replies,
											);
										}
									}
								},
							),
							// A failed send gets no `Exit` reply, so release its entry here.
							// An interrupt releases it in the `Interrupt` case instead, which
							// still needs it to reach the entity actor.
							Effect.onExit((exit) =>
								Exit.isFailure(exit) &&
								!Cause.hasInterruptsOnly(exit.cause)
									? Effect.sync(() =>
											entries.delete(clientRequestId),
										)
									: Effect.void,
							),
						);
						return pushedReply === undefined
							? send
							: Effect.ensuring(
									send,
									Effect.sync(() =>
										replyRegistry!.unregister(
											clientRequestId,
											pushedReply,
										),
									),
								);
					});
				}
				case "Ack": {
					const entry = entries.get(String(message.requestId));
					if (entry === undefined || entry.lastChunkId === undefined)
						return Effect.void;
					const lastChunkId = entry.lastChunkId;
					return callActor({
						resolve: target.resolve,
						call: (stub) =>
							stub.acknowledge(
								entry.storageRequestId,
								lastChunkId,
							),
						retry: boundedRetry,
					}).pipe(
						Effect.flatMap((replies) =>
							deliverReplies(entry, replies),
						),
					);
				}
				case "Interrupt": {
					const clientRequestId = String(message.requestId);
					const entry = entries.get(clientRequestId);
					entries.delete(clientRequestId);
					requestTargets.delete(clientRequestId);
					if (entry === undefined) return Effect.void;
					if (isUninterruptibleForClient(entry.rpc.annotations))
						return Effect.void;
					return callActor({
						resolve: target.resolve,
						call: (stub) =>
							stub.interrupt(
								entry.storageRequestId,
								clientRequestId,
							),
						retry: boundedRetry,
					});
				}
				default:
					return Effect.void;
			}
		}

		return (entityId: string) => {
			const id = EntityId.make(entityId);
			const target = ClientTarget.context({
				address: EntityAddress.make({
					shardId: ShardId.make(entity.getShardGroup(id), 1),
					entityId: id,
					entityType: entity.type,
				}),
				resolve: () => resolveEntity(entity.type, entityId),
			});
			const result: Record<string, unknown> = {};
			for (const tag of entity.protocol.requests.keys()) {
				const rpc = entity.protocol.requests.get(
					tag,
				)! as Rpc.AnyWithProps;
				const isStream = RpcSchema.isStreamSchema(rpc.successSchema);
				const contextFor = (
					ambient: Context.Context<never>,
					methodOptions?: {
						readonly context?: Context.Context<never>;
					},
				) => {
					const currentEntityName = Option.getOrUndefined(
						Context.getOption(ambient, CurrentEntityName),
					);
					const replyRegistry = Option.getOrUndefined(
						Context.getOption(ambient, CurrentReplyRegistry),
					);
					let requestContext =
						currentEntityName === undefined
							? target
							: Context.add(
									target,
									CurrentEntityName,
									currentEntityName,
								);
					if (replyRegistry !== undefined) {
						requestContext = Context.add(
							requestContext,
							CurrentReplyRegistry,
							replyRegistry,
						);
					}
					if (methodOptions?.context !== undefined) {
						requestContext = Context.merge(
							methodOptions.context,
							requestContext,
						);
					}
					return requestContext;
				};
				result[tag] = (
					payload: unknown,
					methodOptions?: {
						readonly context?: Context.Context<never>;
						readonly asQueue?: boolean;
					},
				) => {
					// The caller's context is read when the request runs, so it carries
					// the calling entity's reply target. The untyped protocol erases
					// the stream/queue return shape, hence `any`.
					const call = (ambient: Context.Context<never>): any =>
						client.client(tag, payload, {
							...methodOptions,
							context: contextFor(ambient, methodOptions),
						});
					// With `asQueue`, a stream RPC yields its Dequeue as an Effect.
					return isStream && methodOptions?.asQueue !== true
						? Stream.unwrap(
								Effect.map(Effect.context<never>(), call),
							)
						: Effect.contextWith(call);
				};
			}
			return result as any;
		};
	});

	type ClientFactory = (entityId: string) => any;
	// Each entity's base client is built once from the layer context and
	// scope, so it needs no caller scope, outlives the scope it was first
	// requested in, and resolves client middleware from the layer.
	const clientContext = Context.add(layerContext, Scope.Scope, layerScope);
	const clients = new Map<
		Entity.Entity<any, any>,
		Deferred.Deferred<ClientFactory>
	>();
	const makeClient = (
		entity: Entity.Entity<any, any>,
	): Effect.Effect<ClientFactory> =>
		Effect.suspend(() => {
			if (!entities.has(entity.type)) return unknownEntity(entity);
			const existing = clients.get(entity);
			if (existing !== undefined) return Deferred.await(existing);
			const deferred = Deferred.makeUnsafe<ClientFactory>();
			clients.set(entity, deferred);
			return buildClient(entity).pipe(
				Effect.provideContext(clientContext),
				Effect.tap((factory) => Deferred.succeed(deferred, factory)),
				Effect.uninterruptible,
			);
		});

	const registerEntity = Effect.fnUntraced(function* (
		entity: Entity.Entity<any, any>,
		build: Effect.Effect<unknown, never, unknown>,
		buildOptions?: Record<string, unknown>,
	) {
		const declared = entities.has(entity.type);
		if (!declared && !isClusterCronEntity(entity)) {
			return yield* unknownEntity(entity);
		}
		const context = yield* Effect.context<never>();
		const registration: EntityRegistration = {
			entity,
			build: build as any,
			options: buildOptions as EntityRegistration["options"],
			context,
		};
		if (!registry.register(entity.type, registration)) return;
		if (!declared) entities.set(entity.type, entity);
		yield* Effect.addFinalizer(() =>
			Effect.sync(() => {
				registry.unregister(entity.type, registration);
				if (!declared && entities.get(entity.type) === entity) {
					entities.delete(entity.type);
				}
			}),
		);
	});

	const registerSingleton = Effect.fnUntraced(function* (
		name: string,
		run: Effect.Effect<void, unknown, unknown>,
	) {
		const context = yield* Effect.context<never>();
		const registration: SingletonRegistration = {
			run: run as SingletonRegistration["run"],
			context,
		};
		if (!singletons.register(name, registration)) {
			return yield* Effect.die(
				`Singleton '${name}' is already registered`,
			);
		}
		yield* Effect.addFinalizer(() =>
			Effect.sync(() => singletons.unregister(name, registration)),
		);
	});

	const sharding = Sharding.of({
		getRegistrationEvents: Stream.never,
		getShardId: (_entityId, group) => ShardId.make(group, 1),
		hasShardId: () => true,
		// A Snowflake needs a machine id unique across the fleet, which actors
		// do not have. Request ids are uuidv7 instead.
		getSnowflake: notSupported("Sharding.getSnowflake"),
		isShutdown: Effect.succeed(false),
		makeClient: makeClient as Sharding["Service"]["makeClient"],
		registerEntity: registerEntity as Sharding["Service"]["registerEntity"],
		registerSingleton:
			registerSingleton as Sharding["Service"]["registerSingleton"],
		send: () => notSupported("Sharding.send"),
		sendOutgoing: () => notSupported("Sharding.sendOutgoing"),
		notify: () => notSupported("Sharding.notify"),
		reset: (requestId) => {
			const target = requestTargets.get(String(requestId));
			if (target === undefined) return Effect.succeed(false);
			return callActor({
				resolve: target.resolve,
				call: (stub) => stub.reset(target.storageRequestId),
				retry: boundedRetry,
			}).pipe(Effect.orDie, Effect.as(true));
		},
		pollStorage: notSupported("Sharding.pollStorage"),
		activeEntityCount: Effect.succeed(0),
	});

	const getWorkflowStub = (
		workflowName: string,
		executionId: string,
	): WorkflowStub =>
		makeWorkflowStub(
			rivetClient.getOrCreate(workflowActorName(workflowName), [
				executionId,
			]),
		);

	const workflowEngine = makeWorkflowEngine({
		workflows,
		getWorkflowStub,
		currentTimeMillis: () => clock.currentTimeMillisUnsafe(),
	});

	const runtime: ClusterRuntime = {
		entities,
		registry,
		singletons,
		workflows,
		deferredState,
		getWorkflowStub,
		host,
		heartbeatMillis,
		wakeSingleton,
		actionTimeoutMillis,
		markReady: () => ready.resolve(),
	};

	const queueStore = makePersistedQueueStore((name) =>
		makeQueueStub(rivetClient.getOrCreate(queueActorName, [name])),
	);
	const queueContext = yield* Layer.build(
		PersistedQueue.layer.pipe(
			Layer.provide(
				Layer.succeed(PersistedQueue.PersistedQueueStore)(queueStore),
			),
		),
	);

	return Context.make(Sharding, sharding).pipe(
		Context.add(WorkflowEngine.WorkflowEngine, workflowEngine),
		Context.add(
			PersistedQueue.PersistedQueueFactory,
			Context.get(queueContext, PersistedQueue.PersistedQueueFactory),
		),
		Context.add(RivetClusterRuntime, runtime),
	);
});

/**
 * Provides the cluster `Sharding` service backed by Rivet Actors.
 *
 * Use it in a process that only calls entities, such as an API server.
 * `Entity.client` resolves each entity's actor through the RivetKit client.
 * To host the entities as well, use {@link serve}.
 *
 * @since 3.1.2
 */
export const layer = (options: Options): Layer.Layer<ClusterServices> =>
	Layer.effectContext(make(options)) as Layer.Layer<ClusterServices>;

/**
 * The services {@link layer} and {@link serve} provide.
 *
 * @since 3.1.2
 */
export type ClusterServices =
	| Sharding
	| WorkflowEngine.WorkflowEngine
	| PersistedQueue.PersistedQueueFactory;

/**
 * Options for {@link serve}.
 *
 * @since 3.1.2
 */
export interface ServeOptions extends Options {
	/**
	 * RivetKit registry configuration, passed through to `setup`. `use` lists
	 * any other actors the node hosts beside the entity actors.
	 */
	readonly registry?: RegistryConfigInput<any> | undefined;
}

const entityActorDefinitions = (
	runtime: ClusterRuntime,
): Record<string, AnyActorDefinition> => {
	const actors: Record<string, AnyActorDefinition> = {};
	for (const entityType of runtime.entities.keys()) {
		const registration = runtime.registry.get(entityType);
		const maxIdleTime = (
			registration?.options as
				| { readonly maxIdleTime?: Duration.Input }
				| undefined
		)?.maxIdleTime;
		actors[entityType] = makeEntityActor(runtime.host, {
			actionTimeout: runtime.actionTimeoutMillis,
			sleepTimeout:
				maxIdleTime === undefined
					? undefined
					: Duration.toMillis(Duration.fromInputUnsafe(maxIdleTime)),
		});
	}
	return actors;
};

/**
 * Builds the entity handler layer and hosts its entities on this Rivet node.
 *
 * The handler layer is built first, so every `Entity.toLayer` registration
 * is complete before the registry starts. One actor definition is registered
 * per entity type, named after the type, and the RivetKit registry starts
 * with those definitions plus `options.registry.use`.
 *
 * @since 3.1.2
 */
export const serve = <E, R>(
	handlers: Layer.Layer<never, E, R>,
	options: ServeOptions,
): Layer.Layer<ClusterServices, E, Exclude<R, ClusterServices>> =>
	Layer.effectContext(
		Effect.gen(function* () {
			const clusterContext = yield* make(options);
			const runtime = Context.get(clusterContext, RivetClusterRuntime);
			yield* Layer.build(
				Layer.provide(handlers, Layer.succeedContext(clusterContext)),
			);
			const registryConfig = options.registry ?? { use: {} };
			const actors = entityActorDefinitions(runtime);
			for (const workflowName of runtime.workflows.names()) {
				actors[workflowActorName(workflowName)] = makeWorkflowActor(
					{
						registry: runtime.workflows,
						deferredState: runtime.deferredState,
						heartbeatMillis: runtime.heartbeatMillis,
						ready: runtime.host.ready,
						getStub: runtime.getWorkflowStub,
					},
					{ actionTimeout: runtime.actionTimeoutMillis },
				);
			}
			actors[queueActorName] = makeQueueActor({
				heartbeatMillis: runtime.heartbeatMillis,
				actionTimeout: runtime.actionTimeoutMillis,
			});
			actors[singletonActorName] = makeSingletonActor({
				registry: runtime.singletons,
				heartbeatMillis: runtime.heartbeatMillis,
				ready: runtime.host.ready,
			});
			for (const name of Object.keys(actors)) {
				if (textEncoder.encode(name).byteLength > maxActorNameBytes) {
					return yield* Effect.die(
						new Error(
							`RivetCluster: entity type "${name}" is longer than the ${maxActorNameBytes}-byte actor name limit`,
						),
					);
				}
				if (registryConfig.use?.[name] !== undefined) {
					return yield* Effect.die(
						new Error(
							`RivetCluster: actor name "${name}" is used by both an entity type and registry.use`,
						),
					);
				}
			}
			const rivetRegistry = setup({
				// Requests and replies of up to 2 MB cross the actor boundary.
				maxIncomingMessageSize:
					maximumEncodedSize + messageOverheadBytes,
				maxOutgoingMessageSize:
					maximumEncodedSize + messageOverheadBytes,
				...registryConfig,
				use: { ...registryConfig.use, ...actors },
			});
			runtime.markReady();
			rivetRegistry.start();
			// Every node wakes every singleton it registers. A wake is
			// idempotent, so this both starts singletons on the first deploy and
			// restarts any whose wake chain was lost.
			yield* Effect.forkIn(
				Effect.forEach(
					runtime.singletons.names(),
					(name) =>
						Effect.tryPromise(() =>
							runtime.wakeSingleton(name),
						).pipe(
							Effect.retry(singletonWakeRetry),
							Effect.catchCause((cause) =>
								Effect.logError("Failed to wake singleton", {
									name,
									cause,
								}),
							),
						),
					{ concurrency: "unbounded", discard: true },
				),
				yield* Effect.scope,
			);
			return Context.omit(RivetClusterRuntime)(clusterContext);
		}),
	) as any;

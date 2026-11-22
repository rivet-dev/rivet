/**
 * The Rivet Actor definition that hosts one Effect Cluster entity type.
 *
 * Each actor instance is one entity, keyed by its entity id. Its SQLite
 * database is the entity's mailbox, and RivetKit schedules provide its wake
 * time. The actions are internal transport for `RivetCluster` clients.
 *
 * @internal
 */
import * as EntityAddress from "effect/cluster/EntityAddress";
import * as EntityId from "effect/cluster/EntityId";
import * as EntityType from "effect/cluster/EntityType";
import * as ShardId from "effect/cluster/ShardId";
import * as Effect from "effect/Effect";
import { type AnyActorDefinition, actor, UserError } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";
import { encodeName } from "./clusterName.ts";
import { makeEntityKeepAlive } from "./entityKeepAlive.ts";
import type { EntityRegistry } from "./entityRegistry.ts";
import { type EntityManager, makeEntityManager } from "./entityRuntime.ts";
import { makeEntitySql } from "./entitySql.ts";
import { entityMigrations, loadEntityState } from "./entityStorage.ts";
import { makeActorSql } from "./sql.ts";
import {
	makeScheduleTimer,
	type TimerCron,
	type TimerKind,
	type TimerSchedule,
	timerAction,
} from "./timer.ts";
import type { EntityDeliveryOptions, EntityInvokeResult } from "./types.ts";

/** What an entity actor needs from the cluster that hosts it. */
export interface EntityActorHost {
	readonly registry: EntityRegistry;
	/** Interval of the heartbeat wake while keep-alive is held or a request runs. */
	readonly heartbeatMillis: number;
	/** Resolves once the Effect layer has registered every entity's handlers. */
	readonly ready: Promise<void>;
	/** Pushes a scheduled reply to the entity named by `replyTo`. */
	readonly deliverReply: (
		replyTo: string,
		requestId: string,
		reply: string,
	) => Promise<boolean>;
}

/** Per-type actor options derived from the entity registration. */
export interface EntityActorOptions {
	/** Idle time before the actor sleeps. Maps `Entity.toLayer`'s `maxIdleTime`. */
	readonly sleepTimeout?: number | undefined;
	/** Upper bound of one action, including an ask that waits for its reply. */
	readonly actionTimeout: number;
}

interface EntityActorVars {
	manager: EntityManager | undefined;
}

/** The narrow view of the RivetKit actor context the entity program uses. */
interface EntityActorContext {
	readonly name: string;
	readonly key: ReadonlyArray<string>;
	readonly db: RawAccess;
	readonly schedule: TimerSchedule;
	readonly cron: TimerCron;
	readonly vars: EntityActorVars;
	readonly abortSignal: AbortSignal;
	keepAwake<T>(promise: Promise<T>): Promise<T>;
}

const makeEntityProgram = (c: EntityActorContext, host: EntityActorHost) =>
	Effect.gen(function* () {
		const sql = makeActorSql(c.db);
		const timer = makeScheduleTimer({
			schedule: c.schedule,
			cron: c.cron,
			heartbeatMillis: host.heartbeatMillis,
		});
		const stored = yield* loadEntityState(sql);
		const entityType = c.name;
		const entityId = c.key[0];
		if (entityId === undefined) {
			return yield* Effect.die(
				new Error(
					"Entity actor requires its entity id as the actor key",
				),
			);
		}
		const entityName = encodeName(entityType, entityId);

		// `persist` runs only after `update` is called, by which time `manager`
		// is assigned.
		let manager: EntityManager | undefined;
		const keepAlive = makeEntityKeepAlive({
			startHold: () => c.keepAwake(Effect.runPromise(keepAlive.await)),
			wanted: stored.keepAlive,
			persist: (wanted) =>
				manager === undefined
					? Effect.die(
							new Error("Entity keep-alive used before startup"),
						)
					: manager.saveKeepAlive(wanted),
			retryCapMillis: () => host.heartbeatMillis,
		});
		const built = makeEntityManager({
			sql,
			timer,
			entitySql: makeEntitySql(c.db),
			registry: host.registry,
			address: EntityAddress.make({
				shardId: ShardId.make("default", 1),
				entityType: EntityType.make(entityType),
				entityId: EntityId.make(entityId),
			}),
			entityName,
			keepAlive,
			heartbeatMillis: host.heartbeatMillis,
			keepAwake: (effect) => {
				c.keepAwake(Effect.runPromise(effect)).catch(() => undefined);
			},
			deliverReply: host.deliverReply,
		});
		manager = built;

		// Interrupt handlers as soon as shutdown starts, so none outlives the
		// actor while a replacement replays its rows.
		c.abortSignal.addEventListener(
			"abort",
			() => {
				Effect.runFork(built.shutdown);
			},
			{ once: true },
		);
		return built;
	});

/**
 * The error code a stopping entity actor returns. Clients retry it, and the
 * mailbox deduplicates the retry by request id.
 */
export const entityStoppingCode = "cluster_entity_stopping";

const stoppingError = () =>
	new UserError("Entity actor is stopping", { code: entityStoppingCode });

/**
 * Runs an entity action. A failure while the actor is stopping, such as an
 * ask whose handler shutdown interrupted, becomes a retryable error.
 */
const runAction = <A>(
	manager: EntityManager,
	effect: Effect.Effect<A>,
): Promise<A> => {
	if (manager.isStopping()) return Promise.reject(stoppingError());
	return Effect.runPromise(effect).catch((error) => {
		throw manager.isStopping() ? stoppingError() : error;
	});
};

const getManager = (c: { readonly vars: EntityActorVars }): EntityManager => {
	const manager = c.vars.manager;
	if (manager === undefined) {
		throw new Error("Entity actor received a request before it started");
	}
	return manager;
};

/** Builds the actor definition that hosts one entity type. */
export const makeEntityActor = (
	host: EntityActorHost,
	options: EntityActorOptions,
): AnyActorDefinition =>
	actor({
		db: db({
			onMigrate: async (raw) => {
				for (const statement of entityMigrations) {
					await raw.execute(statement);
				}
			},
		}),
		createVars: (): EntityActorVars => ({ manager: undefined }),
		onWake: async (c) => {
			await host.ready;
			const manager = await Effect.runPromise(
				makeEntityProgram(c as unknown as EntityActorContext, host),
			);
			c.vars.manager = manager;
			// Run the timer pass at once instead of waiting for a scheduled wake:
			// it replays rows a previous run left unprocessed, rebuilds a
			// kept-alive entity, and re-arms the wakes. It reads only a few rows
			// for an idle entity.
			c.keepAwake(Effect.runPromise(manager.timer("due"))).catch(
				(error) => {
					c.log.error({ msg: "entity wake replay failed", error });
				},
			);
		},
		onSleep: async (c) => {
			const manager = c.vars.manager;
			if (manager === undefined) return;
			await Effect.runPromise(manager.prepareSleep);
		},
		actions: {
			invoke: (
				c,
				envelope: string,
				discard: boolean,
				delivery: EntityDeliveryOptions | null,
			): Promise<EntityInvokeResult> =>
				runAction(
					getManager(c),
					getManager(c).invoke(
						envelope,
						discard,
						delivery ?? undefined,
					),
				),
			acknowledge: (
				c,
				requestId: string,
				replyId: string,
			): Promise<ReadonlyArray<string>> =>
				runAction(
					getManager(c),
					getManager(c).acknowledge(requestId, replyId),
				),
			interrupt: (
				c,
				storageRequestId: string,
				clientRequestId: string | null,
			): Promise<void> =>
				Effect.runPromise(
					getManager(c).interrupt(
						storageRequestId,
						clientRequestId ?? storageRequestId,
					),
				),
			reset: (c, requestId: string): Promise<void> =>
				Effect.runPromise(getManager(c).reset(requestId)),
			deliverReply: (
				c,
				requestId: string,
				reply: string,
			): Promise<boolean> =>
				Effect.runPromise(getManager(c).deliverReply(requestId, reply)),
			[timerAction]: (c, kind: TimerKind): Promise<void> =>
				Effect.runPromise(getManager(c).timer(kind)),
		},
		options: {
			actionTimeout: options.actionTimeout,
			...(options.sleepTimeout === undefined
				? {}
				: { sleepTimeout: options.sleepTimeout }),
		},
	}) as unknown as AnyActorDefinition;

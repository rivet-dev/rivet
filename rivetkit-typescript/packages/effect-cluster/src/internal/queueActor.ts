/**
 * The Rivet Actor behind one persisted queue, one actor per queue name.
 *
 * Items, their attempt counts, and their in-flight leases live in the
 * actor's SQLite. A taker's lease is refreshed while its handler runs, so an
 * item whose taker died is redelivered once its lease expires; leasing counts
 * an attempt, and an expired lease on the last attempt fails the item. A
 * `take` with no available item waits inside the actor for a bounded time and
 * then returns nothing, and the client takes again with the same taker id.
 *
 * Portions adapted from Effect (MIT License,
 * Copyright (c) 2023 Effectful Technologies Inc). See `LICENSE-effect`.
 *
 * @internal
 */
import * as Effect from "effect/Effect";
import * as Semaphore from "effect/Semaphore";
import { type AnyActorDefinition, actor } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";
import { armTimer } from "./entityStorage.ts";
import {
	completeItem,
	earliestLeaseExpiry,
	expireLeases,
	extendLease,
	failItem,
	leasedItem,
	leaseNextItem,
	offerItem,
	type QueueItem,
	queueMigrations,
	releaseItem,
	releaseOwnedItem,
} from "./queueStorage.ts";
import { type ActorSql, makeActorSql } from "./sql.ts";
import {
	type ActorTimer,
	makeScheduleTimer,
	type TimerCron,
	type TimerKind,
	type TimerSchedule,
	timerAction,
} from "./timer.ts";

/** The actor name that hosts every persisted queue. */
export const queueActorName = "effect.queue";

/** How long one `take` waits for an item before returning nothing. */
export const takeWaitMillis = 25_000;

interface Waiter {
	readonly takerId: string;
	readonly maxAttempts: number;
	readonly leaseMillis: number;
	resolve: (item: QueueItem) => void;
}

interface QueueRuntime {
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
	readonly runTimer: () => Promise<void>;
}

const makeQueueRuntime = (sql: ActorSql, timer: ActorTimer): QueueRuntime => {
	// Leasing reads and then writes rows and the waiter list, so every
	// operation runs under one permit.
	const entry = Semaphore.makeUnsafe(1);
	const run = <A>(effect: Effect.Effect<A>): Promise<A> =>
		Effect.runPromise(Semaphore.withPermit(entry, effect));
	const waiters: Array<Waiter> = [];

	// Waiters differ in maxAttempts, so one waiter finding nothing does not
	// mean a later one will; every waiter gets its own lease attempt.
	const wakeWaiters = Effect.gen(function* () {
		const now = Date.now();
		let index = 0;
		while (index < waiters.length) {
			const waiter = waiters[index];
			const item = yield* leaseNextItem(
				sql,
				now,
				now + waiter.leaseMillis,
				waiter.maxAttempts,
				waiter.takerId,
			);
			if (item === undefined) {
				index++;
				continue;
			}
			waiters.splice(index, 1);
			waiter.resolve(item);
		}
		// Include initial visibility deadlines even when no taker is waiting.
		const earliest = yield* earliestLeaseExpiry(sql);
		if (earliest !== undefined) yield* armTimer(timer, earliest);
	});

	const take = async (
		takerId: string,
		maxAttempts: number,
		leaseMillis: number,
	): Promise<QueueItem | null> => {
		let resolveItem: (item: QueueItem) => void = () => undefined;
		const pending = new Promise<QueueItem>((resolve) => {
			resolveItem = resolve;
		});
		const leased = await run(
			Effect.gen(function* () {
				// A retried take keeps the item already leased to its taker id.
				const existingLease = yield* leasedItem(
					sql,
					takerId,
					Date.now(),
				);
				if (existingLease !== undefined) return existingLease;
				const existing = waiters.find(
					(waiter) => waiter.takerId === takerId,
				);
				if (existing === undefined) {
					waiters.push({
						takerId,
						maxAttempts,
						leaseMillis,
						resolve: resolveItem,
					});
				} else {
					existing.resolve = resolveItem;
				}
				yield* wakeWaiters;
				return undefined;
			}),
		);
		if (leased !== undefined) return leased;

		let waitTimer: ReturnType<typeof setTimeout> | undefined;
		const timedOut = new Promise<"timeout">((resolve) => {
			waitTimer = setTimeout(() => resolve("timeout"), takeWaitMillis);
		});
		const outcome = await Promise.race([pending, timedOut]);
		clearTimeout(waitTimer);
		if (outcome !== "timeout") return outcome;
		// Stop waiting. A waiter that was resolved in the meantime still hands
		// its item back, because the lease now belongs to this taker.
		const removed = await run(
			Effect.sync(() => {
				const index = waiters.findIndex(
					(waiter) =>
						waiter.takerId === takerId &&
						waiter.resolve === resolveItem,
				);
				if (index < 0) return false;
				waiters.splice(index, 1);
				return true;
			}),
		);
		return removed ? null : pending;
	};

	return {
		offer: (id, element, delayMillis, deduplicate) =>
			run(
				Effect.andThen(
					offerItem(
						sql,
						id,
						element,
						delayMillis > 0 ? Date.now() + delayMillis : null,
						deduplicate,
					),
					wakeWaiters,
				),
			),
		take,
		cancelTake: (takerId) =>
			run(
				Effect.gen(function* () {
					const index = waiters.findIndex(
						(waiter) => waiter.takerId === takerId,
					);
					if (index >= 0) {
						waiters.splice(index, 1);
						return;
					}
					// The cancel can race the delivery; release an item already
					// leased to the taker instead of stranding it.
					yield* releaseOwnedItem(sql, takerId, Date.now());
					yield* wakeWaiters;
				}),
			),
		complete: (id, takerId) => run(completeItem(sql, id, takerId)),
		fail: (id, lastFailure, takerId, terminal, retryDelayMillis) =>
			run(
				Effect.andThen(
					failItem(
						sql,
						id,
						lastFailure,
						takerId,
						terminal,
						retryDelayMillis > 0
							? Date.now() + retryDelayMillis
							: null,
					),
					wakeWaiters,
				),
			),
		release: (id, takerId) =>
			run(Effect.andThen(releaseItem(sql, id, takerId), wakeWaiters)),
		extend: (id, leaseMillis, takerId) =>
			run(extendLease(sql, id, Date.now() + leaseMillis, takerId)),
		runTimer: () =>
			run(
				Effect.gen(function* () {
					yield* timer.fired;
					yield* expireLeases(sql, Date.now());
					yield* wakeWaiters;
				}),
			),
	};
};

interface QueueVars {
	runtime: QueueRuntime | undefined;
}

interface QueueContext {
	readonly db: RawAccess;
	readonly schedule: TimerSchedule;
	readonly cron: TimerCron;
	readonly vars: QueueVars;
}

const getRuntime = (c: { readonly vars: QueueVars }): QueueRuntime => {
	const runtime = c.vars.runtime;
	if (runtime === undefined) {
		throw new Error("Queue actor received a request before it started");
	}
	return runtime;
};

/** Builds the actor definition that hosts every persisted queue. */
export const makeQueueActor = (options: {
	readonly heartbeatMillis: number;
	readonly actionTimeout: number;
}): AnyActorDefinition =>
	actor({
		db: db({
			onMigrate: async (raw) => {
				for (const statement of queueMigrations)
					await raw.execute(statement);
			},
		}),
		createVars: (): QueueVars => ({ runtime: undefined }),
		onWake: async (rawContext) => {
			const c = rawContext as unknown as QueueContext;
			const timer = makeScheduleTimer({
				schedule: c.schedule,
				cron: c.cron,
				heartbeatMillis: options.heartbeatMillis,
			});
			c.vars.runtime = makeQueueRuntime(makeActorSql(c.db), timer);
		},
		actions: {
			offer: (
				c,
				id: string,
				element: string,
				delayMillis: number,
				deduplicate: "untilExecuted" | "untilRemoved" | null,
			): Promise<void> =>
				getRuntime(c).offer(
					id,
					element,
					delayMillis,
					deduplicate ?? undefined,
				),
			take: (
				c,
				takerId: string,
				maxAttempts: number,
				leaseMillis: number,
			): Promise<QueueItem | null> =>
				getRuntime(c).take(takerId, maxAttempts, leaseMillis),
			cancelTake: (c, takerId: string): Promise<void> =>
				getRuntime(c).cancelTake(takerId),
			complete: (c, id: string, takerId: string): Promise<void> =>
				getRuntime(c).complete(id, takerId),
			fail: (
				c,
				id: string,
				lastFailure: string,
				takerId: string,
				terminal: boolean,
				retryDelayMillis: number,
			): Promise<void> =>
				getRuntime(c).fail(
					id,
					lastFailure,
					takerId,
					terminal,
					retryDelayMillis,
				),
			release: (c, id: string, takerId: string): Promise<void> =>
				getRuntime(c).release(id, takerId),
			extend: (
				c,
				id: string,
				leaseMillis: number,
				takerId: string,
			): Promise<void> => getRuntime(c).extend(id, leaseMillis, takerId),
			[timerAction]: (c, _kind: TimerKind): Promise<void> =>
				getRuntime(c).runTimer(),
		},
		options: { actionTimeout: options.actionTimeout },
	}) as unknown as AnyActorDefinition;

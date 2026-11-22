/**
 * Wake scheduling for cluster actors, built on RivetKit schedules.
 *
 * SQLite remains the source of truth for why an actor must wake; schedules
 * are only hints, and every wake recomputes them from storage. Two kinds of
 * wake exist:
 *
 * - A due time, such as the earliest `DeliverAt`. RivetKit deletes a one-shot
 *   row before it dispatches the action, so a process that dies in between
 *   loses that fire. `set` therefore writes a guard row one heartbeat after
 *   the due row, and inserts new rows before cancelling old ones, so no crash
 *   leaves the actor without a row.
 * - A heartbeat while the actor holds keep-alive or runs persisted work. It is
 *   a recurring `c.cron.every` job, which RivetKit advances instead of
 *   deleting on each fire, so a lost fire is retried one interval later.
 *
 * A fire that finds nothing to do recomputes the schedules, which is
 * harmless.
 *
 * @internal
 */
import * as Effect from "effect/Effect";
import * as Semaphore from "effect/Semaphore";

/** @internal */
export interface ActorTimer {
	/** The due time of the armed one-shot wake, or `null` when none is armed. */
	readonly get: Effect.Effect<number | null>;
	/** Replaces the one-shot wake with one due at `at` (epoch milliseconds). */
	readonly set: (at: number) => Effect.Effect<void>;
	/** Removes the one-shot wake and its guard. */
	readonly delete: Effect.Effect<void>;
	/** Forgets cached schedule state after a fire, which RivetKit may have changed. */
	readonly fired: Effect.Effect<void>;
	/** Starts the recurring heartbeat unless it already runs. */
	readonly armHeartbeat: Effect.Effect<void>;
	/** Stops the recurring heartbeat. */
	readonly disarmHeartbeat: Effect.Effect<void>;
}

/** The subset of `c.schedule` the timer uses. */
export interface TimerSchedule {
	at(timestamp: number, action: string, ...args: unknown[]): Promise<string>;
	cancel(id: string): Promise<boolean>;
	list(): Promise<
		ReadonlyArray<{
			readonly id: string;
			readonly action: string;
			readonly args: ReadonlyArray<unknown>;
			readonly runAt: number;
		}>
	>;
}

/** The subset of `c.cron` the timer uses. */
export interface TimerCron {
	every(options: {
		readonly name: string;
		readonly interval: number;
		readonly action: string;
		readonly args?: Array<unknown>;
		readonly maxHistory?: number;
	}): Promise<void>;
	get(name: string): Promise<unknown>;
	delete(name: string): Promise<boolean>;
}

/** The action every cluster actor exposes for its wakes. */
export const timerAction = "clusterTimer";

/** Kinds passed as the timer action's first argument. */
export type TimerKind = "due" | "guard" | "heartbeat";

const heartbeatJob = "cluster-heartbeat";

/** RivetKit rejects recurring jobs with a shorter interval. */
export const minimumHeartbeatMillis = 5_000;

interface LoadedDue {
	readonly dueAt: number | null;
	readonly ids: ReadonlyArray<string>;
}

/** @internal */
export const makeScheduleTimer = (options: {
	readonly schedule: TimerSchedule;
	readonly cron: TimerCron;
	readonly heartbeatMillis: number;
}): ActorTimer => {
	const { schedule, cron } = options;
	const permit = Semaphore.makeUnsafe(1);
	let loaded: LoadedDue | undefined;
	let heartbeatArmed: boolean | undefined;

	const load: Effect.Effect<LoadedDue> = Effect.suspend(() => {
		if (loaded !== undefined) return Effect.succeed(loaded);
		return Effect.map(
			Effect.promise(() => schedule.list()),
			(events) => {
				let dueAt: number | null = null;
				const ids: Array<string> = [];
				for (const event of events) {
					if (event.action !== timerAction) continue;
					const kind = event.args[0];
					if (kind !== "due" && kind !== "guard") continue;
					ids.push(event.id);
					if (kind !== "due") continue;
					if (dueAt === null || event.runAt < dueAt)
						dueAt = event.runAt;
				}
				const result: LoadedDue = { dueAt, ids };
				loaded = result;
				return result;
			},
		);
	});

	const cancelAll = (ids: ReadonlyArray<string>): Effect.Effect<void> =>
		Effect.forEach(ids, (id) => Effect.promise(() => schedule.cancel(id)), {
			discard: true,
		});

	const set = (at: number): Effect.Effect<void> =>
		Semaphore.withPermit(
			permit,
			Effect.gen(function* () {
				const current = yield* load;
				// Clear the cache first so a failure below forces a reload.
				loaded = undefined;
				const dueId = yield* Effect.promise(() =>
					schedule.at(at, timerAction, "due" satisfies TimerKind),
				);
				const guardId = yield* Effect.promise(() =>
					schedule.at(
						at + options.heartbeatMillis,
						timerAction,
						"guard" satisfies TimerKind,
					),
				);
				yield* cancelAll(current.ids);
				loaded = { dueAt: at, ids: [dueId, guardId] };
			}),
		);

	const remove: Effect.Effect<void> = Semaphore.withPermit(
		permit,
		Effect.gen(function* () {
			const current = yield* load;
			if (current.ids.length === 0) return;
			loaded = undefined;
			yield* cancelAll(current.ids);
			loaded = { dueAt: null, ids: [] };
		}),
	);

	const isHeartbeatArmed: Effect.Effect<boolean> = Effect.suspend(() => {
		if (heartbeatArmed !== undefined) return Effect.succeed(heartbeatArmed);
		return Effect.map(
			Effect.promise(() => cron.get(heartbeatJob)),
			(job) => {
				heartbeatArmed = job !== undefined;
				return heartbeatArmed;
			},
		);
	});

	const armHeartbeat: Effect.Effect<void> = Semaphore.withPermit(
		permit,
		Effect.gen(function* () {
			if (yield* isHeartbeatArmed) return;
			yield* Effect.promise(() =>
				cron.every({
					name: heartbeatJob,
					interval: options.heartbeatMillis,
					action: timerAction,
					args: ["heartbeat" satisfies TimerKind],
					maxHistory: 0,
				}),
			);
			heartbeatArmed = true;
		}),
	);

	const disarmHeartbeat: Effect.Effect<void> = Semaphore.withPermit(
		permit,
		Effect.gen(function* () {
			if (!(yield* isHeartbeatArmed)) return;
			yield* Effect.promise(() => cron.delete(heartbeatJob));
			heartbeatArmed = false;
		}),
	);

	return {
		get: Semaphore.withPermit(
			permit,
			Effect.map(load, (current) => current.dueAt),
		),
		set,
		delete: remove,
		fired: Semaphore.withPermit(
			permit,
			Effect.sync(() => {
				loaded = undefined;
			}),
		),
		armHeartbeat,
		disarmHeartbeat,
	};
};

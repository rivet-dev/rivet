/**
 * The Rivet Actor that runs cluster singletons, one actor per singleton name.
 *
 * Rivet runs at most one instance of an actor key, so a singleton runs at
 * most once. A `wake` starts the registered effect and records that it should
 * run. While it runs the actor stays awake and keeps a recurring heartbeat,
 * so a crash or a lost node restarts it within one heartbeat. When the actor
 * stops, for example when its node drains, the effect is interrupted and an
 * immediate wake brings it back on the next node. An effect that fails is
 * restarted by the next heartbeat; one that succeeds stays idle until the
 * next `wake`.
 *
 * @internal
 */
import * as Cause from "effect/Cause";
import * as Effect from "effect/Effect";
import * as Exit from "effect/Exit";
import * as Fiber from "effect/Fiber";
import { type AnyActorDefinition, actor } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";
import type { SingletonRegistry } from "./singletonRegistry.ts";
import {
	type ActorTimer,
	makeScheduleTimer,
	type TimerCron,
	type TimerSchedule,
	timerAction,
} from "./timer.ts";

/** The actor name that hosts every singleton. */
export const singletonActorName = "effect.singleton";

/** What a singleton actor needs from the cluster that hosts it. */
export interface SingletonActorHost {
	readonly registry: SingletonRegistry;
	readonly heartbeatMillis: number;
	/** Resolves once the Effect layer has registered every singleton. */
	readonly ready: Promise<void>;
}

/** Result of the `wake` action. */
export type SingletonWakeResult = "started" | "running" | "unregistered";

interface SingletonVars {
	timer: ActorTimer | undefined;
	fiber: Fiber.Fiber<void, unknown> | undefined;
}

interface SingletonContext {
	readonly key: ReadonlyArray<string>;
	readonly db: RawAccess;
	readonly schedule: TimerSchedule;
	readonly cron: TimerCron;
	readonly vars: SingletonVars;
	readonly abortSignal: AbortSignal;
	readonly log: { error(fields: Record<string, unknown>): void };
	keepAwake<T>(promise: Promise<T>): Promise<T>;
}

type StateRow = {
	readonly wanted: number;
};

const migrations = [
	`CREATE TABLE IF NOT EXISTS singleton_state (
    id INTEGER PRIMARY KEY CHECK (id = 0),
    wanted INTEGER NOT NULL DEFAULT 0
  )`,
];

const isWanted = async (raw: RawAccess): Promise<boolean> => {
	const rows = await raw.execute<StateRow>(
		"SELECT wanted FROM singleton_state WHERE id = 0",
	);
	return rows[0]?.wanted === 1;
};

const saveWanted = async (raw: RawAccess, wanted: boolean): Promise<void> => {
	await raw.execute(
		`INSERT INTO singleton_state (id, wanted) VALUES (0, ?)
     ON CONFLICT (id) DO UPDATE SET wanted = excluded.wanted`,
		wanted ? 1 : 0,
	);
};

const getTimer = (c: SingletonContext): ActorTimer => {
	const timer = c.vars.timer;
	if (timer === undefined) {
		throw new Error("Singleton actor received a request before it started");
	}
	return timer;
};

/** Starts the registered effect unless it is already running. */
const start = (
	c: SingletonContext,
	host: SingletonActorHost,
): SingletonWakeResult => {
	if (c.vars.fiber !== undefined) return "running";
	const name = c.key[0];
	const registration =
		name === undefined ? undefined : host.registry.get(name);
	if (registration === undefined) return "unregistered";

	const fiber = Effect.runFork(
		registration.run.pipe(
			Effect.scoped,
			Effect.provideContext(registration.context),
		),
	);
	c.vars.fiber = fiber;
	const timer = getTimer(c);
	const supervise = Effect.gen(function* () {
		yield* timer.armHeartbeat;
		const exit = yield* Fiber.await(fiber);
		if (c.vars.fiber === fiber) c.vars.fiber = undefined;
		if (Exit.isSuccess(exit)) {
			// A finished singleton keeps no wake of its own.
			yield* timer.disarmHeartbeat;
			return;
		}
		if (Cause.hasInterruptsOnly(exit.cause)) return;
		// The heartbeat stays armed, so its next fire restarts the effect.
		yield* Effect.logError(
			"Singleton failed, restarting on the next heartbeat",
			{
				singleton: name,
				cause: exit.cause,
			},
		);
	});
	c.keepAwake(Effect.runPromise(supervise)).catch((error) => {
		c.log.error({
			msg: "singleton supervision failed",
			singleton: name,
			error,
		});
	});
	return "started";
};

const stop = (c: SingletonContext): Effect.Effect<void> =>
	Effect.suspend(() => {
		const fiber = c.vars.fiber;
		if (fiber === undefined) return Effect.void;
		c.vars.fiber = undefined;
		return Effect.asVoid(Fiber.interrupt(fiber));
	});

/** Builds the actor definition that hosts every singleton. */
export const makeSingletonActor = (
	host: SingletonActorHost,
): AnyActorDefinition =>
	actor({
		db: db({
			onMigrate: async (raw) => {
				for (const statement of migrations)
					await raw.execute(statement);
			},
		}),
		createVars: (): SingletonVars => ({
			timer: undefined,
			fiber: undefined,
		}),
		onWake: async (rawContext) => {
			const c = rawContext as unknown as SingletonContext;
			await host.ready;
			c.vars.timer = makeScheduleTimer({
				schedule: c.schedule,
				cron: c.cron,
				heartbeatMillis: host.heartbeatMillis,
			});
			// Stop the effect as soon as shutdown starts, so it never runs
			// beside its replacement on another node.
			c.abortSignal.addEventListener(
				"abort",
				() => {
					Effect.runFork(stop(c));
				},
				{ once: true },
			);
			if (await isWanted(c.db)) start(c, host);
		},
		onSleep: async (rawContext) => {
			const c = rawContext as unknown as SingletonContext;
			const wasRunning = c.vars.fiber !== undefined;
			await Effect.runPromise(stop(c));
			if (!wasRunning) return;
			// The actor is stopping, so this wake lands on its next start.
			await Effect.runPromise(getTimer(c).set(Date.now() + 1_000));
		},
		actions: {
			wake: async (rawContext): Promise<SingletonWakeResult> => {
				const c = rawContext as unknown as SingletonContext;
				await saveWanted(c.db, true);
				return start(c, host);
			},
			[timerAction]: async (rawContext): Promise<void> => {
				const c = rawContext as unknown as SingletonContext;
				const timer = getTimer(c);
				await Effect.runPromise(timer.fired);
				if (!(await isWanted(c.db))) {
					await Effect.runPromise(timer.disarmHeartbeat);
					return;
				}
				start(c, host);
			},
		},
	}) as unknown as AnyActorDefinition;

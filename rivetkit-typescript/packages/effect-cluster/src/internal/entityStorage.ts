/**
 * Entity actor storage: mailbox schema, the persisted keep-alive flag, and
 * the computation of the actor's next wake time.
 *
 * Portions adapted from Effect (MIT License,
 * Copyright (c) 2023 Effectful Technologies Inc). See `LICENSE-effect`.
 *
 * @internal
 */
import * as Effect from "effect/Effect";
import type { ActorSql } from "./sql.ts";
import type { ActorTimer } from "./timer.ts";

const entityDdl = [
	`CREATE TABLE IF NOT EXISTS cluster_messages (
    request_id TEXT PRIMARY KEY,
    message_id TEXT UNIQUE,
    envelope TEXT NOT NULL,
    discard INTEGER NOT NULL DEFAULT 0,
    processed INTEGER NOT NULL DEFAULT 0,
    last_reply_id TEXT,
    deliver_at INTEGER,
    reply_to TEXT
  )`,
	`CREATE TABLE IF NOT EXISTS cluster_replies (
    reply_id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL,
    reply TEXT NOT NULL,
    kind TEXT NOT NULL,
    sequence INTEGER,
    acked INTEGER NOT NULL DEFAULT 0,
    UNIQUE (request_id, sequence)
  )`,
	`CREATE INDEX IF NOT EXISTS cluster_messages_deliver_at_idx
    ON cluster_messages (processed, deliver_at)`,
	`CREATE INDEX IF NOT EXISTS cluster_replies_unacked_idx
    ON cluster_replies (request_id) WHERE kind = 'Chunk' AND acked = 0`,
	// One row recording whether the entity wants keep-alive across restarts.
	`CREATE TABLE IF NOT EXISTS cluster_entity_state (
    id INTEGER PRIMARY KEY CHECK (id = 0),
    keep_alive INTEGER NOT NULL DEFAULT 0,
    name TEXT
  )`,
];

/** Statements that create the entity mailbox tables. Run from `onMigrate`. */
export const entityMigrations: ReadonlyArray<string> = entityDdl;

type DeliverAtRow = {
	readonly deliver_at: number | null;
};

/** @internal */
export const earliestDeliverAt = (
	sql: ActorSql,
): Effect.Effect<number | undefined> =>
	Effect.map(
		sql.exec<DeliverAtRow>(
			"SELECT min(deliver_at) AS deliver_at FROM cluster_messages WHERE processed = 0 AND deliver_at IS NOT NULL",
		),
		(rows) => rows[0]?.deliver_at ?? undefined,
	);

/** Moves the timer earlier to `at`. It never moves an armed timer later. */
export const armTimer = (timer: ActorTimer, at: number): Effect.Effect<void> =>
	Effect.flatMap(timer.get, (current) =>
		current === null || current > at ? timer.set(at) : Effect.void,
	);

type EntityStateRow = {
	readonly keep_alive: number;
	readonly name: string | null;
};

/** @internal */
export interface EntityState {
	readonly keepAlive: boolean;
}

/** @internal */
export const loadEntityState = (sql: ActorSql): Effect.Effect<EntityState> =>
	Effect.map(
		sql.exec<EntityStateRow>(
			"SELECT keep_alive, name FROM cluster_entity_state WHERE id = 0",
		),
		(rows) => ({ keepAlive: rows[0]?.keep_alive === 1 }),
	);

/** @internal */
export const saveKeepAlive = (
	sql: ActorSql,
	enabled: boolean,
): Effect.Effect<void> =>
	Effect.asVoid(
		sql.exec(
			`INSERT INTO cluster_entity_state (id, keep_alive) VALUES (0, ?)
       ON CONFLICT (id) DO UPDATE SET keep_alive = excluded.keep_alive`,
			enabled ? 1 : 0,
		),
	);

type UnprocessedRow = {
	readonly request_id: string;
	readonly deliver_at: number | null;
};

/** @internal */
export interface WakePlan {
	/** When the earliest row not running in this wake must be delivered. */
	readonly dueAt: number | undefined;
	/** Whether keep-alive is held or a persisted request is running. */
	readonly heartbeat: boolean;
}

/**
 * Computes what the actor must wake for from the whole entity state: due and
 * future rows that are not running in this wake, and a heartbeat while
 * keep-alive is held or a persisted request is running.
 */
export const planWake = (
	sql: ActorSql,
	options: {
		readonly now: number;
		readonly isRunning: (requestId: string) => boolean;
	},
): Effect.Effect<WakePlan> =>
	Effect.gen(function* () {
		let dueAt = Number.POSITIVE_INFINITY;
		let heartbeat = (yield* loadEntityState(sql)).keepAlive;
		const rows = yield* sql.exec<UnprocessedRow>(
			"SELECT request_id, deliver_at FROM cluster_messages WHERE processed = 0",
		);
		for (const row of rows) {
			if (options.isRunning(row.request_id)) {
				heartbeat = true;
				continue;
			}
			const rowDueAt =
				row.deliver_at === null
					? options.now
					: Math.max(row.deliver_at, options.now);
			dueAt = Math.min(dueAt, rowDueAt);
		}
		return {
			dueAt: Number.isFinite(dueAt) ? dueAt : undefined,
			heartbeat,
		};
	});

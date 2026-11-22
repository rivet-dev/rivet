/**
 * Durable queue actor storage: queue items, their attempt counts, and their
 * in-flight leases.
 *
 * Portions adapted from Effect (MIT License,
 * Copyright (c) 2023 Effectful Technologies Inc). See `LICENSE-effect`.
 *
 * @internal
 */
import * as Effect from "effect/Effect";
import type { ActorSql } from "./sql.ts";

/** Statements that create the queue tables. Run from `onMigrate`. */
export const queueMigrations: ReadonlyArray<string> = [
	`CREATE TABLE IF NOT EXISTS queue_items (
    id TEXT PRIMARY KEY,
    element TEXT NOT NULL,
    position INTEGER NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    completed INTEGER NOT NULL DEFAULT 0,
    lease_until INTEGER,
    lease_owner TEXT,
    max_attempts INTEGER,
    last_failure TEXT
  )`,
	`CREATE INDEX IF NOT EXISTS queue_items_take_idx
    ON queue_items (completed, position)`,
	`CREATE INDEX IF NOT EXISTS queue_items_lease_idx
    ON queue_items (lease_until)`,
	// MAX(position) needs a bare position index; completed rows are retained
	// for deduplication, so the (completed, position) index cannot serve it.
	`CREATE INDEX IF NOT EXISTS queue_items_position_idx
    ON queue_items (position)`,
	// A retried take and a cancel look the taker's row up by owner.
	`CREATE INDEX IF NOT EXISTS queue_items_owner_idx
    ON queue_items (lease_owner)`,
];

/** An item leased to a taker. `attempts` counts the attempts before this one. */
export interface QueueItem {
	readonly id: string;
	readonly element: string;
	readonly attempts: number;
}

type LeaseExpiryRow = {
	readonly lease_until: number | null;
};

/** @internal */
export const offerItem = (
	sql: ActorSql,
	id: string,
	element: string,
	visibleAt: number | null,
	deduplicate?: "untilExecuted" | "untilRemoved" | undefined,
): Effect.Effect<void> =>
	sql.transaction(
		Effect.gen(function* () {
			if (deduplicate === "untilExecuted") {
				// A completed or failed row releases the id, so the insert below
				// stores the offer as new work after every existing row.
				yield* sql.exec(
					"DELETE FROM queue_items WHERE id = ? AND completed <> 0",
					id,
				);
			}
			// Reuse lease_until for initial visibility. INSERT OR IGNORE keeps the
			// original deadline and payload when an id is offered again.
			yield* sql.exec(
				`INSERT OR IGNORE INTO queue_items (id, element, position, lease_until)
       VALUES (?, ?, (SELECT IFNULL(MAX(position), 0) + 1 FROM queue_items), ?)`,
				id,
				element,
				visibleAt,
			);
		}),
	);

/** @internal */
export const leaseNextItem = (
	sql: ActorSql,
	now: number,
	leaseUntil: number,
	maxAttempts: number,
	owner: string,
): Effect.Effect<QueueItem | undefined> =>
	Effect.gen(function* () {
		const row = (yield* sql.exec<QueueItem>(
			`SELECT id, element, attempts FROM queue_items
       WHERE completed = 0 AND attempts < ? AND (lease_until IS NULL OR lease_until <= ?)
       ORDER BY position ASC LIMIT 1`,
			maxAttempts,
			now,
		))[0];
		if (row === undefined) return undefined;
		yield* sql.exec(
			`UPDATE queue_items SET attempts = attempts + 1, lease_until = ?, lease_owner = ?, max_attempts = ?
       WHERE id = ?`,
			leaseUntil,
			owner,
			maxAttempts,
			row.id,
		);
		return row;
	});

/** @internal */
export const leasedItem = (
	sql: ActorSql,
	owner: string,
	now: number,
): Effect.Effect<QueueItem | undefined> =>
	Effect.map(
		sql.exec<QueueItem>(
			`SELECT id, element, attempts - 1 AS attempts FROM queue_items
       WHERE lease_owner = ? AND completed = 0 AND lease_until > ?
       LIMIT 1`,
			owner,
			now,
		),
		(rows) => rows[0],
	);

/** @internal */
export const completeItem = (
	sql: ActorSql,
	id: string,
	owner: string,
): Effect.Effect<void> =>
	Effect.asVoid(
		sql.exec(
			"UPDATE queue_items SET completed = 1, lease_until = NULL WHERE id = ? AND lease_owner = ?",
			id,
			owner,
		),
	);

/** @internal */
export const failItem = (
	sql: ActorSql,
	id: string,
	lastFailure: string,
	owner: string,
	terminal: boolean,
	visibleAt: number | null,
): Effect.Effect<void> => {
	const failed = "? OR attempts >= max_attempts";
	return Effect.asVoid(
		sql.exec(
			`UPDATE queue_items
       SET lease_until = CASE WHEN ${failed} THEN NULL ELSE ? END, lease_owner = NULL, last_failure = ?,
           completed = CASE WHEN ${failed} THEN 2 ELSE 0 END,
           position = (SELECT IFNULL(MAX(position), 0) + 1 FROM queue_items)
       WHERE id = ? AND lease_owner = ? AND completed = 0`,
			terminal ? 1 : 0,
			visibleAt,
			lastFailure,
			terminal ? 1 : 0,
			id,
			owner,
		),
	);
};

/** @internal */
export const releaseItem = (
	sql: ActorSql,
	id: string,
	owner: string,
): Effect.Effect<void> =>
	Effect.asVoid(
		sql.exec(
			`UPDATE queue_items SET attempts = attempts - 1, lease_until = NULL, lease_owner = NULL
       WHERE id = ? AND lease_owner = ? AND completed = 0`,
			id,
			owner,
		),
	);

/** @internal */
export const releaseOwnedItem = (
	sql: ActorSql,
	owner: string,
	now: number,
): Effect.Effect<void> =>
	Effect.asVoid(
		sql.exec(
			`UPDATE queue_items SET attempts = attempts - 1, lease_until = NULL, lease_owner = NULL
       WHERE lease_owner = ? AND completed = 0 AND lease_until > ?`,
			owner,
			now,
		),
	);

/** @internal */
export const extendLease = (
	sql: ActorSql,
	id: string,
	leaseUntil: number,
	owner: string,
): Effect.Effect<void> =>
	Effect.asVoid(
		sql.exec(
			"UPDATE queue_items SET lease_until = ? WHERE id = ? AND lease_owner = ? AND lease_until IS NOT NULL",
			leaseUntil,
			id,
			owner,
		),
	);

/** @internal */
export const expireLeases = (sql: ActorSql, now: number): Effect.Effect<void> =>
	Effect.asVoid(
		sql.exec(
			`UPDATE queue_items
       SET lease_until = NULL,
           completed = CASE WHEN lease_owner IS NOT NULL AND attempts >= max_attempts THEN 2 ELSE 0 END
       WHERE lease_until IS NOT NULL AND lease_until <= ? AND completed = 0`,
			now,
		),
	);

/** @internal */
export const earliestLeaseExpiry = (
	sql: ActorSql,
): Effect.Effect<number | undefined> =>
	Effect.map(
		sql.exec<LeaseExpiryRow>(
			"SELECT min(lease_until) AS lease_until FROM queue_items WHERE lease_until IS NOT NULL",
		),
		(rows) => rows[0]?.lease_until ?? undefined,
	);

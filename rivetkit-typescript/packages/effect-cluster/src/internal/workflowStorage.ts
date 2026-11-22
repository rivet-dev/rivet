/**
 * Workflow actor storage: the execution row, activity results, durable
 * deferred exits, and the clock due table.
 *
 * Portions adapted from Effect (MIT License,
 * Copyright (c) 2023 Effectful Technologies Inc). See `LICENSE-effect`.
 *
 * @internal
 */
import * as Effect from "effect/Effect";
import type { ActorSql } from "./sql.ts";

/** Statements that create the workflow tables. Run from `onMigrate`. */
export const workflowMigrations: ReadonlyArray<string> = [
	`CREATE TABLE IF NOT EXISTS workflow_execution (
    id INTEGER PRIMARY KEY CHECK (id = 0),
    workflow_name TEXT NOT NULL,
    execution_id TEXT NOT NULL,
    payload TEXT NOT NULL,
    parent_name TEXT,
    parent_execution_id TEXT,
    result TEXT,
    resume_pending INTEGER NOT NULL DEFAULT 0
  )`,
	`CREATE TABLE IF NOT EXISTS workflow_activities (
    key TEXT PRIMARY KEY,
    exit TEXT NOT NULL
  )`,
	`CREATE TABLE IF NOT EXISTS workflow_deferreds (
    name TEXT PRIMARY KEY,
    exit TEXT NOT NULL
  )`,
	`CREATE TABLE IF NOT EXISTS workflow_clocks (
    name TEXT PRIMARY KEY,
    deferred_name TEXT NOT NULL,
    wake_up INTEGER NOT NULL,
    fired INTEGER NOT NULL DEFAULT 0
  )`,
];

/** How a pending wake was requested. Only an explicit wake replays a failure suspension. */
export type WakeMode = "explicit" | "automatic";

/** @internal */
export interface ExecutionParent {
	readonly workflowName: string;
	readonly executionId: string;
}

/** @internal */
export interface ExecutionRow {
	readonly workflowName: string;
	readonly executionId: string;
	readonly payload: string;
	readonly parent: ExecutionParent | undefined;
	readonly result: string | undefined;
	readonly resumePending: WakeMode | undefined;
}

type StoredExecutionRow = {
	readonly workflow_name: string;
	readonly execution_id: string;
	readonly payload: string;
	readonly parent_name: string | null;
	readonly parent_execution_id: string | null;
	readonly result: string | null;
	readonly resume_pending: number;
};

type ExitRow = {
	readonly exit: string;
};

type ClockWakeUpRow = {
	readonly wake_up: number | null;
};

type ClockRow = {
	readonly name: string;
	readonly deferred_name: string;
};

const encodeWakeMode = (mode: WakeMode | undefined): number => {
	if (mode === undefined) return 0;
	return mode === "explicit" ? 1 : 2;
};

const decodeWakeMode = (value: number): WakeMode | undefined => {
	if (value === 1) return "explicit";
	if (value === 2) return "automatic";
	return undefined;
};

/** @internal */
export const loadExecution = (
	sql: ActorSql,
): Effect.Effect<ExecutionRow | undefined> =>
	Effect.map(
		sql.exec<StoredExecutionRow>(
			`SELECT workflow_name, execution_id, payload, parent_name, parent_execution_id, result, resume_pending
     FROM workflow_execution WHERE id = 0`,
		),
		(rows) => {
			const row = rows[0];
			if (row === undefined) return undefined;
			const parent =
				row.parent_name !== null && row.parent_execution_id !== null
					? {
							workflowName: row.parent_name,
							executionId: row.parent_execution_id,
						}
					: undefined;
			return {
				workflowName: row.workflow_name,
				executionId: row.execution_id,
				payload: row.payload,
				parent,
				result: row.result ?? undefined,
				resumePending: decodeWakeMode(row.resume_pending),
			};
		},
	);

/** @internal */
export const createExecution = (
	sql: ActorSql,
	workflowName: string,
	executionId: string,
	payload: string,
	parent: ExecutionParent | undefined,
): Effect.Effect<void> =>
	Effect.asVoid(
		sql.exec(
			`INSERT OR IGNORE INTO workflow_execution
       (id, workflow_name, execution_id, payload, parent_name, parent_execution_id, result)
     VALUES (0, ?, ?, ?, ?, ?, NULL)`,
			workflowName,
			executionId,
			payload,
			parent?.workflowName ?? null,
			parent?.executionId ?? null,
		),
	);

/** @internal */
export const setParent = (
	sql: ActorSql,
	parent: ExecutionParent,
): Effect.Effect<void> =>
	Effect.asVoid(
		sql.exec(
			"UPDATE workflow_execution SET parent_name = ?, parent_execution_id = ? WHERE id = 0 AND parent_name IS NULL",
			parent.workflowName,
			parent.executionId,
		),
	);

/** @internal */
export const saveSuspension = (
	sql: ActorSql,
	result: string,
	pending: WakeMode | undefined,
): Effect.Effect<void> =>
	Effect.asVoid(
		sql.exec(
			"UPDATE workflow_execution SET result = ?, resume_pending = ? WHERE id = 0",
			result,
			encodeWakeMode(pending),
		),
	);

/** Saves a final result. A pending explicit wake carries the parent wake. */
export const saveCompletion = (
	sql: ActorSql,
	result: string,
	wakeParent: boolean,
): Effect.Effect<void> =>
	Effect.asVoid(
		sql.exec(
			"UPDATE workflow_execution SET result = ?, resume_pending = ? WHERE id = 0",
			result,
			wakeParent ? 1 : 0,
		),
	);

/** @internal */
export const setResumePending = (
	sql: ActorSql,
	pending: WakeMode | undefined,
): Effect.Effect<void> =>
	Effect.asVoid(
		sql.exec(
			"UPDATE workflow_execution SET resume_pending = ? WHERE id = 0",
			encodeWakeMode(pending),
		),
	);

/** @internal */
export const loadActivity = (
	sql: ActorSql,
	key: string,
): Effect.Effect<string | undefined> =>
	Effect.map(
		sql.exec<ExitRow>(
			"SELECT exit FROM workflow_activities WHERE key = ?",
			key,
		),
		(rows) => rows[0]?.exit,
	);

/** @internal */
export const saveActivity = (
	sql: ActorSql,
	key: string,
	exit: string,
): Effect.Effect<void> =>
	Effect.asVoid(
		sql.exec(
			"INSERT OR IGNORE INTO workflow_activities (key, exit) VALUES (?, ?)",
			key,
			exit,
		),
	);

/** @internal */
export const loadDeferred = (
	sql: ActorSql,
	name: string,
): Effect.Effect<string | undefined> =>
	Effect.map(
		sql.exec<ExitRow>(
			"SELECT exit FROM workflow_deferreds WHERE name = ?",
			name,
		),
		(rows) => rows[0]?.exit,
	);

/** Saves a deferred exit unless one exists. Returns whether this call saved it. */
export const saveDeferred = (
	sql: ActorSql,
	name: string,
	exit: string,
): Effect.Effect<boolean> =>
	Effect.map(
		sql.exec<{ readonly name: string }>(
			"INSERT OR IGNORE INTO workflow_deferreds (name, exit) VALUES (?, ?) RETURNING name",
			name,
			exit,
		),
		(rows) => rows.length > 0,
	);

/** @internal */
export const saveClock = (
	sql: ActorSql,
	name: string,
	deferredName: string,
	wakeUp: number,
): Effect.Effect<void> =>
	Effect.asVoid(
		sql.exec(
			"INSERT OR IGNORE INTO workflow_clocks (name, deferred_name, wake_up, fired) VALUES (?, ?, ?, 0)",
			name,
			deferredName,
			wakeUp,
		),
	);

/** @internal */
export const earliestClockWakeUp = (
	sql: ActorSql,
): Effect.Effect<number | undefined> =>
	Effect.map(
		sql.exec<ClockWakeUpRow>(
			"SELECT min(wake_up) AS wake_up FROM workflow_clocks WHERE fired = 0",
		),
		(rows) => rows[0]?.wake_up ?? undefined,
	);

/** @internal */
export const dueClocks = (
	sql: ActorSql,
	now: number,
): Effect.Effect<
	Array<{ readonly name: string; readonly deferredName: string }>
> =>
	Effect.map(
		sql.exec<ClockRow>(
			"SELECT name, deferred_name FROM workflow_clocks WHERE fired = 0 AND wake_up <= ?",
			now,
		),
		(rows) =>
			rows.map((row) => ({
				name: row.name,
				deferredName: row.deferred_name,
			})),
	);

/** @internal */
export const markClockFired = (
	sql: ActorSql,
	name: string,
): Effect.Effect<void> =>
	Effect.asVoid(
		sql.exec("UPDATE workflow_clocks SET fired = 1 WHERE name = ?", name),
	);

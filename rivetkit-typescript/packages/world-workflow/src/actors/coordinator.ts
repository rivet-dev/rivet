/**
 * Coordinator actor.
 *
 * Singleton, keyed `["coordinator"]`. Serves as a cross-run index for:
 *
 * - `runs.list()` paginated queries
 * - `hooks.getByToken()` global token lookup
 *
 * The coordinator is an *index*, not a source of truth. The authoritative
 * state for each run lives in its `workflowRunActor`. Writes to the
 * coordinator are best-effort and eventually consistent.
 */

import { actor } from "rivetkit";
import type { WorkflowRunStatus } from "../types";

type HookStatus = "pending" | "disposed";

interface CoordinatorRunRow {
	id: string;
	workflowName: string;
	status: WorkflowRunStatus;
	createdAt: number;
	updatedAt: number;
	deploymentId?: string;
	parentRunId?: string;
}

interface CoordinatorHookRow {
	hookId: string;
	runId: string;
	token: string;
	status: HookStatus;
	createdAt: number;
}

interface CoordinatorState {
	runs: Record<string, CoordinatorRunRow>;
	hookTokens: Record<string, CoordinatorHookRow>;
	hookIds: Record<string, { runId: string; token: string }>;
}

export const coordinatorActor = actor({
	state: {
		runs: {},
		hookTokens: {},
		hookIds: {},
	} as CoordinatorState,
	actions: {
		registerRun: (c, row: CoordinatorRunRow) => {
			c.state.runs[row.id] = row;
		},

		updateRunStatus: (
			c,
			runId: string,
			status: WorkflowRunStatus,
			updatedAt: number,
		) => {
			const row = c.state.runs[runId];
			if (!row) return;
			row.status = status;
			row.updatedAt = updatedAt;
		},

		getRun: (c, runId: string) => {
			return c.state.runs[runId] ?? null;
		},

		listRuns: (
			c,
			opts?: {
				cursor?: string;
				limit?: number;
				workflowName?: string;
				status?: WorkflowRunStatus;
				deploymentId?: string;
				parentRunId?: string;
				createdAfter?: number;
				createdBefore?: number;
			},
		) => {
			const limit = opts?.limit ?? 50;
			let items = Object.values(c.state.runs);
			if (opts?.workflowName) {
				items = items.filter(
					(r) => r.workflowName === opts.workflowName,
				);
			}
			if (opts?.status) {
				items = items.filter((r) => r.status === opts.status);
			}
			if (opts?.deploymentId) {
				items = items.filter(
					(r) => r.deploymentId === opts.deploymentId,
				);
			}
			if (opts?.parentRunId) {
				items = items.filter(
					(r) => r.parentRunId === opts.parentRunId,
				);
			}
			if (opts?.createdAfter !== undefined) {
				const after = opts.createdAfter;
				items = items.filter((r) => r.createdAt >= after);
			}
			if (opts?.createdBefore !== undefined) {
				const before = opts.createdBefore;
				items = items.filter((r) => r.createdAt < before);
			}
			items.sort((a, b) => b.createdAt - a.createdAt);

			const startIdx = opts?.cursor
				? Number.parseInt(opts.cursor, 10)
				: 0;
			const slice = items.slice(startIdx, startIdx + limit);
			const nextIdx = startIdx + slice.length;
			return {
				data: slice,
				cursor: nextIdx < items.length ? String(nextIdx) : null,
				hasMore: nextIdx < items.length,
			};
		},

		registerHookToken: (
			c,
			row: CoordinatorHookRow,
		): { ok: true } | { ok: false; reason: "conflict" } => {
			const existing = c.state.hookTokens[row.token];
			if (existing && existing.status !== "disposed") {
				return { ok: false, reason: "conflict" };
			}
			c.state.hookTokens[row.token] = row;
			c.state.hookIds[row.hookId] = {
				runId: row.runId,
				token: row.token,
			};
			return { ok: true };
		},

		lookupHookToken: (c, token: string) => {
			return c.state.hookTokens[token] ?? null;
		},

		lookupHookId: (
			c,
			hookId: string,
		): { runId: string; token: string } | null => {
			return c.state.hookIds[hookId] ?? null;
		},

		updateHookStatus: (
			c,
			token: string,
			status: HookStatus,
		): void => {
			const row = c.state.hookTokens[token];
			if (!row) return;
			row.status = status;
			if (status === "disposed") {
				// Free the token slot for future reuse.
				delete c.state.hookTokens[token];
			}
		},

		disposeHookTokensForRun: (c, runId: string) => {
			for (const [token, row] of Object.entries(c.state.hookTokens)) {
				if (row.runId === runId) {
					delete c.state.hookTokens[token];
				}
			}
		},
	},
});

// APPROVAL REQUEST (Queue Wait Demo)
// Demonstrates: Queue waits with timeout for approval workflows
// One actor per approval request - actor key is the request ID

import { actor, event, queue } from "rivetkit";
import type { RawAccess } from "rivetkit/db";
import { Loop, workflow } from "rivetkit/workflow";
import {
	loadRecord,
	recordDb,
	saveRecord,
	updateRecord,
} from "./record-store.ts";

export type RequestStatus = "pending" | "approved" | "rejected" | "timeout";

export type ApprovalRequest = {
	id: string;
	title: string;
	description: string;
	status: RequestStatus;
	createdAt: number;
	decidedAt?: number;
	decidedBy?: string;
	deciding?: boolean; // True when a decision is being processed
};

const QUEUE_DECISION = "decision" as const;

const APPROVAL_TIMEOUT_MS = 30000;

export type ApprovalRequestInput = {
	title?: string;
	description?: string;
};

type ApprovalDecision = {
	approved: boolean;
	approver: string;
};

export const approval = actor({
	db: recordDb(),
	onCreate: async (c, input?: ApprovalRequestInput) => {
		await saveRecord<ApprovalRequest>(c.db, {
			id: c.key[0] as string,
			title: input?.title ?? "Untitled Request",
			description: input?.description ?? "",
			status: "pending",
			createdAt: Date.now(),
		});
	},
	queues: {
		decision: queue<ApprovalDecision>(),
	},
	events: {
		requestUpdated: event<ApprovalRequest>(),
		requestCreated: event<ApprovalRequest>(),
	},

	actions: {
		getRequest: (c): Promise<ApprovalRequest> =>
			loadRecord<ApprovalRequest>(c.db),

		approve: async (c, approver: string) => {
			const request = await markDeciding(c.db);
			if (!request) return;
			c.broadcast("requestUpdated", request);
			await c.queue.send(QUEUE_DECISION, { approved: true, approver });
		},

		reject: async (c, approver: string) => {
			const request = await markDeciding(c.db);
			if (!request) return;
			c.broadcast("requestUpdated", request);
			await c.queue.send(QUEUE_DECISION, { approved: false, approver });
		},
	},

	run: workflow(async (ctx) => {
		await ctx.loop("approval-loop", async (loopCtx) => {
			await loopCtx.step("init-request", async (c) => {
				const request = await loadRecord<ApprovalRequest>(c.db);
				c.log.info({
					msg: "waiting for approval decision",
					requestId: request.id,
					title: request.title,
				});
				c.broadcast("requestCreated", request);
			});

			const [decisionMessage] = await loopCtx.queue.nextBatch(
				"wait-decision",
				{
					names: [QUEUE_DECISION],
					timeout: APPROVAL_TIMEOUT_MS,
				},
			);
			const decision = decisionMessage?.body ?? null;

			await loopCtx.step("update-status", async (c) => {
				const updated = await updateRecord<ApprovalRequest>(
					c.db,
					(request) => {
						request.deciding = false;
						if (decision === null) {
							request.status = "timeout";
							c.log.info({
								msg: "request timed out",
								requestId: request.id,
							});
						} else if (decision.approved) {
							request.status = "approved";
							request.decidedBy = decision.approver;
							c.log.info({
								msg: "request approved",
								requestId: request.id,
								approver: decision.approver,
							});
						} else {
							request.status = "rejected";
							request.decidedBy = decision.approver;
							c.log.info({
								msg: "request rejected",
								requestId: request.id,
								approver: decision.approver,
							});
						}
						request.decidedAt = Date.now();
					},
				);
				c.broadcast("requestUpdated", updated);
			});

			return Loop.break(undefined);
		});
	}),
});

/**
 * Flags a pending request as deciding so clients can show that a decision is
 * on its way. Returns `undefined` when the request was already decided.
 */
async function markDeciding(
	database: RawAccess,
): Promise<ApprovalRequest | undefined> {
	return await database.transaction(
		async (tx) => {
			const request = await loadRecord<ApprovalRequest>(tx);
			if (request.status !== "pending") return undefined;
			request.deciding = true;
			await saveRecord(tx, request);
			return request;
		},
		{ name: "mark-deciding" },
	);
}

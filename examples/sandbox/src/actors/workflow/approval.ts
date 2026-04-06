// APPROVAL REQUEST (Queue Wait Demo)
// Demonstrates: Queue waits with timeout for approval workflows
// One actor per approval request - actor key is the request ID

import { actor, event, queue } from "rivetkit";
import { Loop, workflow } from "rivetkit/workflow";
import { actorCtx } from "./_helpers.ts";

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

type State = ApprovalRequest;

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
	createState: (c, input?: ApprovalRequestInput): ApprovalRequest => ({
		id: c.key[0] as string,
		title: input?.title ?? "Untitled Request",
		description: input?.description ?? "",
		status: "pending",
		createdAt: Date.now(),
	}),
	queues: {
		decision: queue<ApprovalDecision>(),
	},
	events: {
		requestUpdated: event<ApprovalRequest>(),
		requestCreated: event<ApprovalRequest>(),
	},

	actions: {
		getRequest: (c): ApprovalRequest => c.state,

		approve: async (c, approver: string) => {
			if (c.state.status !== "pending") return;
			c.state.deciding = true;
			c.broadcast("requestUpdated", c.state);
			await c.queue.send(QUEUE_DECISION, { approved: true, approver });
		},

		reject: async (c, approver: string) => {
			if (c.state.status !== "pending") return;
			c.state.deciding = true;
			c.broadcast("requestUpdated", c.state);
			await c.queue.send(QUEUE_DECISION, { approved: false, approver });
		},
	},

	run: workflow(async (ctx) => {
		await ctx.loop("approval-loop", async (loopCtx) => {
				const c = actorCtx<State>(loopCtx);

				await loopCtx.step("init-request", async () => {
					ctx.log.info({
						msg: "waiting for approval decision",
						requestId: c.state.id,
						title: c.state.title,
					});
					c.broadcast("requestCreated", c.state);
				});

				const [decisionMessage] = await loopCtx.queue.nextBatch(
					"wait-decision",
					{
						names: [QUEUE_DECISION],
						timeout: APPROVAL_TIMEOUT_MS,
					},
				);
				const decision = decisionMessage?.body ?? null;

				await loopCtx.step("update-status", async () => {
					c.state.deciding = false;
					if (decision === null) {
						c.state.status = "timeout";
						ctx.log.info({ msg: "request timed out", requestId: c.state.id });
					} else if (decision.approved) {
						c.state.status = "approved";
						c.state.decidedBy = decision.approver;
						ctx.log.info({
							msg: "request approved",
							requestId: c.state.id,
							approver: decision.approver,
						});
					} else {
						c.state.status = "rejected";
						c.state.decidedBy = decision.approver;
						ctx.log.info({
							msg: "request rejected",
							requestId: c.state.id,
							approver: decision.approver,
						});
					}
					c.state.decidedAt = Date.now();
					c.broadcast("requestUpdated", c.state);
				});

				return Loop.break(undefined);
			});
	}),
});

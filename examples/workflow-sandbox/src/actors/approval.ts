// APPROVAL REQUEST (Listen Demo)
// Demonstrates: Message listening with timeout for approval workflows
// One actor per approval request - actor key is the request ID

import { actor } from "rivetkit";
import { Loop, workflow, workflowQueueName } from "rivetkit/workflow";
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

const QUEUE_DECISION = workflowQueueName("decision");

const APPROVAL_TIMEOUT_MS = 30000;

export type ApprovalRequestInput = {
	title?: string;
	description?: string;
};

export const approval = actor({
	createState: (c, input: ApprovalRequestInput): ApprovalRequest => ({
		id: c.key[0] as string,
		title: input?.title ?? "Untitled Request",
		description: input?.description ?? "",
		status: "pending",
		createdAt: Date.now(),
	}),

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
		await ctx.loop({
			name: "approval-loop",
			run: async (loopCtx) => {
				const c = actorCtx<State>(loopCtx);

				await loopCtx.step("init-request", async () => {
					ctx.log.info({
						msg: "waiting for approval decision",
						requestId: c.state.id,
						title: c.state.title,
					});
					c.broadcast("requestCreated", c.state);
				});

				const decision = await loopCtx.listenWithTimeout<{
					approved: boolean;
					approver: string;
				}>("wait-decision", "decision", APPROVAL_TIMEOUT_MS);

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
			},
		});
	}),
});

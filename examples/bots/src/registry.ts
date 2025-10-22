import { actor, setup } from "rivetkit";
import type { WorkspaceInput, WorkspaceState } from "./types";
import { sendSlackMessage } from "./utils";

const DAY_IN_MS = 86_400_000;

export const slackWorkspaceBot = actor({
	createState: async (c, input): Promise<WorkspaceState> => {
		// Schedule first daily report
		const nextReportAt = Date.now() + DAY_IN_MS;
		await c.schedule.at(nextReportAt, "sendDailyReport");

		return {
			workspaceId: (input as WorkspaceInput).workspaceId,
			channelId: (input as WorkspaceInput).channelId,
			messageCount: 0,
			nextReportAt,
		};
	},

	actions: {
		// Called by the Slack webhook in the Hono server
		handleMessage: async (c, text: string) => {
			c.state.messageCount++;

			const msg = text.toLowerCase().trim();
			let response: string | undefined;

			if (msg === "ping") {
				response = "pong";
			} else if (msg === "count") {
				response = `I've received ${c.state.messageCount} messages in this workspace`;
			} else if (msg === "help") {
				response =
					"Available commands:\n• ping - responds with pong\n• count - shows message count\n• help - shows this message";
			}

			if (response) {
				await sendSlackMessage(c.state.channelId, response);
			}
		},

		sendDailyReport: async (c) => {
			// Schedule next report
			const nextReportAt = Date.now() + DAY_IN_MS;
			c.state.nextReportAt = nextReportAt;
			await c.schedule.at(nextReportAt, "sendDailyReport");

			// Send report to Slack if we have a channel
			if (c.state.channelId) {
				const report = `Daily report: ${c.state.messageCount} messages received so far`;
				await sendSlackMessage(c.state.channelId, report);
			}
		},
	},
});

export const registry = setup({
	use: { slackWorkspaceBot },
});

export type Registry = typeof registry;

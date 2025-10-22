import { Resend } from "resend";
import { actor, setup } from "rivetkit";
import type { CampaignInput, CampaignState } from "./types";

const DAY_IN_MS = 86_400_000;

const EMAIL_SUBJECT = "Daily campaign update";
const EMAIL_BODY = [
	"<p>Hi there,</p>",
	"<p>This is your automated daily campaign email from RivetKit.</p>",
	"<p>Have a great day!</p>",
].join("");

const emailCampaignUser = actor({
	createState: (_c, input: CampaignInput): CampaignState => ({
		email: input.email,
	}),

	onCreate: async (c) => {
		const nextSendAt = Date.now() + DAY_IN_MS;
		c.state.nextSendAt = nextSendAt;
		await c.schedule.at(nextSendAt, "sendDailyEmail");
	},

	actions: {
		sendDailyEmail: async (c) => {
			const resend = new Resend(process.env.RESEND_API_KEY ?? "");

			const { data, error } = await resend.emails.send({
				from: process.env.RESEND_FROM_EMAIL ?? "",
				to: c.state.email,
				subject: EMAIL_SUBJECT,
				html: EMAIL_BODY,
			});

			c.state.lastSentAt = Date.now();
			c.state.lastMessageId = data?.id ?? String(error ?? "");

			const nextSendAt = Date.now() + DAY_IN_MS;
			c.state.nextSendAt = nextSendAt;
			await c.schedule.at(nextSendAt, "sendDailyEmail");
		},

		getStatus: (c) => c.state,
	},
});

export const registry = setup({
	use: { emailCampaignUser },
});

export type Registry = typeof registry;

import { registry } from "./registry";

const { client } = registry.start();

async function main() {
	const userEmail = process.env.CAMPAIGN_USER_EMAIL;
	const userId = process.env.CAMPAIGN_USER_ID ?? "demo-user";

	if (!userEmail) {
		console.warn(
			"Set CAMPAIGN_USER_EMAIL to schedule the daily email campaign (e.g. CAMPAIGN_USER_EMAIL=user@example.com).",
		);
		return;
	}

	const campaign = client.emailCampaignUser.getOrCreate(userId, {
		createWithInput: { email: userEmail },
	});
	const status = await campaign.getStatus();

	const nextSend = status.nextSendAt
		? new Date(status.nextSendAt).toISOString()
		: "not scheduled";

	console.log(
		`Next daily email for ${status.email} scheduled at ${nextSend}`,
	);
}

void main();

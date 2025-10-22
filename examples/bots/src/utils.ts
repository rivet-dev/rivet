export async function sendSlackMessage(
	channelId: string,
	text: string,
): Promise<void> {
	const token = process.env.SLACK_BOT_TOKEN;
	if (!token) {
		return;
	}

	await fetch("https://slack.com/api/chat.postMessage", {
		method: "POST",
		headers: {
			Authorization: `Bearer ${token}`,
			"Content-Type": "application/json",
		},
		body: JSON.stringify({
			channel: channelId,
			text,
		}),
	});
}

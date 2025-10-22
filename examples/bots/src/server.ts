import { serve } from "@hono/node-server";
import { Hono } from "hono";
import { registry } from "./registry";

const { client } = registry.start();

const app = new Hono();

app.post("/slack/events", async (c) => {
	const body = await c.req.json();

	if (body.type === "url_verification") {
		return c.json({ challenge: body.challenge });
	}

	if (body.type === "event_callback" && body.event.type === "message") {
		const workspaceId = body.team_id;
		const channelId = body.event.channel;

		const bot = client.slackWorkspaceBot.getOrCreate(workspaceId, {
			createWithInput: {
				workspaceId,
				channelId,
			},
		});

		await bot.handleMessage(body.event.text);

		return c.json({ ok: true });
	}

	return c.json({ ok: false }, 400);
});

serve({ fetch: app.fetch, port: 8080 });
console.log("Slack bot listening on port 8080");

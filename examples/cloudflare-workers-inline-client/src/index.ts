import { createInlineClient } from "@rivetkit/cloudflare-workers";
import { registry } from "./registry";

const { client, ActorHandler } = createInlineClient(registry);

// IMPORTANT: Your Durable Object must be exported here
export { ActorHandler };

export default {
	fetch: async (request) => {
		const url = new URL(request.url);

		if (
			request.method === "POST" &&
			url.pathname.startsWith("/increment/")
		) {
			const name = url.pathname.slice("/increment/".length);

			const counter = client.counter.getOrCreate(name);
			const newCount = await counter.increment(1);

			return new Response(`New Count: ${newCount}`, {
				headers: { "Content-Type": "text/plain" },
			});
		}

		return new Response("Not Found", { status: 404 });
	},
} satisfies ExportedHandler;

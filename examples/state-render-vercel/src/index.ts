import "./env.ts";
import { registry } from "./actors.ts";
import { port, useRivetCloud } from "./env.ts";

if (useRivetCloud) {
	const { serve } = await import("@hono/node-server");
	const { default: app } = await import("./server.ts");

	serve({ fetch: app.fetch, port }, () => {
		console.log(`state-render listening on http://0.0.0.0:${port}`);
	});
} else {
	registry.start();
}

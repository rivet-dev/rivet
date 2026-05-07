import "./env.ts";
import { port } from "./env.ts";

const { serve } = await import("@hono/node-server");
const { default: app } = await import("./server.ts");

serve({ fetch: app.fetch, port }, () => {
	console.log(`stream-render listening on http://0.0.0.0:${port}`);
});

import { Hono } from "hono";
import { serveStatic, upgradeWebSocket } from "hono/deno";
import { registry } from "./registry";

const serverOutput = registry.start({
	inspector: {
		enabled: true,
	},
	disableDefaultServer: true,
	basePath: "/api",
	getUpgradeWebSocket: () => upgradeWebSocket,
	overrideServerAddress: `${process.env.FREESTYLE_ENDPOINT ?? "http://localhost:8080"}/api`,
});

const app = new Hono();
app.use("/api/*", async (c) => {
	return await serverOutput.fetch(c.req.raw);
});
app.use("*", serveStatic({ root: "./public" }));

// Under the hood, Freestyle uses Deno
// for their Web Deploy instances
// @ts-expect-error
Deno.serve({ port: 8080 }, app.fetch);

import { Hono } from "hono";
import { serveStatic, upgradeWebSocket } from "hono/deno";
import { registry } from "./registry.ts";

globalThis.addEventListener("unhandledrejection", (event) => {
	console.error("Unhandled promise rejection:", event.reason);
	event.preventDefault();
});

const serverOutput = registry.start({
	inspector: {
		enabled: true,
	},
	runnerKind: "serverless",
	disableDefaultServer: true,
	noWelcome: true,
	runEngine: false,
	autoConfigureServerless: false,
	basePath: "/api/rivet",
	getUpgradeWebSocket: () => upgradeWebSocket,
});

const app = new Hono();

app.use("/api/rivet/*", async (c) => {
	return await serverOutput.fetch(c.req.raw);
});

app.use("*", serveStatic({ root: "./public" }));

// @ts-expect-error
Deno.serve(app.fetch);

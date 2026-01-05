import { Hono } from "hono";
import { upgradeWebSocket, websocket } from "hono/bun";
import { cors } from "hono/cors";
import { registry } from "./registry";

const { client, fetch } = registry.start({
	basePath: "/rivet",
	// Hono requires using Hono.serve
	disableDefaultServer: true,
	// Override endpoint
	overrideServerAddress: "http://localhost:8080/rivet",
	// Specify Hono-specific upgradeWebSocket
	getUpgradeWebSocket: () => upgradeWebSocket,
});

// Setup router
const app = new Hono();

app.use(
	"*",
	cors({
		origin: "http://localhost:5173",
		credentials: true,
	}),
);

app.use("/rivet/*", async (c) => {
	return await fetch(c.req.raw, c.env);
});

// Example HTTP endpoint
app.post("/increment/:name", async (c) => {
	const name = c.req.param("name");

	const counter = client.counter.getOrCreate(name);
	const newCount = await counter.increment(1);

	return c.text(`New Count: ${newCount}`);
});

Bun.serve({
	port: 8080,
	fetch: app.fetch,
	websocket,
});

console.log("Listening at http://localhost:8080");

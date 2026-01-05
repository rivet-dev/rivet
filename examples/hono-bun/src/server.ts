import { Hono } from "hono";
import { cors } from "hono/cors";
import { createClient } from "rivetkit/client";
import { registry } from "./registry";

const client = createClient<typeof registry>();

// Setup router
const app = new Hono();

app.use(
	"*",
	cors({
		origin: "http://localhost:5173",
		credentials: true,
	}),
);

app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));

// Example HTTP endpoint
app.post("/increment/:name", async (c) => {
	const name = c.req.param("name");

	const counter = client.counter.getOrCreate(name);
	const newCount = await counter.increment(1);

	return c.text(`New Count: ${newCount}`);
});

export default app;

console.log("Listening at http://localhost:6420");

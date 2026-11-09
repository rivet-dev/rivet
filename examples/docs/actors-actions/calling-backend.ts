import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";
import { Hono } from "hono";

// Define actor
const counter = actor({
  vars: { count: 0 },
  actions: {
    increment: (c, amount: number) => {
      c.vars.count += amount;
      return c.vars.count;
    }
  }
});

// Create registry
const registry = setup({ use: { counter } });

// Create client
const client = createClient<typeof registry>("http://localhost:6420");

const app = new Hono();

// Mount Rivet handler
app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));

// Use the client to call actions on a request
app.get("/foo", async (c) => {
	const counterActor = client.counter.getOrCreate();
	const result = await counterActor.increment(42);
	return c.text(String(result));
});

export default app;

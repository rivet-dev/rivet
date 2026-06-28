import { Hono } from "hono";
import { createClient } from "rivetkit/client";

const app = new Hono();
const client = createClient();

app.post("/increment/:name", async (c) => {
  const counterHandle = client.counter.getOrCreate([c.req.param("name")]);
  const newCount = await counterHandle.increment(1);
  return c.json({ count: newCount });
});

import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";
import { Hono } from "hono";

const processor = actor({
  state: {},
  actions: {
    process: (c, body: unknown) => ({ processed: true }),
    destroySelf: (c) => c.destroy(),
  },
});

const registry = setup({ use: { processor } });
const client = createClient<typeof registry>("http://localhost:6420");
const app = new Hono();

// Bad: creating an actor for each API request
app.post("/process", async (c) => {
  const actorHandle = client.processor.getOrCreate([crypto.randomUUID()]);
  const result = await actorHandle.process(await c.req.json());
  await actorHandle.destroySelf();
  return c.json(result);
});

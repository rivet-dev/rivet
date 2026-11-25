import { createClient } from "rivetkit/client";
import { actor, setup } from "rivetkit";

const counter = actor({
  vars: { count: 0 },
  actions: {
    increment: (c, amount: number) => {
      c.vars.count += amount;
      return c.vars.count;
    }
  }
});

const registry = setup({ use: { counter } });
const client = createClient<typeof registry>("http://localhost:6420");
const counterActor = client.counter.getOrCreate();

// Give up if the call takes longer than 5 seconds
const count = await counterActor.action({
  name: "increment",
  args: [1],
  signal: AbortSignal.timeout(5_000),
});

// Connections take a signal the same way
const conn = counterActor.connect();
const controller = new AbortController();
const pending = conn.action({
  name: "increment",
  args: [1],
  signal: controller.signal,
});

// Stop waiting, for example when the user leaves the page
controller.abort();

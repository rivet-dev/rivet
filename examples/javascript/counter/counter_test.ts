import { TestClient } from "@rivet-gg/actor-client/test";
import { assertEquals } from "@std/assert";
import type Counter from "./counter.ts";

const client = new TestClient();

// Get-or-create a counter actor
const counter = await client.get<Counter>({ name: "counter" });

// Listen for update count events (https://rivet.gg/docs/events)
counter.on("countUpdate", (count: number) => console.log("New count:", count));

// Increment the count over remote procedure call (https://rivet.gg/docs/rpc)
const count1 = await counter.increment(1);
const count2 = await counter.increment(2);
assertEquals(count2, count1 + 2);

// Disconnect from the actor when finished (https://rivet.gg/docs/connections)
await counter.disconnect();

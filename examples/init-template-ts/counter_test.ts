import { TestClient } from "@rivet-gg/actor-client/test";
import type Counter from "./counter.ts";

const client = new TestClient();

// Get-or-create a counter actor
const counter = await client.get<Counter>({ name: "counter" });

// Listen for update count events (https://rivet.gg/docs/events)
counter.on("count", (count: number) => console.log("New count:", count));

// Increment the count over remote procedure call (https://rivet.gg/docs/rpc)
await counter.increment(1);

// Disconnect from the actor when finished (https://rivet.gg/docs/connections)
counter.disconnect();

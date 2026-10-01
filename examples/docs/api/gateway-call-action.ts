import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>(process.env.RIVET_ENDPOINT);

// getOrCreate resolves the actor by name and key, the same as
// rvt-method=getOrCreate&rvt-key=my-counter over HTTP.
const counter = client.counter.getOrCreate(["my-counter"]);

// Calling an action sends POST /gateway/{actor}/action/increment
// with body { "args": [1] } and returns the "output" field.
const count = await counter.increment(1);
console.log(count); // 1

import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>(process.env.RIVET_ENDPOINT);
const conn = client.counter.getOrCreate(["my-counter"]).connect();

// Actions called on a connection are sent as ActionRequest messages over the
// WebSocket instead of separate HTTP requests. The promise resolves with the
// ActionResponse output.
const count = await conn.increment(1);
console.log(count); // 1

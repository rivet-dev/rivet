import { createClient } from "rivetkit/client";

const client = createClient();
const handle = client.counter.getOrCreate(["my-counter"]);

// Stateless: each call is independent
await handle.increment(1);

// Stateful: keep a connection open for realtime events
const conn = handle.connect();
conn.on("count", (value) => console.log(value));
await conn.increment(1);

import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>("http://localhost:6420");

// Get or create a counter actor for the key "my-counter"
const counter = client.counter.getOrCreate(["my-counter"]);

// Call actions
const count = await counter.increment(3);
console.log("New count:", count);

// Listen to realtime events
const connection = counter.connect();
connection.on("newCount", (newCount: number) => {
	console.log("Count changed:", newCount);
});

// Increment through connection
await connection.increment(1);

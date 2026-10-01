import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>(process.env.RIVET_ENDPOINT);
const conn = client.counter.getOrCreate(["my-counter"]).connect();

// on() sends a SubscriptionRequest and routes matching Event messages to the
// callback. The returned function unsubscribes.
const unsubscribe = conn.on("countChanged", (count: number) => {
	console.log("count is now", count);
});

await conn.increment(1); // logs "count is now 1"
unsubscribe();

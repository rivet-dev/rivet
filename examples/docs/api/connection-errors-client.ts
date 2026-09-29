import { ActorError, createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>(process.env.RIVET_ENDPOINT);
const conn = client.counter.getOrCreate(["my-counter"]).connect();

// Error messages that answer an action reject that action's promise.
try {
	await conn.increment(1);
} catch (error) {
	if (error instanceof ActorError) {
		console.error(`${error.group}.${error.code}: ${error.message}`);
	}
}

// Errors that are not tied to an action go to onError.
conn.onError((error) => {
	console.error(`connection error: ${error.group}.${error.code}`);
});

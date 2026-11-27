import { createClient } from "rivetkit/client";
import type { registry } from "./counter";

const client = createClient<typeof registry>("http://localhost:6420");

// docs:start by-name
// idFromName("user-123") + get(id), or getByName("user-123")
const counter = client.counter.getOrCreate(["user-123"]);
const value = await counter.increment(5);
// docs:end by-name

// docs:start unique
// newUniqueId(): create a new actor and keep its ID
const created = await client.counter.create(["session", crypto.randomUUID()]);
const actorId = await created.resolve();

// idFromString(hex): look the actor up again by its ID
const sameCounter = client.counter.getForId(actorId);
await sameCounter.get();
// docs:end unique

// docs:start existing
// Only reach an actor that already exists; calls fail if it was never created
const existing = client.counter.get(["user-123"]);
await existing.get();
// docs:end existing

// docs:start region
// locationHint: choose the region when the actor is first created
const nearUser = client.counter.getOrCreate(["user-456"], {
	createInRegion: "atl",
});
await nearUser.get();
// docs:end region

console.log(value);

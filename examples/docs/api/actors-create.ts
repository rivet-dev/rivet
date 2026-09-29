import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>(process.env.RIVET_ENDPOINT);

// create() sends POST /actors and returns a handle bound to the new actor ID.
// It fails if an actor named "counter" with this key already exists.
const counter = await client.counter.create([`counter-${crypto.randomUUID()}`]);

const actorId = await counter.resolve();
console.log(actorId);

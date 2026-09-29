import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>(process.env.RIVET_ENDPOINT);

// getOrCreate() returns a handle immediately. The PUT /actors call happens
// lazily when the handle is first used, or when you call resolve().
const counter = client.counter.getOrCreate(["my-counter"]);

const actorId = await counter.resolve();
console.log(actorId);

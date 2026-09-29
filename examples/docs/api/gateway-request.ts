import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>(process.env.RIVET_ENDPOINT);

const counter = client.counter.getOrCreate(["my-counter"]);

// .fetch() sends the request to the actor's onRequest handler. It accepts
// the same arguments as the global fetch and returns a standard Response.
const response = await counter.fetch("/increment", { method: "POST" });
const data = await response.json();
console.log(data); // { count: 1 }

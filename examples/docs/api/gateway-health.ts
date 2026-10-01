import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>(process.env.RIVET_ENDPOINT);

// The RivetKit client has no dedicated health call. Resolve the actor ID,
// build its gateway URL, and hit /health with plain fetch instead.
const actorId = await client.counter.getOrCreate(["my-counter"]).resolve();
const gatewayUrl = await client.counter.getForId(actorId).getGatewayUrl();

const response = await fetch(`${gatewayUrl}/health`);
console.log(response.status, await response.text()); // 200 ok

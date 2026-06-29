import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>("http://localhost:6420");

const actor = client.counter.getOrCreate(["my-counter"]);

// Get the raw gateway URL
const gatewayUrl = await actor.getGatewayUrl();
// gatewayUrl = "https://...rivet.dev/..."

// Use with native fetch
const response = await fetch(`${gatewayUrl}/request/`);
const data = await response.json();
console.log(data); // { count: 0 }

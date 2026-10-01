import { createClient } from "rivetkit/client";
import type { registry } from "./index";

const client = createClient<typeof registry>(process.env.RIVET_ENDPOINT);

// connect() opens a WebSocket to /gateway/{actor}/connect and completes the
// Init handshake. The connection reconnects automatically if it drops.
const counter = client.counter.getOrCreate(["my-counter"]);
const conn = counter.connect();

// Close the socket when you are done with it.
await conn.dispose();

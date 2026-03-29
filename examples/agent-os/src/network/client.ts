// Networking: start a server inside the VM, fetch from it, and create a preview URL.

import { createClient } from "rivetkit/client";
import type { registry } from "./server.ts";

const client = createClient<typeof registry>("http://localhost:6420");
const agent = client.vm.getOrCreate(["my-agent"]);

// Write a server script to run inside the VM on a fixed port
await agent.writeFile(
	"/tmp/server.js",
	`
const http = require("http");
const server = http.createServer((req, res) => {
  res.writeHead(200, { "Content-Type": "application/json" });
  res.end(JSON.stringify({ status: "ok", method: req.method, url: req.url }));
});
server.listen(3000, "0.0.0.0", () => {
  console.log("Server ready on port 3000");
});
`,
);

// Spawn the server inside the VM
const proc = (await agent.spawn("node", ["/tmp/server.js"])) as {
	pid: number;
};
console.log("Spawned server process:", proc.pid);

// Give the server time to start
await new Promise((r) => setTimeout(r, 1000));

// vmFetch routes HTTP requests to the VM's localhost
const response = (await agent.vmFetch(
	3000,
	"http://localhost/api/test",
)) as { body: Uint8Array };
const body = JSON.parse(new TextDecoder().decode(response.body));
console.log("vmFetch response:", body);

// Create a signed preview URL for external access
const preview = (await agent.createSignedPreviewUrl(3000)) as {
	path: string;
	token: string;
	port: number;
	expiresAt: number;
};
console.log("Preview URL path:", preview.path);
console.log("Expires at:", new Date(preview.expiresAt).toISOString());

// Expire the preview URL
await agent.expireSignedPreviewUrl(preview.token);
console.log("Preview URL expired");

await agent.killProcess(proc.pid);

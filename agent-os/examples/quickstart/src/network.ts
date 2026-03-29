// Networking: fetch from services running inside the VM.
//
// The VM can run HTTP servers that bind to real host ports.
// vm.fetch(port, request) routes HTTP requests to those ports.

import { AgentOs } from "@rivet-dev/agent-os-core";

const vm = await AgentOs.create();

// Write a server script to run INSIDE the VM
await vm.writeFile(
	"/tmp/server.js",
	`
const http = require("http");
const server = http.createServer((req, res) => {
  res.writeHead(200, { "Content-Type": "application/json" });
  res.end(JSON.stringify({ status: "ok", method: req.method, url: req.url }));
});
server.listen(0, "0.0.0.0", () => {
  console.log("LISTENING:" + server.address().port);
});
`,
);

// Spawn the server inside the VM and wait for it to start
let resolvePort: (port: number) => void;
const portPromise = new Promise<number>((resolve) => {
	resolvePort = resolve;
});

const proc = vm.spawn("node", ["/tmp/server.js"], {
	onStdout: (data: Uint8Array) => {
		const text = new TextDecoder().decode(data);
		const match = text.match(/LISTENING:(\d+)/);
		if (match) resolvePort(Number(match[1]));
	},
});

const port = await portPromise;
console.log("Server listening on port", port);

// vm.fetch() sends HTTP requests to the VM server via localhost
const response = await vm.fetch(port, new Request("http://localhost/api/test"));
const json = await response.json();
console.log("Response:", json);

proc.kill();
await vm.dispose();

// Sandbox mounting: write files and run commands in a Docker sandbox.
//
// The /sandbox mount projects the sandbox filesystem into the VM.
// The sandbox toolkit is accessible via the tools RPC server.

import { createClient } from "rivetkit/client";
import type { registry } from "./server.ts";

const client = createClient<typeof registry>("http://localhost:6420");
const agent = client.vm.getOrCreate(["my-agent"]);

// Write and read a file through the mounted sandbox filesystem
await agent.writeFile("/sandbox/hello.txt", "Hello from agentOS!");
const content = (await agent.readFile("/sandbox/hello.txt")) as Uint8Array;
console.log("Read from sandbox mount:", new TextDecoder().decode(content));

// Get the tools RPC port to call sandbox commands
const env = (await agent.exec("echo $AGENTOS_TOOLS_PORT")) as {
	stdout: string;
};
const port = env.stdout.trim();

// Run a command inside the Docker sandbox via the toolkit RPC
await agent.writeFile(
	"/tmp/sandbox-cmd.mjs",
	`
import { writeFileSync } from "node:fs";
const res = await fetch("http://127.0.0.1:${port}/call", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    toolkit: "sandbox",
    tool: "run-command",
    input: { command: "echo", args: ["hello from Docker sandbox"] },
  }),
});
writeFileSync("/tmp/sandbox-out.json", await res.text());
`,
);
const proc = (await agent.spawn("node", ["/tmp/sandbox-cmd.mjs"])) as {
	pid: number;
};
await agent.waitProcess(proc.pid);
const result = (await agent.readFile("/tmp/sandbox-out.json")) as Uint8Array;
console.log("Sandbox command:", new TextDecoder().decode(result));

// List processes in the sandbox
await agent.writeFile(
	"/tmp/sandbox-ps.mjs",
	`
import { writeFileSync } from "node:fs";
const res = await fetch("http://127.0.0.1:${port}/call", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ toolkit: "sandbox", tool: "list-processes", input: {} }),
});
writeFileSync("/tmp/sandbox-ps.json", await res.text());
`,
);
const psProc = (await agent.spawn("node", ["/tmp/sandbox-ps.mjs"])) as {
	pid: number;
};
await agent.waitProcess(psProc.pid);
const psList = (await agent.readFile("/tmp/sandbox-ps.json")) as Uint8Array;
console.log("Sandbox processes:", new TextDecoder().decode(psList));

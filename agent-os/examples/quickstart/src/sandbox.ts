// Sandbox extension: mount a Docker sandbox filesystem and run commands.
//
// Requires Docker. Starts a sandbox-agent container, mounts its filesystem
// at /sandbox, and registers the sandbox toolkit for running commands.

import { AgentOs } from "@rivet-dev/agent-os-core";
import common from "@rivet-dev/agent-os-common";
import { SandboxAgent } from "sandbox-agent";
import { docker } from "sandbox-agent/docker";
import { createSandboxFs, createSandboxToolkit } from "@rivet-dev/agent-os-sandbox";

// Start a Docker-backed sandbox.
const sandbox = await SandboxAgent.start({
	sandbox: docker(),
});

// Mount the sandbox filesystem at /sandbox and register the toolkit.
const vm = await AgentOs.create({
	software: [common],
	mounts: [
		{
			path: "/sandbox",
			driver: createSandboxFs({ client: sandbox }),
		},
	],
	toolKits: [createSandboxToolkit({ client: sandbox })],
});

// Write and read a file through the mounted sandbox filesystem.
await vm.writeFile("/sandbox/hello.txt", "Hello from agentOS!");
const content = await vm.readFile("/sandbox/hello.txt");
console.log("Read from sandbox mount:", new TextDecoder().decode(content));

// Get the tools RPC port to call sandbox commands
const env = await vm.exec("echo $AGENTOS_TOOLS_PORT");
const port = env.stdout.trim();

// Run a command inside the Docker sandbox via the toolkit RPC
await vm.writeFile(
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
const proc = vm.spawn("node", ["/tmp/sandbox-cmd.mjs"]);
await vm.waitProcess(proc.pid);
const result = await vm.readFile("/tmp/sandbox-out.json");
console.log("Sandbox command:", new TextDecoder().decode(result));

// List processes in the sandbox
await vm.writeFile(
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
const psProc = vm.spawn("node", ["/tmp/sandbox-ps.mjs"]);
await vm.waitProcess(psProc.pid);
const psList = await vm.readFile("/tmp/sandbox-ps.json");
console.log("Sandbox processes:", new TextDecoder().decode(psList));

await vm.dispose();
await sandbox.dispose();

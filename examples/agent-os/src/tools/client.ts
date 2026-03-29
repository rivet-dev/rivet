// Host toolkits: call tools registered on the server from inside the VM.
//
// Each toolkit's tools are accessible via an HTTP RPC server inside the VM
// at the port specified by AGENTOS_TOOLS_PORT. Node scripts inside the VM
// can call the server directly with fetch.

import { createClient } from "rivetkit/client";
import type { registry } from "./server.ts";

const client = createClient<typeof registry>("http://localhost:6420");
const agent = client.vm.getOrCreate(["my-agent"]);

// Get the tools RPC port from the VM environment
const env = (await agent.exec("echo $AGENTOS_TOOLS_PORT")) as {
	stdout: string;
};
const port = env.stdout.trim();
console.log("Tools RPC port:", port);

// Helper: call a tool via the RPC server and read the result from a temp file
async function callTool(
	toolkit: string,
	tool: string,
	input: Record<string, unknown>,
): Promise<unknown> {
	const outFile = `/tmp/${toolkit}-${tool}-out.json`;
	await agent.writeFile(
		"/tmp/tool-call.mjs",
		`
import { writeFileSync } from "node:fs";
const res = await fetch("http://127.0.0.1:${port}/call", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify(${JSON.stringify({ toolkit, tool, input })}),
});
writeFileSync("${outFile}", await res.text());
`,
	);
	const proc = (await agent.spawn("node", ["/tmp/tool-call.mjs"])) as {
		pid: number;
	};
	await agent.waitProcess(proc.pid);
	const data = (await agent.readFile(outFile)) as Uint8Array;
	return JSON.parse(new TextDecoder().decode(data));
}

// Call the weather tool
const weather = await callTool("weather", "get", { city: "London" });
console.log("Weather:", JSON.stringify(weather));

// Call the calculator tool
const sum = await callTool("calc", "add", { a: 10, b: 32 });
console.log("Sum:", JSON.stringify(sum));

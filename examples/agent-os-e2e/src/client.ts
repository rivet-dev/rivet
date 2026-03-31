// Agent OS E2E Smoke Test
//
// Tests: VM boot, filesystem, subprocess execution, preview URLs, agent session.
//
// Usage:
//   1. Start the server:  npx tsx src/server.ts
//   2. Run the client:    npx tsx src/client.ts
//
// Requires ANTHROPIC_API_KEY environment variable.

import { createClient } from "rivetkit/client";
import type { registry } from "./server.ts";

const ANTHROPIC_API_KEY = process.env.ANTHROPIC_API_KEY;
if (!ANTHROPIC_API_KEY) {
	console.error("ANTHROPIC_API_KEY is required");
	process.exit(1);
}

const client = createClient<typeof registry>("http://localhost:6420");
const agent = client.vm.getOrCreate(["e2e-test"]);

// --- Step 1: Filesystem basics ---
console.log("=== Step 1: Filesystem ===");
await agent.writeFile("/tmp/hello.txt", "Hello from Agent OS!");
const raw = (await agent.readFile("/tmp/hello.txt")) as Uint8Array;
const text = new TextDecoder().decode(raw);
console.log(`writeFile + readFile: "${text.trim()}"`);
assert(text.includes("Hello from Agent OS!"), "filesystem round-trip");

await agent.mkdir("/home/user/project");
const entries = (await agent.readdir("/home/user/project")) as string[];
console.log("mkdir + readdir:", entries.filter((e) => e !== "." && e !== ".."));
console.log("exists /home/user/project:", await agent.exists("/home/user/project"));
console.log("exists /nonexistent:", await agent.exists("/nonexistent"));

// --- Step 2: Subprocess execution ---
console.log("\n=== Step 2: Processes ===");
const echo = (await agent.exec("echo 'hello from bash'")) as {
	stdout: string;
	exitCode: number;
};
console.log(`exec echo: "${echo.stdout.trim()}" (exit ${echo.exitCode})`);
assert(echo.exitCode === 0, "echo exit code");
assert(echo.stdout.trim() === "hello from bash", "echo output");

const pipe = (await agent.exec("echo hello | tr a-z A-Z")) as {
	stdout: string;
};
console.log(`exec pipe: "${pipe.stdout.trim()}"`);
assert(pipe.stdout.trim() === "HELLO", "pipe output");

await agent.writeFile("/tmp/data.txt", "apple\nbanana\ncherry\napricot\n");
const grep = (await agent.exec("grep ap /tmp/data.txt")) as {
	stdout: string;
};
console.log(`exec grep: "${grep.stdout.trim()}"`);
assert(grep.stdout.includes("apple"), "grep apple");
assert(grep.stdout.includes("apricot"), "grep apricot");

const cat = (await agent.exec("cat /tmp/hello.txt")) as { stdout: string };
console.log(`exec cat: "${cat.stdout.trim()}"`);
assert(cat.stdout.includes("Hello from Agent OS!"), "cat reads file written by writeFile");

// --- Step 3: Preview URL ---
console.log("\n=== Step 3: Preview URL ===");

// Write a tiny HTTP server script into the VM
await agent.writeFile(
	"/tmp/server.mjs",
	`import http from "node:http";
const server = http.createServer((req, res) => {
  res.writeHead(200, { "Content-Type": "text/plain" });
  res.end("preview ok");
});
server.listen(8080, () => console.log("listening on 8080"));
`,
);

// Spawn the server inside the VM
const serverProc = (await agent.spawn("node", ["/tmp/server.mjs"])) as {
	pid: number;
};
console.log(`Spawned preview server: pid ${serverProc.pid}`);

// Give the server a moment to bind
await new Promise((r) => setTimeout(r, 1000));

// Create a signed preview URL for port 8080
const preview = (await agent.createSignedPreviewUrl(8080, 60)) as {
	path: string;
	token: string;
	port: number;
	expiresAt: number;
};
console.log(`Preview path: ${preview.path}`);
console.log(`Preview token: ${preview.token}`);

// Fetch through the preview proxy
const gatewayUrl = await agent.getGatewayUrl();
const previewUrl = `${gatewayUrl}${preview.path}`;
console.log(`Fetching preview URL: ${previewUrl}`);
const previewResponse = await fetch(previewUrl);
const previewBody = await previewResponse.text();
console.log(`Preview response: ${previewResponse.status} "${previewBody}"`);
assert(previewResponse.status === 200, "preview status 200");
assert(previewBody === "preview ok", "preview body matches");

// Clean up the server process
await agent.killProcess(serverProc.pid);

// --- Step 4: Agent session (Pi + Anthropic) ---
console.log("\n=== Step 4: Agent session ===");
console.log("Creating Pi agent session...");
const session = (await agent.createSession("pi", {
	env: { ANTHROPIC_API_KEY },
})) as { sessionId: string };
console.log(`Session created: ${session.sessionId}`);

// Subscribe to streaming events via WebSocket connection
const conn = agent.connect();
conn.on("sessionEvent", (data: any) => {
	const event = data?.event ?? data;
	const params = event?.params;
	if (params?.update?.sessionUpdate === "agent_message_chunk") {
		process.stdout.write(params.update.content?.text ?? "");
	}
});

// Wait for WebSocket to establish
await new Promise((r) => setTimeout(r, 500));

console.log("\nSending prompt...");
const response = (await agent.sendPrompt(
	session.sessionId,
	'Write the text "E2E test passed!" to /tmp/e2e-result.txt using the write tool. Then use the bash tool to run `cat /tmp/e2e-result.txt` and tell me what it says.',
)) as { stopReason?: string };
console.log(`\n\nPrompt completed: ${response?.stopReason ?? "done"}`);

// --- Step 5: Verify agent wrote the file ---
console.log("\n=== Step 5: Verify agent output ===");
try {
	const agentData = (await agent.readFile("/tmp/e2e-result.txt")) as Uint8Array;
	const agentText = new TextDecoder().decode(agentData);
	console.log(`Agent wrote: "${agentText.trim()}"`);
	assert(agentText.includes("E2E test passed!"), "agent file content");
} catch (err: any) {
	console.error(`Failed to read agent output: ${err.message}`);
	process.exit(1);
}

// --- Cleanup ---
await conn.dispose();
await agent.closeSession(session.sessionId);

console.log("\n=== Results ===");
console.log("All checks passed!");

// Simple assertion helper
function assert(condition: boolean, label: string) {
	if (!condition) {
		console.error(`FAILED: ${label}`);
		process.exit(1);
	}
	console.log(`  OK: ${label}`);
}

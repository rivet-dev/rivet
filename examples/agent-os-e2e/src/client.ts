// Agent OS E2E Smoke Test
//
// Tests: VM boot, filesystem, subprocess execution, preview URLs, agent session.
//
// Usage:
//   1. Start the server:  npx tsx src/server.ts
//   2. Run the client:    npx tsx src/client.ts
//
import { LLMock } from "@copilotkit/llmock";
import { createClient } from "rivetkit/client";
import type { registry } from "./server.ts";

const LLMOCK_PORT = Number(process.env.E2E_LLMOCK_PORT ?? "41235");
const LLMOCK_READY = "E2E_LLMOCK_OK";

const client = createClient<typeof registry>(
	process.env.RIVET_ENDPOINT ?? "http://localhost:6420",
);
const ACTOR_KEY = process.env.E2E_ACTOR_KEY ?? "e2e-test";
const agent = client.vm.getOrCreate([ACTOR_KEY]);

const llmock = new LLMock({ port: LLMOCK_PORT, logLevel: "silent" });
const SECRET = "BANANA-7731";
llmock.addFixtures([
	{
		match: { predicate: () => true },
		response: { content: LLMOCK_READY },
	},
]);
const llmockUrl = await llmock.start();
console.log(`llmock: ${llmockUrl}`);

// --- Step 1: Filesystem basics ---
console.log("=== Step 1: Filesystem ===");
await agent.writeFile("/tmp/hello.txt", "Hello from Agent OS!");
const raw = (await agent.readFile("/tmp/hello.txt")) as Uint8Array;
const text = new TextDecoder().decode(raw);
console.log(`writeFile + readFile: "${text.trim()}"`);
assert(text.includes("Hello from Agent OS!"), "filesystem round-trip");

await agent.mkdir("/home/user/project");
const entries = (await agent.readdir("/home/user/project")) as string[];
console.log(
	"mkdir + readdir:",
	entries.filter((e) => e !== "." && e !== ".."),
);
console.log(
	"exists /home/user/project:",
	await agent.exists("/home/user/project"),
);
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
assert(
	cat.stdout.includes("Hello from Agent OS!"),
	"cat reads file written by writeFile",
);

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

// Create a signed preview URL for port 8080
const preview = (await agent.createSignedPreviewUrl(8080, 60)) as {
	path: string;
	token: string;
	port: number;
	expiresAt: number;
};
console.log(`Preview path: ${preview.path}`);
console.log(`Preview token: ${preview.token}`);

// Fetch through the preview proxy. getGatewayUrl() returns a routing URL that
// already carries a query string (rvt-namespace/rvt-method/rvt-crash-policy/...),
// so the preview path must be inserted into the pathname before the query rather
// than naively appended (which would land inside the rvt-crash-policy value).
const gatewayUrl = await agent.getGatewayUrl();
const previewUrlObj = new URL(gatewayUrl);
previewUrlObj.pathname =
	previewUrlObj.pathname.replace(/\/$/, "") + preview.path;
const previewUrl = previewUrlObj.toString();
console.log(`Fetching preview URL: ${previewUrl}`);
let previewResponse = new Response("", { status: 503 });
let previewBody = "";
const previewDeadline = Date.now() + 10_000;
while (Date.now() < previewDeadline) {
	previewResponse = await fetch(previewUrl);
	previewBody = await previewResponse.text();
	if (previewResponse.status === 200) break;
	await new Promise((r) => setTimeout(r, 250));
}
console.log(`Preview response: ${previewResponse.status} "${previewBody}"`);
assert(previewResponse.status === 200, "preview status 200");
assert(previewBody === "preview ok", "preview body matches");

// Clean up the server process
await agent.killProcess(serverProc.pid);

// --- Step 4: Agent session (Pi + llmock) ---
console.log("\n=== Step 4: Agent session ===");
console.log("Creating Pi agent session...");
await agent.mkdir("/home/user/.pi/agent");
await agent.writeFile(
	"/home/user/.pi/agent/models.json",
	JSON.stringify(
		{
			providers: {
				anthropic: {
					baseUrl: llmockUrl,
					apiKey: "mock-key",
				},
			},
		},
		null,
		2,
	),
);
await agent.writeFile(
	"/home/user/.pi/agent/auth.json",
	JSON.stringify(
		{
			anthropic: {
				type: "api_key",
				key: "mock-key",
			},
		},
		null,
		2,
	),
);
await agent.mkdir("/home/user/workspace");
const session = (await agent.createSession("pi", {
	cwd: "/home/user/workspace",
	env: {
		HOME: "/home/user",
		ANTHROPIC_API_KEY: "mock-key",
		ANTHROPIC_BASE_URL: llmockUrl,
		PI_CODING_AGENT_DIR: "/home/user/.pi/agent",
		PI_SKIP_VERSION_CHECK: "1",
	},
})) as { sessionId: string };
console.log(`Session created: ${session.sessionId}`);
const transcriptPath = `/root/.agentos/threads/${session.sessionId}.md`;
llmock.prependFixture({
	match: {
		predicate: (req: any) =>
			req.messages?.at?.(-1)?.role === "tool" &&
			JSON.stringify(req.messages.at(-1)).includes(SECRET),
	},
	response: { content: SECRET },
});
llmock.prependFixture({
	match: {
		predicate: (req: any) => {
			const body = JSON.stringify(req).toLowerCase();
			return (
				body.includes("after wake") &&
				body.includes(transcriptPath.toLowerCase()) &&
				!body.includes(SECRET.toLowerCase())
			);
		},
	},
	response: {
		toolCalls: [
			{
				id: "call_read_resume_transcript",
				name: "read",
				arguments: JSON.stringify({ path: transcriptPath }),
			},
		],
	},
});

// Subscribe to streaming events via WebSocket connection
const conn = agent.connect();
let initialStream = "";
conn.on("sessionEvent", (data: any) => {
	const event = data?.event ?? data;
	const params = event?.params;
	if (params?.update?.sessionUpdate === "agent_message_chunk") {
		const text = params.update.content?.text ?? "";
		initialStream += text;
		process.stdout.write(text);
	}
});

// Track VM lifecycle so we can prove the actor actually slept (VM torn down)
// before the resume prompt, and that a fresh VM was booted to serve it.
let vmShutdownCount = 0;
let vmBootedCount = 0;
let lastShutdownReason: string | undefined;
conn.on("vmShutdown", (data: any) => {
	const payload = data?.payload ?? data;
	lastShutdownReason = payload?.reason ?? lastShutdownReason;
	vmShutdownCount++;
	console.log(`\n[lifecycle] vmShutdown (reason=${lastShutdownReason})`);
});
conn.on("vmBooted", () => {
	vmBootedCount++;
	console.log(`[lifecycle] vmBooted`);
});

// Wait for WebSocket to establish
await new Promise((r) => setTimeout(r, 500));

console.log("\nSending prompt...");
const response = (await agent.sendPrompt(
	session.sessionId,
	`Reply with the exact text ${LLMOCK_READY}.`,
)) as { stopReason?: string; text?: string };
console.log(`\n\nPrompt completed: ${response?.stopReason ?? "done"}`);
assert(
	(initialStream || response?.text || "").includes(LLMOCK_READY),
	"Pi session reached host llmock",
);

// --- Step 5: Session resume across a real actor sleep/wake ---
// Plant a memorable fact, force the actor to sleep (VM torn down, live_sessions
// cleared), then resume the SAME session with a second prompt. The actor must
// reconstruct enough session state from agent_os_session_events for the prompt
// to continue after wake.
console.log("\n=== Step 5: Session resume across sleep/wake ===");

console.log(`Planting secret "${SECRET}" in session ${session.sessionId}...`);
console.log("\nResume prompt 1 (plant secret)...");
const plant = (await agent.sendPrompt(
	session.sessionId,
	`Remember this secret code for later: ${SECRET}. Just acknowledge with "ok".`,
)) as { stopReason?: string };
console.log(`\n\nPlant prompt completed: ${plant?.stopReason ?? "done"}`);

// Force a real sleep deterministically via the engine admin endpoint
// (POST /actors/{id}/sleep). An open WebSocket keeps the actor awake
// (CanSleep::ActiveConnections), so we first DISPOSE the conn, then signal sleep,
// then poll the actor record until `sleep_ts` is set -- which means the actor
// workflow tore down the VM (clearing Vars.live_sessions). This is more
// deterministic than waiting out the idle timeout.
const shutdownsBefore = vmShutdownCount;
const ENGINE = process.env.RIVET_ENDPOINT ?? "http://localhost:6420";
const NS = "default";
const ACTOR_NAME = "vm";

async function resolveActor(): Promise<any> {
	const url = `${ENGINE}/actors?namespace=${NS}&name=${ACTOR_NAME}&key=${encodeURIComponent(ACTOR_KEY)}`;
	const res = await fetch(url);
	if (!res.ok) throw new Error(`list actors failed: ${res.status}`);
	const body = (await res.json()) as { actors: any[] };
	const a =
		body.actors?.find((x) => x.destroy_ts == null) ?? body.actors?.[0];
	if (!a) throw new Error("actor not found by key");
	return a;
}

async function forceActorToSleep(label: string) {
	const actor = await resolveActor();
	console.log(`Resolved actor_id=${actor.actor_id} for ${label}`);

	console.log(
		`Signaling actor sleep via engine admin endpoint (${label})...`,
	);
	const sleepRes = await fetch(
		`${ENGINE}/actors/${actor.actor_id}/sleep?namespace=${NS}`,
		{
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: "{}",
		},
	);
	console.log(`  sleep endpoint -> HTTP ${sleepRes.status}`);
	assert(sleepRes.ok, `${label}: engine admin sleep endpoint accepted`);

	const sleepDeadline = Date.now() + 30_000;
	while (Date.now() < sleepDeadline) {
		const a = await resolveActor();
		if (a.sleep_ts != null) {
			console.log(
				`  actor asleep (sleep_ts=${a.sleep_ts}) -> VM torn down`,
			);
			return;
		}
		await new Promise((r) => setTimeout(r, 500));
	}
	assert(false, `${label}: actor actually slept (sleep_ts set)`);
}

console.log(
	"\nDisconnecting WebSocket so the actor can sleep (no active connection)...",
);
await conn.dispose();
await new Promise((r) => setTimeout(r, 500));

const actor = await resolveActor();
console.log(`Resolved actor_id=${actor.actor_id}`);

console.log("Signaling actor sleep via engine admin endpoint...");
const sleepRes = await fetch(
	`${ENGINE}/actors/${actor.actor_id}/sleep?namespace=${NS}`,
	{
		method: "POST",
		headers: { "Content-Type": "application/json" },
		body: "{}",
	},
);
console.log(`  sleep endpoint -> HTTP ${sleepRes.status}`);
assert(
	sleepRes.ok,
	"engine admin sleep endpoint accepted (VM teardown signaled)",
);

// Poll until the actor record shows sleep_ts (asleep -> VM torn down).
const sleepDeadline = Date.now() + 30_000;
let slept = false;
while (Date.now() < sleepDeadline) {
	const a = await resolveActor();
	if (a.sleep_ts != null) {
		slept = true;
		console.log(`  actor asleep (sleep_ts=${a.sleep_ts}) -> VM torn down`);
		break;
	}
	await new Promise((r) => setTimeout(r, 500));
}
assert(
	slept,
	"actor actually slept (sleep_ts set -> VM destroyed, live_sessions cleared)",
);
if (vmShutdownCount > shutdownsBefore) {
	console.log(`  also observed vmShutdown reason: ${lastShutdownReason}`);
}

// Re-open a connection and subscribe to lifecycle BEFORE the resume prompt, so we
// catch the fresh vmBooted that the wake triggers.
const bootsBeforeResume = vmBootedCount;
const conn2 = agent.connect();
let resumeBooted = false;
let recallStream = "";
conn2.on("vmBooted", () => {
	resumeBooted = true;
	console.log(`[lifecycle] vmBooted (resume wake)`);
});
conn2.on("sessionEvent", (data: any) => {
	const event = data?.event ?? data;
	const params = event?.params;
	if (params?.update?.sessionUpdate === "agent_message_chunk") {
		const t = params.update.content?.text ?? "";
		recallStream += t;
		process.stdout.write(t);
	}
});
await new Promise((r) => setTimeout(r, 500));

console.log("\nResume prompt 2 (post-wake prompt -- triggers lazy resume)...");
const recall = (await agent.sendPrompt(
	session.sessionId,
	"After wake, what secret code did I ask you to remember? Use prior session context if needed and reply with only the code.",
)) as { stopReason?: string; text?: string };
console.log(`\n\nRecall prompt completed: ${recall?.stopReason ?? "done"}`);

// A fresh VM boots to serve the resumed session. The vmBooted broadcast is a
// best-effort client-side signal (it can race the WS attach), so it is logged
// but NOT asserted: the hard teardown proof is the `slept` assertion above
// (sleep_ts -> actor Terminated), and the hard resume proof below is that the
// same session event log contains both the pre-sleep prompt and the completed
// post-wake prompt after the VM was destroyed.
if (resumeBooted || vmBootedCount > bootsBeforeResume) {
	console.log(
		"  fresh VM booted to serve resumed session (vmBooted observed)",
	);
} else {
	console.log(
		"  (vmBooted broadcast not observed on client conn -- non-fatal; teardown proven by sleep_ts, resume proven by persisted post-wake prompt)",
	);
}

// Verify both prompts are persisted in the same session event log. Combined
// with the completed post-wake `sendPrompt`, this proves lazy resume continued
// the same persisted session after VM teardown.
async function persistedPromptText(): Promise<string> {
	const events = (await agent.getSessionEvents(session.sessionId)) as any[];
	return events
		.filter((ev) => ev?.method === "user_prompt")
		.map((ev) => ev?.params?.text ?? "")
		.join("\n");
}

const prompts = await persistedPromptText();
assert(
	prompts.includes(SECRET),
	"pre-sleep prompt persisted in session event log",
);
assert(
	prompts.toLowerCase().includes("after wake"),
	"post-wake prompt persisted in same session event log",
);
assert(recall != null, "post-wake prompt completed");
const recallText = recallStream || recall?.text || "";
assert(
	recallText.includes(SECRET),
	"post-wake agent recalled prior-turn secret via resumed context",
);

await conn2.dispose();
await agent.closeSession(session.sessionId);

// --- Cleanup ---
await llmock.stop();

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

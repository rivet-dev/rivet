// Standalone test for WebSocket and SQLite through rivetkit-napi
// Run: npx tsx tests/standalone-ws-sqlite.mts
//
// Requires: engine running on localhost:6420, test-envoy on port 5051

const endpoint = "http://127.0.0.1:6420";
const token = "dev";
const namespace = "default";
const poolName = "test-envoy";

async function createActor(name: string, key: string) {
	const resp = await fetch(`${endpoint}/actors?namespace=${namespace}`, {
		method: "POST",
		headers: { Authorization: `Bearer ${token}`, "Content-Type": "application/json" },
		body: JSON.stringify({ name, key, runner_name_selector: poolName, crash_policy: "sleep" }),
	});
	const body = await resp.json();
	if (!resp.ok) throw new Error(`Create actor failed: ${resp.status} ${JSON.stringify(body)}`);
	return body.actor.actor_id as string;
}

// --- Test 1: WebSocket echo ---
async function testWebSocket() {
	console.log("\n=== WebSocket Test ===");
	const actorId = await createActor("test", `ws-${Date.now()}`);
	console.log("Actor:", actorId.slice(0, 12));

	const wsEndpoint = endpoint.replace("http://", "ws://");
	const ws = new WebSocket(`${wsEndpoint}/ws`, [
		"rivet",
		"rivet_target.actor",
		`rivet_actor.${actorId}`,
		`rivet_token.${token}`,
	]);

	const result = await new Promise<string>((resolve, reject) => {
		const timeout = setTimeout(() => reject(new Error("WebSocket timeout")), 10_000);
		ws.addEventListener("open", () => ws.send("hello"));
		ws.addEventListener("message", (ev) => {
			clearTimeout(timeout);
			ws.close();
			resolve(ev.data as string);
		});
		ws.addEventListener("error", (e) => {
			clearTimeout(timeout);
			reject(new Error(`WebSocket error: ${(e as any)?.message ?? "unknown"}`));
		});
	});

	console.log("Response:", result);
	console.log(result === "Echo: hello" ? "✓ PASS" : `✗ FAIL (expected "Echo: hello")`);
	return result === "Echo: hello";
}

// --- Test 2: HTTP action (baseline) ---
async function testAction() {
	console.log("\n=== Action Test ===");
	const actorId = await createActor("test", `act-${Date.now()}`);
	console.log("Actor:", actorId.slice(0, 12));

	const resp = await fetch(`${endpoint}/ping`, {
		headers: {
			"X-Rivet-Token": token,
			"X-Rivet-Target": "actor",
			"X-Rivet-Actor": actorId,
		},
	});
	const body = await resp.text();
	console.log(`HTTP ${resp.status}: ${body.slice(0, 60)}`);
	console.log(resp.ok ? "✓ PASS" : "✗ FAIL");
	return resp.ok;
}

// --- Run ---
let passed = 0;
let failed = 0;

try {
	(await testAction()) ? passed++ : failed++;
} catch (e) {
	console.log("✗ FAIL:", (e as Error).message);
	failed++;
}

try {
	(await testWebSocket()) ? passed++ : failed++;
} catch (e) {
	console.log("✗ FAIL:", (e as Error).message);
	failed++;
}

console.log(`\n${passed} passed, ${failed} failed`);
process.exit(failed > 0 ? 1 : 0);

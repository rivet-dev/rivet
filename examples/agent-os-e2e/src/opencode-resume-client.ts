import { LLMock } from "@copilotkit/llmock";
import { createClient } from "rivetkit/client";
import type { registry } from "./opencode-resume-server.ts";

type LlmockMessage = {
	role?: string;
	content?: string | null;
};

const LLMOCK_PORT = Number(process.env.E2E_LLMOCK_PORT ?? "41235");
const client = createClient<typeof registry>(
	process.env.RIVET_ENDPOINT ?? "http://localhost:6420",
);
const ACTOR_KEY = process.env.E2E_ACTOR_KEY ?? `opencode-resume-${Date.now()}`;
const agent = client.vm.getOrCreate([ACTOR_KEY]);
const llmock = new LLMock({ port: LLMOCK_PORT, logLevel: "silent" });
const llmockUrl = await llmock.start();

const ENGINE = process.env.RIVET_ENDPOINT ?? "http://localhost:6420";
const NS = "default";
const ACTOR_NAME = "vm";

console.log(`llmock: ${llmockUrl}`);
console.log(`actor key: ${ACTOR_KEY}`);

try {
	await runNativeResume();
	await runMissingStoreFallback();
	console.log("\n=== Results ===");
	console.log("OpenCode resume checks passed!");
} finally {
	await llmock.stop();
}

async function runNativeResume() {
	console.log("\n=== OpenCode native resume across sleep/wake ===");
	const token = "ORCHID-2718";
	const firstPrompt = `Remember the native OpenCode token: ${token}.`;
	const secondPrompt = "What native OpenCode token did I give you earlier?";

	llmock.prependFixture({
		match: {
			predicate: (req: unknown) =>
				hasUserMessageContaining(req, secondPrompt),
		},
		response: { content: `The token was ${token}.` },
	});
	llmock.prependFixture({
		match: {
			predicate: (req: unknown) =>
				hasUserMessageContaining(req, firstPrompt),
		},
		response: { content: `I will remember ${token}.` },
	});

	const home = `/tmp/opencode-native-${crypto.randomUUID()}`;
	const workspace = `/tmp/opencode-native-workspace-${crypto.randomUUID()}`;
	await createOpenCodeHome(home);
	await mkdirp(workspace);

	const { sessionId } = (await agent.createSession("opencode", {
		cwd: workspace,
		env: openCodeEnv(home),
	})) as { sessionId: string };
	console.log(`session: ${sessionId}`);

	await agent.sendPrompt(sessionId, firstPrompt);
	await forceActorToSleep("OpenCode native resume");

	const recall = (await agent.sendPrompt(sessionId, secondPrompt)) as {
		text?: string;
	};
	const secondRequest = llmock
		.getRequests()
		.find((req: unknown) => hasUserMessageContaining(req, secondPrompt));
	assert(
		secondRequest != null,
		"second OpenCode native LLM request observed",
	);
	assert(
		hasUserMessageContaining(secondRequest, firstPrompt),
		"native session/load preserved prior prompt context after real sleep/wake",
	);
	assert(
		!hasUserMessageContaining(
			secondRequest,
			"You are continuing an earlier session",
		),
		"native session/load did not inject fallback transcript preamble",
	);
	assert(
		(recall.text ?? "").includes(token),
		"native post-wake response used preserved context",
	);
	await agent.closeSession(sessionId);
}

async function runMissingStoreFallback() {
	console.log("\n=== OpenCode missing-store fallback across sleep/wake ===");
	const token = "FALLBACK-4930";
	const firstPrompt = `Remember the fallback OpenCode token: ${token}.`;
	const secondPrompt =
		"After missing-store wake, continue using the transcript. What fallback OpenCode token did I give you?";

	llmock.prependFixture({
		match: {
			predicate: (req: unknown) =>
				hasUserMessageContaining(req, secondPrompt),
		},
		response: { content: `The token was ${token}.` },
	});
	llmock.prependFixture({
		match: {
			predicate: (req: unknown) =>
				hasUserMessageContaining(req, firstPrompt),
		},
		response: { content: `I will remember ${token}.` },
	});

	const home = `/tmp/opencode-fallback-${crypto.randomUUID()}`;
	const workspace = `/tmp/opencode-fallback-workspace-${crypto.randomUUID()}`;
	await createOpenCodeHome(home);
	await mkdirp(workspace);

	const { sessionId } = (await agent.createSession("opencode", {
		cwd: workspace,
		env: openCodeEnv(home),
	})) as { sessionId: string };
	console.log(`session: ${sessionId}`);

	await agent.sendPrompt(sessionId, firstPrompt);
	await agent.deleteFile(`${home}/.local/share/opencode`, {
		recursive: true,
	});
	await forceActorToSleep("OpenCode missing-store fallback");

	const recall = (await agent.sendPrompt(sessionId, secondPrompt)) as {
		text?: string;
	};
	const transcriptPath = `/root/.agentos/threads/${sessionId}.md`;
	const secondRequest = llmock
		.getRequests()
		.find((req: unknown) => hasUserMessageContaining(req, secondPrompt));
	assert(
		secondRequest != null,
		"second OpenCode fallback LLM request observed",
	);
	assert(
		hasUserMessageContaining(
			secondRequest,
			"You are continuing an earlier session",
		),
		"missing-store resume injected fallback transcript preamble",
	);
	assert(
		hasUserMessageContaining(secondRequest, transcriptPath),
		"fallback preamble pointed at the stable transcript path",
	);
	assert(
		(recall.text ?? "").includes(token),
		"fallback post-wake response completed after transcript fallback",
	);
	await agent.closeSession(sessionId);
}

async function resolveActor(): Promise<any> {
	const url = `${ENGINE}/actors?namespace=${NS}&name=${ACTOR_NAME}&key=${encodeURIComponent(ACTOR_KEY)}`;
	const res = await fetch(url);
	if (!res.ok) throw new Error(`list actors failed: ${res.status}`);
	const body = (await res.json()) as { actors: any[] };
	const actor =
		body.actors?.find((candidate) => candidate.destroy_ts == null) ??
		body.actors?.[0];
	if (!actor) throw new Error("actor not found by key");
	return actor;
}

async function forceActorToSleep(label: string) {
	const actor = await resolveActor();
	console.log(`Resolved actor_id=${actor.actor_id} for ${label}`);

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
		const sleptActor = await resolveActor();
		if (sleptActor.sleep_ts != null) {
			console.log(
				`  actor asleep (sleep_ts=${sleptActor.sleep_ts}) -> VM torn down`,
			);
			return;
		}
		await new Promise((resolve) => setTimeout(resolve, 500));
	}
	assert(false, `${label}: actor actually slept (sleep_ts set)`);
}

async function mkdirp(path: string) {
	const parts = path.split("/").filter(Boolean);
	let current = "";
	for (const part of parts) {
		current += `/${part}`;
		try {
			await agent.mkdir(current);
		} catch {
			// Directory already exists.
		}
	}
}

async function createOpenCodeHome(homeDir: string) {
	await mkdirp(`${homeDir}/.config/opencode`);
	await agent.writeFile(
		`${homeDir}/.config/opencode/opencode.json`,
		JSON.stringify(
			{
				$schema: "https://opencode.ai/config.json",
				autoupdate: false,
				share: "disabled",
				snapshot: false,
				model: "anthropic/claude-sonnet-4-20250514",
				provider: {
					anthropic: {
						options: {
							baseURL: `${llmockUrl}/v1`,
						},
					},
				},
			},
			null,
			2,
		),
	);
}

function openCodeEnv(homeDir: string) {
	return {
		HOME: homeDir,
		ANTHROPIC_API_KEY: "mock-key",
	};
}

function getLlmockMessages(req: unknown): LlmockMessage[] {
	const directMessages = (req as { messages?: LlmockMessage[] }).messages;
	if (Array.isArray(directMessages)) return directMessages;

	const bodyMessages = (req as { body?: { messages?: LlmockMessage[] } }).body
		?.messages;
	return Array.isArray(bodyMessages) ? bodyMessages : [];
}

function hasUserMessageContaining(req: unknown, expected: string): boolean {
	return getLlmockMessages(req).some(
		(message) =>
			message.role === "user" &&
			typeof message.content === "string" &&
			message.content.includes(expected),
	);
}

function assert(condition: boolean, label: string) {
	if (!condition) {
		console.error(`FAILED: ${label}`);
		process.exit(1);
	}
	console.log(`  OK: ${label}`);
}

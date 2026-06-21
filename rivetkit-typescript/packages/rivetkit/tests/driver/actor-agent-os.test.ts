import { createRequire } from "node:module";
import { describe, expect, test } from "vitest";
import { describeDriverMatrix } from "./shared-matrix";
import { setupDriverTest } from "./shared-utils";

const DRIVER_API_TOKEN = "dev";
const require = createRequire(import.meta.url);
const hasAgentOsCore = (() => {
	try {
		require.resolve("@rivet-dev/agent-os-core");
		return true;
	} catch {
		return false;
	}
})();

async function forceActorSleep(input: {
	endpoint: string;
	namespace: string;
	actorId: string;
}) {
	const response = await fetch(
		`${input.endpoint}/actors/${encodeURIComponent(input.actorId)}/sleep?namespace=${encodeURIComponent(input.namespace)}`,
		{
			method: "POST",
			headers: {
				Authorization: `Bearer ${DRIVER_API_TOKEN}`,
				"Content-Type": "application/json",
			},
			body: "{}",
		},
	);
	if (!response.ok) {
		throw new Error(
			`failed to force actor sleep: ${response.status} ${await response.text()}`,
		);
	}
}

async function waitForActorSleep(input: {
	endpoint: string;
	namespace: string;
	actorId: string;
	timeoutMs: number;
}) {
	const deadline = Date.now() + input.timeoutMs;
	while (Date.now() < deadline) {
		const response = await fetch(
			`${input.endpoint}/actors?actor_ids=${encodeURIComponent(input.actorId)}&namespace=${encodeURIComponent(input.namespace)}`,
			{
				headers: {
					Authorization: `Bearer ${DRIVER_API_TOKEN}`,
				},
			},
		);
		expect(response.ok).toBe(true);
		const body = (await response.json()) as {
			actors: Array<{ sleep_ts?: number | null }>;
		};
		if (body.actors[0]?.sleep_ts) {
			return;
		}
		await new Promise((resolve) => setTimeout(resolve, 500));
	}
	throw new Error(`timed out waiting for actor ${input.actorId} to sleep`);
}

async function waitForSessionEvents(input: {
	actor: any;
	sessionId: string;
	predicate: (events: any[]) => boolean;
	timeoutMs: number;
}): Promise<any[]> {
	const deadline = Date.now() + input.timeoutMs;
	let lastEvents: any[] = [];
	while (Date.now() < deadline) {
		lastEvents = (await input.actor.getSessionEvents(
			input.sessionId,
		)) as any[];
		if (input.predicate(lastEvents)) {
			return lastEvents;
		}
		await new Promise((resolve) => setTimeout(resolve, 250));
	}
	throw new Error(
		`timed out waiting for session events; last events=${JSON.stringify(lastEvents)}`,
	);
}

function hasUpdate(events: any[], kind: string): boolean {
	return events.some((event) => {
		const update = event?.params?.update;
		return update?.sessionUpdate === kind;
	});
}

function expectPromptBeforeFollowingUpdate(events: any[], promptText: string) {
	const promptIndex = events.findIndex(
		(event) =>
			event?.method === "user_prompt" &&
			event?.params?.text === promptText,
	);
	expect(promptIndex).toBeGreaterThanOrEqual(0);

	const updateIndex = events.findIndex(
		(event, index) =>
			index > promptIndex && event?.method === "session/update",
	);
	expect(updateIndex).toBeGreaterThan(promptIndex);
}

function parsePromptBlocks(
	text: string,
): Array<{ type: string; text: string }> {
	return JSON.parse(text) as Array<{ type: string; text: string }>;
}

function parseProbeBlock(blocks: Array<{ type: string; text: string }>) {
	const probe = blocks.find((block) => block.type === "probe");
	expect(probe).toBeDefined();
	return JSON.parse(probe!.text) as { cwd?: string; env?: string };
}

describeDriverMatrix("Actor Agent Os", (driverTestConfig) => {
	describe.skipIf(driverTestConfig.skip?.agentOs || !hasAgentOsCore)(
		"Actor agentOS Tests",
		() => {
			// --- Filesystem ---

			test("writeFile and readFile round-trip", async (c) => {
				const { client } = await setupDriverTest(c, {
					...driverTestConfig,
					useRealTimers: true,
				});
				const actor = client.agentOsTestActor.getOrCreate([
					`fs-${crypto.randomUUID()}`,
				]);

				await actor.writeFile("/home/user/hello.txt", "hello world");
				const data = await actor.readFile("/home/user/hello.txt");
				expect(new TextDecoder().decode(data)).toBe("hello world");
			}, 60_000);

			test.skipIf(driverTestConfig.skip?.sleep)(
				"filesystem survives sleep and wake",
				async (c) => {
					const { client, endpoint, namespace } =
						await setupDriverTest(c, {
							...driverTestConfig,
							useRealTimers: true,
						});
					const actorKey = `fs-sleep-${crypto.randomUUID()}`;
					const path = "/home/user/sleep-persist.txt";
					const actor = client.agentOsTestActor.getOrCreate([
						actorKey,
					]);

					await actor.writeFile(path, "durable hello");
					const actorId = await actor.resolve();
					await forceActorSleep({ endpoint, namespace, actorId });
					await waitForActorSleep({
						endpoint,
						namespace,
						actorId,
						timeoutMs: 30_000,
					});

					const actorAfterWake = client.agentOsTestActor.getOrCreate([
						actorKey,
					]);
					const data = await actorAfterWake.readFile(path);
					expect(new TextDecoder().decode(data)).toBe(
						"durable hello",
					);
				},
				90_000,
			);

			test("session capture persists tool calls and message chunks", async (c) => {
				const { client } = await setupDriverTest(c, {
					...driverTestConfig,
					useRealTimers: true,
				});
				const actor = client.agentOsTestActor.getOrCreate([
					`session-capture-${crypto.randomUUID()}`,
				]);

				const { sessionId } = (await actor.createSession("opencode", {
					env: { MOCK_RESUME_SCENARIO: "native" },
				})) as { sessionId: string };
				const result = (await actor.sendPrompt(
					sessionId,
					"capture both update kinds",
				)) as { text: string };
				expect(parsePromptBlocks(result.text).at(-1)?.text).toBe(
					"capture both update kinds",
				);

				const events = await waitForSessionEvents({
					actor,
					sessionId,
					timeoutMs: 10_000,
					predicate: (events) =>
						hasUpdate(events, "tool_call") &&
						hasUpdate(events, "agent_message_chunk"),
				});
				expect(hasUpdate(events, "tool_call")).toBe(true);
				expect(hasUpdate(events, "agent_message_chunk")).toBe(true);
				expectPromptBeforeFollowingUpdate(
					events,
					"capture both update kinds",
				);
			}, 90_000);

			test.skipIf(driverTestConfig.skip?.sleep)(
				"session fallback resume survives real sleep/wake with external id remap",
				async (c) => {
					const { client, endpoint, namespace } =
						await setupDriverTest(c, {
							...driverTestConfig,
							useRealTimers: true,
						});
					const actorKey = `session-fallback-${crypto.randomUUID()}`;
					const actor = client.agentOsTestActor.getOrCreate([
						actorKey,
					]);
					const cwd = `/home/user/fallback-cwd-${crypto.randomUUID()}`;
					const envProbe = `fallback-env-${crypto.randomUUID()}`;
					await actor.mkdir(cwd);

					const { sessionId } = (await actor.createSession(
						"opencode",
						{
							cwd,
							env: {
								MOCK_RESUME_SCENARIO: "fallthrough",
								MOCK_CWD_ENV_PROBE: envProbe,
							},
						},
					)) as { sessionId: string };
					await actor.sendPrompt(sessionId, "remember alpha");
					await waitForSessionEvents({
						actor,
						sessionId,
						timeoutMs: 10_000,
						predicate: (events) =>
							hasUpdate(events, "tool_call") &&
							hasUpdate(events, "agent_message_chunk"),
					});

					const actorId = await actor.resolve();
					await forceActorSleep({ endpoint, namespace, actorId });
					await waitForActorSleep({
						endpoint,
						namespace,
						actorId,
						timeoutMs: 30_000,
					});

					const actorAfterWake = client.agentOsTestActor.getOrCreate([
						actorKey,
					]);
					const resumed = (await actorAfterWake.sendPrompt(
						sessionId,
						"continue after fallback",
					)) as { text: string };
					const blocks = parsePromptBlocks(resumed.text);
					expect(blocks).toHaveLength(3);
					expect(blocks[0].text).toContain(
						"You are continuing an earlier session",
					);
					expect(blocks[0].text).toContain(
						`/root/.agentos/threads/${sessionId}.md`,
					);
					expect(blocks[1].text).toBe("continue after fallback");
					expect(parseProbeBlock(blocks)).toEqual({
						cwd,
						env: envProbe,
					});

					const events = await waitForSessionEvents({
						actor: actorAfterWake,
						sessionId,
						timeoutMs: 10_000,
						predicate: (events) =>
							events.filter(
								(event) => event?.method === "user_prompt",
							).length >= 2 &&
							hasUpdate(events, "tool_call") &&
							hasUpdate(events, "agent_message_chunk"),
					});
					expect(
						events.some((event) => event?.method === "user_prompt"),
					).toBe(true);
					expectPromptBeforeFollowingUpdate(
						events,
						"continue after fallback",
					);
				},
				120_000,
			);

			test.skipIf(driverTestConfig.skip?.sleep)(
				"session native resume survives real sleep/wake without preamble",
				async (c) => {
					const { client, endpoint, namespace } =
						await setupDriverTest(c, {
							...driverTestConfig,
							useRealTimers: true,
						});
					const actorKey = `session-native-${crypto.randomUUID()}`;
					const actor = client.agentOsTestActor.getOrCreate([
						actorKey,
					]);
					const cwd = `/home/user/native-cwd-${crypto.randomUUID()}`;
					const envProbe = `native-env-${crypto.randomUUID()}`;
					await actor.mkdir(cwd);

					const { sessionId } = (await actor.createSession(
						"opencode",
						{
							cwd,
							env: {
								MOCK_RESUME_SCENARIO: "native",
								MOCK_CWD_ENV_PROBE: envProbe,
							},
						},
					)) as { sessionId: string };
					await actor.sendPrompt(sessionId, "before native sleep");

					const actorId = await actor.resolve();
					await forceActorSleep({ endpoint, namespace, actorId });
					await waitForActorSleep({
						endpoint,
						namespace,
						actorId,
						timeoutMs: 30_000,
					});

					const actorAfterWake = client.agentOsTestActor.getOrCreate([
						actorKey,
					]);
					const resumed = (await actorAfterWake.sendPrompt(
						sessionId,
						"continue after native",
					)) as { text: string };
					const blocks = parsePromptBlocks(resumed.text);
					expect(blocks).toHaveLength(2);
					expect(blocks[0].text).toBe("continue after native");
					expect(parseProbeBlock(blocks)).toEqual({
						cwd,
						env: envProbe,
					});
				},
				120_000,
			);

			test.skipIf(driverTestConfig.skip?.sleep)(
				"closeSession removes persisted session after sleep before lazy resume",
				async (c) => {
					const { client, endpoint, namespace } =
						await setupDriverTest(c, {
							...driverTestConfig,
							useRealTimers: true,
						});
					const actorKey = `session-close-slept-${crypto.randomUUID()}`;
					const actor = client.agentOsTestActor.getOrCreate([
						actorKey,
					]);

					const { sessionId } = (await actor.createSession(
						"opencode",
						{
							env: { MOCK_RESUME_SCENARIO: "native" },
						},
					)) as { sessionId: string };
					await actor.sendPrompt(sessionId, "before close sleep");

					const actorId = await actor.resolve();
					await forceActorSleep({ endpoint, namespace, actorId });
					await waitForActorSleep({
						endpoint,
						namespace,
						actorId,
						timeoutMs: 30_000,
					});

					const actorAfterWake = client.agentOsTestActor.getOrCreate([
						actorKey,
					]);
					await actorAfterWake.closeSession(sessionId);
					const sessions =
						(await actorAfterWake.listPersistedSessions()) as Array<{
							sessionId: string;
						}>;
					expect(
						sessions.some(
							(session) => session.sessionId === sessionId,
						),
					).toBe(false);
					expect(
						await actorAfterWake.getSessionEvents(sessionId),
					).toEqual([]);
				},
				120_000,
			);

			test("mkdir and readdir", async (c) => {
				const { client } = await setupDriverTest(c, {
					...driverTestConfig,
					useRealTimers: true,
				});
				const actor = client.agentOsTestActor.getOrCreate([
					`dir-${crypto.randomUUID()}`,
				]);

				await actor.mkdir("/home/user/subdir");
				await actor.writeFile("/home/user/subdir/a.txt", "a");
				await actor.writeFile("/home/user/subdir/b.txt", "b");
				const entries = await actor.readdir("/home/user/subdir");
				const filtered = entries.filter(
					(e: string) => e !== "." && e !== "..",
				);
				expect(filtered.sort()).toEqual(["a.txt", "b.txt"]);
			}, 60_000);

			test("stat returns file metadata", async (c) => {
				const { client } = await setupDriverTest(c, {
					...driverTestConfig,
					useRealTimers: true,
				});
				const actor = client.agentOsTestActor.getOrCreate([
					`stat-${crypto.randomUUID()}`,
				]);

				await actor.writeFile("/home/user/stat-test.txt", "content");
				const s = await actor.stat("/home/user/stat-test.txt");
				expect(s.isDirectory).toBe(false);
				expect(s.size).toBe(7);
			}, 60_000);

			test("exists returns true for existing file", async (c) => {
				const { client } = await setupDriverTest(c, {
					...driverTestConfig,
					useRealTimers: true,
				});
				const actor = client.agentOsTestActor.getOrCreate([
					`exists-${crypto.randomUUID()}`,
				]);

				await actor.writeFile("/home/user/exists.txt", "x");
				expect(await actor.exists("/home/user/exists.txt")).toBe(true);
				expect(await actor.exists("/home/user/nope.txt")).toBe(false);
			}, 60_000);

			test("move renames a file", async (c) => {
				const { client } = await setupDriverTest(c, {
					...driverTestConfig,
					useRealTimers: true,
				});
				const actor = client.agentOsTestActor.getOrCreate([
					`move-${crypto.randomUUID()}`,
				]);

				await actor.writeFile("/home/user/old.txt", "data");
				await actor.move("/home/user/old.txt", "/home/user/new.txt");
				expect(await actor.exists("/home/user/old.txt")).toBe(false);
				expect(await actor.exists("/home/user/new.txt")).toBe(true);
			}, 60_000);

			test("deleteFile removes a file", async (c) => {
				const { client } = await setupDriverTest(c, {
					...driverTestConfig,
					useRealTimers: true,
				});
				const actor = client.agentOsTestActor.getOrCreate([
					`del-${crypto.randomUUID()}`,
				]);

				await actor.writeFile("/home/user/todelete.txt", "gone");
				await actor.deleteFile("/home/user/todelete.txt");
				expect(await actor.exists("/home/user/todelete.txt")).toBe(
					false,
				);
			}, 60_000);

			test("writeFiles and readFiles batch operations", async (c) => {
				const { client } = await setupDriverTest(c, {
					...driverTestConfig,
					useRealTimers: true,
				});
				const actor = client.agentOsTestActor.getOrCreate([
					`batch-${crypto.randomUUID()}`,
				]);

				const writeResults = await actor.writeFiles([
					{ path: "/home/user/batch-a.txt", content: "aaa" },
					{ path: "/home/user/batch-b.txt", content: "bbb" },
				]);
				expect(writeResults.every((r: any) => r.success)).toBe(true);

				const readResults = await actor.readFiles([
					"/home/user/batch-a.txt",
					"/home/user/batch-b.txt",
				]);
				expect(new TextDecoder().decode(readResults[0].content)).toBe(
					"aaa",
				);
				expect(new TextDecoder().decode(readResults[1].content)).toBe(
					"bbb",
				);
			}, 60_000);

			// Partial-failure verification for the batch DTO mapping.
			// `BatchReadResultDto` uses `Option<ByteBuf>` content and
			// `Option<String>` error, both `skip_serializing_if`. A bug
			// where the partial shape doesn't make it across the encoding
			// wire (e.g. None elided incorrectly, error string not
			// surfaced) would be silent without this test.
			test("readFiles surfaces per-entry error for missing paths", async (c) => {
				const { client } = await setupDriverTest(c, {
					...driverTestConfig,
					useRealTimers: true,
				});
				const actor = client.agentOsTestActor.getOrCreate([
					`partial-${crypto.randomUUID()}`,
				]);

				await actor.writeFile("/home/user/exists.txt", "present");

				const results = await actor.readFiles([
					"/home/user/exists.txt",
					"/home/user/does-not-exist.txt",
				]);

				expect(results).toHaveLength(2);
				// Successful entry: content present, no error field.
				expect(results[0].path).toBe("/home/user/exists.txt");
				expect(new TextDecoder().decode(results[0].content)).toBe(
					"present",
				);
				expect(results[0].error).toBeUndefined();
				// Failed entry: no content, error string surfaced.
				expect(results[1].path).toBe("/home/user/does-not-exist.txt");
				expect(results[1].content).toBeUndefined();
				expect(typeof results[1].error).toBe("string");
				expect(results[1].error?.length).toBeGreaterThan(0);
			}, 60_000);

			test("readdirRecursive lists nested files", async (c) => {
				const { client } = await setupDriverTest(c, {
					...driverTestConfig,
					useRealTimers: true,
				});
				const actor = client.agentOsTestActor.getOrCreate([
					`recursive-${crypto.randomUUID()}`,
				]);

				await actor.mkdir("/home/user/rdir");
				await actor.mkdir("/home/user/rdir/sub");
				await actor.writeFile("/home/user/rdir/top.txt", "t");
				await actor.writeFile("/home/user/rdir/sub/deep.txt", "d");
				const entries = await actor.readdirRecursive("/home/user/rdir");
				const paths = entries.map((e: any) => e.path);
				expect(paths).toContain("/home/user/rdir/top.txt");
				expect(paths).toContain("/home/user/rdir/sub");
				expect(paths).toContain("/home/user/rdir/sub/deep.txt");
			}, 60_000);

			// --- Process execution ---

			test("exec runs a command and returns output", async (c) => {
				const { client } = await setupDriverTest(c, {
					...driverTestConfig,
					useRealTimers: true,
				});
				const actor = client.agentOsTestActor.getOrCreate([
					`exec-${crypto.randomUUID()}`,
				]);

				const result = await actor.exec("echo hello");
				expect(result.exitCode).toBe(0);
				expect(result.stdout.trim()).toBe("hello");
			}, 60_000);

			test("spawn and waitProcess", async (c) => {
				const { client } = await setupDriverTest(c, {
					...driverTestConfig,
					useRealTimers: true,
				});
				const actor = client.agentOsTestActor.getOrCreate([
					`spawn-${crypto.randomUUID()}`,
				]);

				// Write a script that exits with code 42.
				await actor.writeFile("/tmp/exit42.js", "process.exit(42);");

				const { pid } = await actor.spawn("node", ["/tmp/exit42.js"]);
				expect(typeof pid).toBe("number");

				const exitCode = await actor.waitProcess(pid);
				expect(exitCode).toBe(42);
			}, 60_000);

			test("listProcesses returns spawned processes", async (c) => {
				const { client } = await setupDriverTest(c, {
					...driverTestConfig,
					useRealTimers: true,
				});
				const actor = client.agentOsTestActor.getOrCreate([
					`list-proc-${crypto.randomUUID()}`,
				]);

				// Write a long-running script.
				await actor.writeFile(
					"/tmp/long.js",
					"setTimeout(() => {}, 30000);",
				);

				const { pid } = await actor.spawn("node", ["/tmp/long.js"]);
				const procs = await actor.listProcesses();
				expect(procs.some((p: any) => p.pid === pid)).toBe(true);

				await actor.killProcess(pid);
			}, 60_000);

			test("killProcess terminates a running process", async (c) => {
				const { client } = await setupDriverTest(c, {
					...driverTestConfig,
					useRealTimers: true,
				});
				const actor = client.agentOsTestActor.getOrCreate([
					`kill-${crypto.randomUUID()}`,
				]);

				await actor.writeFile(
					"/tmp/hang.js",
					"setTimeout(() => {}, 60000);",
				);

				const { pid } = await actor.spawn("node", ["/tmp/hang.js"]);
				await actor.killProcess(pid);
				const exitCode = await actor.waitProcess(pid);
				// SIGKILL results in non-zero exit code.
				expect(exitCode).not.toBe(0);
			}, 60_000);

			// --- Network ---

			test("vmFetch proxies request to VM service", async (c) => {
				const { client } = await setupDriverTest(c, {
					...driverTestConfig,
					useRealTimers: true,
				});
				const actor = client.agentOsTestActor.getOrCreate([
					`fetch-${crypto.randomUUID()}`,
				]);

				// Write and spawn a simple HTTP server inside the VM.
				await actor.writeFile(
					"/tmp/server.js",
					`
const http = require("http");
const server = http.createServer((req, res) => {
  res.writeHead(200, { "Content-Type": "text/plain" });
  res.end("vm-response");
});
server.listen(9876, "127.0.0.1", () => {
  console.log("listening");
});
`,
				);
				await actor.spawn("node", ["/tmp/server.js"]);

				// Wait for server to start.
				await new Promise((r) => setTimeout(r, 2000));

				const result = await actor.vmFetch(
					9876,
					"http://127.0.0.1:9876/test",
				);
				expect(result.status).toBe(200);
				expect(new TextDecoder().decode(result.body)).toBe(
					"vm-response",
				);
			}, 60_000);

			// --- Cron ---

			test("scheduleCron and listCronJobs", async (c) => {
				const { client } = await setupDriverTest(c, {
					...driverTestConfig,
					useRealTimers: true,
				});
				const actor = client.agentOsTestActor.getOrCreate([
					`cron-${crypto.randomUUID()}`,
				]);

				const { id } = await actor.scheduleCron({
					schedule: "* * * * *",
					action: { type: "exec", command: "echo cron-tick" },
				});
				expect(typeof id).toBe("string");

				const jobs = await actor.listCronJobs();
				expect(jobs.some((j: any) => j.id === id)).toBe(true);

				await actor.cancelCronJob(id);
				const jobsAfter = await actor.listCronJobs();
				expect(jobsAfter.some((j: any) => j.id === id)).toBe(false);
			}, 60_000);
		},
	);
});

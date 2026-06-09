import { createRequire } from "node:module";
import { describe, expect, test } from "vitest";
import { describeDriverMatrix } from "./shared-matrix";
import { setupDriverTest } from "./shared-utils";

const require = createRequire(import.meta.url);
const hasAgentOsCore = (() => {
	try {
		require.resolve("@rivet-dev/agent-os-core");
		return true;
	} catch {
		return false;
	}
})();

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

			test("getProcess returns info for a spawned process", async (c) => {
				const { client } = await setupDriverTest(c, {
					...driverTestConfig,
					useRealTimers: true,
				});
				const actor = client.agentOsTestActor.getOrCreate([
					`getproc-${crypto.randomUUID()}`,
				]);

				await actor.writeFile(
					"/tmp/long-get.js",
					"setTimeout(() => {}, 30000);",
				);
				const { pid } = await actor.spawn("node", ["/tmp/long-get.js"]);

				const info = await actor.getProcess(pid);
				expect(info.pid).toBe(pid);
				expect(info.command).toBe("node");
				expect(info.args).toEqual(["/tmp/long-get.js"]);
				expect(info.running).toBe(true);
				// Rust `Option::None` serializes to `null` (not `undefined`).
				expect(info.exitCode).toBeNull();

				await actor.killProcess(pid);
			}, 60_000);

			test("stopProcess gracefully terminates a process", async (c) => {
				const { client } = await setupDriverTest(c, {
					...driverTestConfig,
					useRealTimers: true,
				});
				const actor = client.agentOsTestActor.getOrCreate([
					`stop-${crypto.randomUUID()}`,
				]);

				await actor.writeFile(
					"/tmp/hang-stop.js",
					"setTimeout(() => {}, 60000);",
				);
				const { pid } = await actor.spawn("node", ["/tmp/hang-stop.js"]);

				await actor.stopProcess(pid);
				const exitCode = await actor.waitProcess(pid);
				// SIGTERM results in non-zero exit code (Node default: 143).
				expect(exitCode).not.toBe(0);
			}, 60_000);

			test("allProcesses snapshot includes a spawned process", async (c) => {
				const { client } = await setupDriverTest(c, {
					...driverTestConfig,
					useRealTimers: true,
				});
				const actor = client.agentOsTestActor.getOrCreate([
					`all-${crypto.randomUUID()}`,
				]);

				await actor.writeFile(
					"/tmp/long-all.js",
					"setTimeout(() => {}, 30000);",
				);
				const { pid } = await actor.spawn("node", ["/tmp/long-all.js"]);

				const procs = await actor.allProcesses();
				expect(Array.isArray(procs)).toBe(true);
				// Our spawned process should appear; correlation by display
				// pid is what `all_processes` provides.
				expect(procs.some((p: any) => p.pid === pid)).toBe(true);

				await actor.killProcess(pid);
			}, 60_000);

			test("processTree returns a forest with the spawned process", async (c) => {
				const { client } = await setupDriverTest(c, {
					...driverTestConfig,
					useRealTimers: true,
				});
				const actor = client.agentOsTestActor.getOrCreate([
					`tree-${crypto.randomUUID()}`,
				]);

				await actor.writeFile(
					"/tmp/long-tree.js",
					"setTimeout(() => {}, 30000);",
				);
				const { pid } = await actor.spawn("node", ["/tmp/long-tree.js"]);

				const tree = await actor.processTree();
				expect(Array.isArray(tree)).toBe(true);
				// The spawned process must appear somewhere in the forest
				// (root or descendant). Flatten and search.
				const flatten = (nodes: any[]): any[] =>
					nodes.flatMap((n) => [n, ...flatten(n.children ?? [])]);
				const all = flatten(tree);
				expect(all.some((n) => n.pid === pid)).toBe(true);

				await actor.killProcess(pid);
			}, 60_000);

			test("writeProcessStdin + closeProcessStdin pipe data and trigger exit", async (c) => {
				const { client } = await setupDriverTest(c, {
					...driverTestConfig,
					useRealTimers: true,
				});
				const actor = client.agentOsTestActor.getOrCreate([
					`stdin-${crypto.randomUUID()}`,
				]);

				// Script reads stdin, counts the bytes, exits with that
				// count as its code. So sending "hello" (5 bytes) and
				// closing stdin produces exit code 5.
				await actor.writeFile(
					"/tmp/stdin-counter.js",
					[
						"let n = 0;",
						"process.stdin.on('data', (chunk) => { n += chunk.length; });",
						"process.stdin.on('end', () => { process.exit(n); });",
					].join("\n"),
				);
				const { pid } = await actor.spawn("node", [
					"/tmp/stdin-counter.js",
				]);

				await actor.writeProcessStdin(pid, "hello");
				await actor.closeProcessStdin(pid);

				const exitCode = await actor.waitProcess(pid);
				expect(exitCode).toBe(5);
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

			// --- Session (gated: requires ANTHROPIC_API_KEY) ---

			// Verifies the full agent-session chain: JS → engine → NAPI →
			// Rust dispatch → agent-os-client → sidecar → Pi CLI →
			// Anthropic API → reply back through every layer. Gated so CI
			// without credentials doesn't run it.
			test.skipIf(!process.env.ANTHROPIC_API_KEY)(
				"createSession + sendPrompt round-trip against live Pi agent",
				async (c) => {
					const { client } = await setupDriverTest(c, {
						...driverTestConfig,
						useRealTimers: true,
					});
					const actor = client.agentOsTestActor.getOrCreate([
						`session-${crypto.randomUUID()}`,
					]);

					// Mirrors the setup of the agent-os-client's own Pi
					// e2e test (crates/client/tests/pi_session_e2e.rs).
					// Pi reads provider config from ~/.pi/agent/models.json;
					// the file must exist before createSession even if the
					// real Anthropic endpoint is being used.
					await actor.mkdir("/home/user/.pi/agent");
					await actor.writeFile(
						"/home/user/.pi/agent/models.json",
						"{}",
					);
					await actor.mkdir("/home/user/workspace");

					const session = await actor.createSession("pi", {
						cwd: "/home/user/workspace",
						env: {
							HOME: "/home/user",
							ANTHROPIC_API_KEY: process.env.ANTHROPIC_API_KEY!,
							PI_SKIP_VERSION_CHECK: "1",
						},
						// `/etc/agentos/instructions.md` isn't seeded by
						// default; skip the OS-instruction injection step.
						skipOsInstructions: true,
					});
					expect(typeof session.sessionId).toBe("string");
					expect(session.sessionId.length).toBeGreaterThan(0);

					// Subscribe to sessionEvent BEFORE sending the prompt.
					// The Rust session-event forwarder broadcasts every
					// `session/update` JSON-RPC notification the Pi
					// adapter emits. Whether the Pi adapter emits any
					// notifications for a given prompt depends on
					// Pi/Anthropic behavior — trivial 1-word replies
					// often produce zero notifications. So we don't
					// assert a count, only that any events that DO come
					// through carry the expected sessionId.
					const sessionEvents: any[] = [];
					const conn = actor.connect();
					conn.on("sessionEvent", (data: any) => {
						sessionEvents.push(data);
					});

					const reply = await actor.sendPrompt(
						session.sessionId,
						"Reply with exactly the word: pong",
					);
					expect(typeof reply.text).toBe("string");
					expect(reply.text.length).toBeGreaterThan(0);
					// Don't pin the exact response — the model may add
					// punctuation or framing. Just confirm the word "pong"
					// showed up (case-insensitive).
					expect(reply.text.toLowerCase()).toContain("pong");

					// Shape check: if any events came through, they
					// must carry the right sessionId. Empty is fine.
					expect(
						sessionEvents.every(
							(e) => e.sessionId === session.sessionId,
						),
					).toBe(true);

					const sessions = await actor.listSessions();
					expect(
						sessions.some((s: any) => s.sessionId === session.sessionId),
					).toBe(true);

					await actor.closeSession(session.sessionId);
					await conn.dispose();
				},
				120_000,
			);
		},
	);
});

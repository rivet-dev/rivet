import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

export function runActorAgentOsTests(driverTestConfig: DriverTestConfig) {
	describe.skipIf(driverTestConfig.skip?.agentOs)(
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
				expect(await actor.exists("/home/user/todelete.txt")).toBe(false);
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
				expect(
					new TextDecoder().decode(readResults[0].content),
				).toBe("aaa");
				expect(
					new TextDecoder().decode(readResults[1].content),
				).toBe("bbb");
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
				await actor.writeFile(
					"/tmp/exit42.js",
					'process.exit(42);',
				);

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
					'setTimeout(() => {}, 30000);',
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
					'setTimeout(() => {}, 60000);',
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
				expect(new TextDecoder().decode(result.body)).toBe("vm-response");
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
}

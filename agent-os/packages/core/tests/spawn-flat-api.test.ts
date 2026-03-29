import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { AgentOs } from "../src/index.js";

describe("flat spawn API", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create();
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("onProcessStderr captures stderr, onProcessExit fires with exit code", async () => {
		await vm.writeFile(
			"/tmp/stderr-exit.mjs",
			'process.stderr.write("err-data\\n"); process.exit(42);',
		);

		const { pid } = vm.spawn("node", ["/tmp/stderr-exit.mjs"], {
			env: { HOME: "/home/user" },
		});

		const stderrChunks: string[] = [];
		vm.onProcessStderr(pid, (data) => {
			stderrChunks.push(new TextDecoder().decode(data));
		});

		const exitCodePromise = new Promise<number>((resolve) => {
			vm.onProcessExit(pid, resolve);
		});

		const exitCode = await exitCodePromise;
		expect(exitCode).toBe(42);
		expect(stderrChunks.join("")).toContain("err-data");
	}, 30_000);

	test("spawn returns { pid }, writeProcessStdin sends data, onProcessStdout receives it", async () => {
		await vm.writeFile(
			"/tmp/echo-stdin.mjs",
			`process.stdin.on("data", (chunk) => process.stdout.write(chunk));`,
		);

		const { pid } = vm.spawn("node", ["/tmp/echo-stdin.mjs"], {
			streamStdin: true,
			env: { HOME: "/home/user" },
		});

		const chunks: string[] = [];
		vm.onProcessStdout(pid, (data) => {
			chunks.push(new TextDecoder().decode(data));
		});

		vm.writeProcessStdin(pid, "hello from flat api\n");

		// Give time for output to arrive
		await new Promise((r) => setTimeout(r, 500));

		vm.killProcess(pid);
		await vm.waitProcess(pid);

		expect(chunks.join("")).toContain("hello from flat api");
	}, 30_000);
});

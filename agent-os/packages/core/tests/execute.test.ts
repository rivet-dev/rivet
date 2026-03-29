import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { AgentOs } from "../src/index.js";

const COMMANDS_DIR = resolve(
	import.meta.dirname,
	"../../../../secure-exec-1/native/wasmvm/target/wasm32-wasip1/release/commands",
);

const skipReason = existsSync(COMMANDS_DIR)
	? false
	: "WASM binaries not built (run make in secure-exec-1/native/wasmvm/)";

describe.skipIf(skipReason)("command execution", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create({ commandDirs: [COMMANDS_DIR] });
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("exec returns stdout with exit code 0", async () => {
		const result = await vm.exec("echo hello");
		expect(result.exitCode).toBe(0);
		expect(result.stdout.trim()).toBe("hello");
	});

	test("exec returns stderr and non-zero exit code", async () => {
		const result = await vm.exec("echo error >&2 && exit 1");
		expect(result.exitCode).toBe(1);
		expect(result.stderr.trim()).toBe("error");
	});

	test("exec with env vars passes them through", async () => {
		const result = await vm.exec("echo $MY_VAR", {
			env: { MY_VAR: "test-value" },
		});
		expect(result.exitCode).toBe(0);
		expect(result.stdout.trim()).toBe("test-value");
	});

	test("exec with cwd sets working directory", async () => {
		await vm.mkdir("/tmp/testdir");
		await vm.writeFile("/tmp/testdir/marker.txt", "found");
		const result = await vm.exec("cat /tmp/testdir/marker.txt", {
			cwd: "/tmp/testdir",
		});
		expect(result.exitCode).toBe(0);
		expect(result.stdout).toContain("found");
	});

	test("spawn and interact with process", async () => {
		const { pid } = vm.spawn("cat", []);
		vm.writeProcessStdin(pid, "hello from stdin\n");
		vm.closeProcessStdin(pid);
		const exitCode = await vm.waitProcess(pid);
		expect(exitCode).toBe(0);
	});

	test("exec node script", async () => {
		await vm.writeFile("/tmp/test.js", 'console.log("node-output");');
		const result = await vm.exec("node /tmp/test.js");
		expect(result.exitCode).toBe(0);
		expect(result.stdout).toContain("node-output");
	});

	test("exec shell pipeline", async () => {
		const result = await vm.exec("echo hello | cat");
		expect(result.exitCode).toBe(0);
		expect(result.stdout).toContain("hello");
	});
});

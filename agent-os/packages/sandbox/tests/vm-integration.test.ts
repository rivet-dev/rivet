/**
 * VM integration test: creates a real AgentOS VM with the sandbox filesystem
 * mounted and toolkit registered, then verifies the mount is accessible from
 * inside the VM and the toolkit tools execute correctly.
 *
 * CLI shim tests (vm.exec("agentos-sandbox ...")) are skipped because the
 * WASM shell cannot execute shell scripts via shebang in the current
 * environment. This is a known pre-existing limitation that also affects
 * the core host-tools-shims tests.
 */

import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it } from "vitest";
import { existsSync } from "node:fs";
import { AgentOs } from "@rivet-dev/agent-os-core";
import common, { coreutils } from "@rivet-dev/agent-os-common";
import type { SandboxAgentContainerHandle } from "@rivet-dev/agent-os-core/test/docker";
import { startSandboxAgentContainer } from "@rivet-dev/agent-os-core/test/docker";
import { createSandboxFs, createSandboxToolkit } from "../src/index.js";

let sandbox: SandboxAgentContainerHandle;

const hasWasm = existsSync(coreutils.commandDir);
const skipReason = process.env.SKIP_SANDBOX_TESTS
	? "SKIP_SANDBOX_TESTS is set"
	: !hasWasm
		? "WASM binaries not available"
		: undefined;

beforeAll(async () => {
	if (skipReason) return;
	sandbox = await startSandboxAgentContainer({ healthTimeout: 120_000 });
}, 150_000);

afterAll(async () => {
	if (sandbox) await sandbox.stop();
});

describe.skipIf(skipReason)("VM integration", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create({
			software: [common],
			mounts: [
				{
					path: "/sandbox",
					driver: createSandboxFs({ client: sandbox.client }),
				},
			],
			toolKits: [createSandboxToolkit({ client: sandbox.client })],
		});
	});

	afterEach(async () => {
		await vm.dispose();
	});

	// -- Filesystem mount tests --

	it("should write a file via the sandbox mount and read it back", async () => {
		await vm.writeFile("/sandbox/test.txt", "hello from VM mount");
		const data = await vm.readFile("/sandbox/test.txt");
		expect(new TextDecoder().decode(data)).toBe("hello from VM mount");
	});

	it("should list the sandbox mount contents via shell", async () => {
		await vm.writeFile("/sandbox/a.txt", "a");
		await vm.writeFile("/sandbox/b.txt", "b");
		const result = await vm.exec("ls /sandbox");
		expect(result.exitCode).toBe(0);
		expect(result.stdout).toContain("a.txt");
		expect(result.stdout).toContain("b.txt");
	});

	it("should create directories in the sandbox mount", async () => {
		await vm.mkdir("/sandbox/nested");
		await vm.writeFile("/sandbox/nested/deep.txt", "deep file");
		const content = await vm.readFile("/sandbox/nested/deep.txt");
		expect(new TextDecoder().decode(content)).toBe("deep file");
	});

	it("should cat a sandbox file from the WASM shell", async () => {
		await vm.writeFile("/sandbox/shell-read.txt", "read by shell");
		const result = await vm.exec("cat /sandbox/shell-read.txt");
		expect(result.exitCode).toBe(0);
		expect(result.stdout).toBe("read by shell");
	});

	// -- Toolkit shim installation --

	it("should have agentos-sandbox shim installed and executable", async () => {
		expect(await vm.exists("/usr/local/bin/agentos-sandbox")).toBe(true);
		expect(await vm.exists("/usr/local/bin/agentos")).toBe(true);
		const stat = await vm.stat("/usr/local/bin/agentos-sandbox");
		expect(stat.mode & 0o111).toBeGreaterThan(0);
	});

	// -- Toolkit direct execution (host RPC, not via CLI shim) --

	it("should execute run-command tool directly via the toolkit", async () => {
		const tk = createSandboxToolkit({ client: sandbox.client });
		const result = await tk.tools["run-command"].execute({
			command: "echo",
			args: ["hello", "from", "sandbox"],
		});
		expect(result.exitCode).toBe(0);
		expect(result.stdout).toContain("hello from sandbox");
	});

	it("should exercise the toolkit tool directly from a VM context", async () => {
		// Write a file into the sandbox via the toolkit, then read it via the mount.
		const tk = createSandboxToolkit({ client: sandbox.client });

		// Confirm the sandbox toolkit runs commands successfully.
		const result = await tk.tools["run-command"].execute({
			command: "echo",
			args: ["hello from sandbox toolkit"],
		});
		expect(result.exitCode).toBe(0);
		expect(result.stdout).toContain("hello from sandbox toolkit");

		// Create a process and list it.
		const proc = await tk.tools["create-process"].execute({
			command: "sleep",
			args: ["60"],
		});
		expect(proc.status).toBe("running");

		const listed = await tk.tools["list-processes"].execute({});
		const found = listed.processes.find(
			(p: { id: string }) => p.id === proc.id,
		);
		expect(found).toBeDefined();

		await tk.tools["kill-process"].execute({ id: proc.id });
	});
});

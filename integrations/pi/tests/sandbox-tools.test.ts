import type {
	Sandbox,
	SandboxExecOptions,
	SandboxExecResult,
} from "@rivet-dev/sandbox-adapter";
import { describe, expect, it, vi } from "vitest";
import { createSandboxTools, resolveSandboxPath } from "../src/index.js";

function sandboxFixture(): Sandbox & { calls: string[] } {
	const files = new Map<string, Uint8Array>([
		["/workspace/file.txt", Buffer.from("before\n")],
	]);
	const calls: string[] = [];
	return {
		calls,
		binding: { provider: "test", id: "sandbox-1" },
		cwd: "/workspace",
		async exec(command: string, options?: SandboxExecOptions): Promise<SandboxExecResult> {
			calls.push(`exec:${command}`);
			const stdout = command.startsWith("rg --files ")
				? "file.txt\0"
				: command.startsWith("rg ")
					? "/workspace/file.txt:1:before\n"
					: "command output\n";
			options?.onOutput?.({
				sequence: 0,
				stream: "stdout",
				data: Buffer.from(stdout),
			});
			return { exitCode: 0, stdout, stderr: "", outcome: "exited" };
		},
		async spawn() {
			throw new Error("not used");
		},
		async readFile(path) {
			calls.push(`read:${path}`);
			const content = files.get(path);
			if (!content) throw new Error("not found");
			return content;
		},
		async writeFile(path, content) {
			calls.push(`write:${path}`);
			files.set(path, Buffer.from(content));
		},
		async stat(path) {
			calls.push(`stat:${path}`);
			return path === "/workspace"
				? { type: "directory", size: 0 }
				: { type: "file", size: files.get(path)?.byteLength ?? 0 };
		},
		async readdir(path) {
			calls.push(`readdir:${path}`);
			return ["file.txt"];
		},
		async exists(path) {
			calls.push(`exists:${path}`);
			return path === "/workspace" || files.has(path);
		},
		async mkdir(path) {
			calls.push(`mkdir:${path}`);
		},
		async remove(path) {
			calls.push(`remove:${path}`);
			files.delete(path);
		},
	};
}

async function execute(
	tools: ReturnType<typeof createSandboxTools>,
	name: string,
	params: unknown,
) {
	const tool = tools.find((candidate) => candidate.name === name)!;
	return tool.execute("call-1", params, undefined, undefined, {
		sessionManager: {
			getSessionId: () => "session-1",
			getSessionFile: () => undefined,
		},
		model: undefined,
		thinkingLevel: "medium",
	} as never);
}

describe("Pi sandbox tools", () => {
	it("routes all seven built-in coding tools through the sandbox", async () => {
		const sandbox = sandboxFixture();
		const tools = createSandboxTools(sandbox);
		expect(tools.map((tool) => tool.name)).toEqual([
			"read",
			"bash",
			"edit",
			"write",
			"grep",
			"find",
			"ls",
		]);

		await execute(tools, "read", { path: "file.txt" });
		await execute(tools, "bash", { command: "pwd" });
		await execute(tools, "edit", {
			path: "file.txt",
			edits: [{ oldText: "before", newText: "after" }],
		});
		await execute(tools, "write", { path: "new.txt", content: "new" });
		await execute(tools, "grep", { pattern: "after" });
		await execute(tools, "find", { pattern: "*.txt" });
		await execute(tools, "ls", {});

		expect(sandbox.calls.some((call) => call.startsWith("exec:pwd"))).toBe(true);
		expect(sandbox.calls.some((call) => call.startsWith("exec:rg "))).toBe(true);
		expect(
			sandbox.calls.some((call) => call.startsWith("exec:rg --files ")),
		).toBe(true);
		expect(sandbox.calls).toContain("read:/workspace/file.txt");
		expect(sandbox.calls).toContain("write:/workspace/file.txt");
		expect(sandbox.calls).toContain("write:/workspace/new.txt");
		expect(sandbox.calls).toContain("readdir:/workspace");
	});

	it("rejects paths outside the mounted working directory", () => {
		expect(() => resolveSandboxPath("/workspace", "../host")).toThrow(
			"Path escapes sandbox working directory",
		);
	});

	it("supports a sandbox whose working directory is its root", () => {
		expect(resolveSandboxPath("/", "workspace/file.txt")).toBe(
			"/workspace/file.txt",
		);
	});
});

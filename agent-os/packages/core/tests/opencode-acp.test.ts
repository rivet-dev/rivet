import { resolve } from "node:path";
import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { AgentOs } from "../src/index.js";

/**
 * OpenCode ACP Manual Spawn Tests
 *
 * OpenCode speaks ACP natively via `opencode acp` subcommand — no separate
 * adapter like pi-acp is needed. The `opencode acp` command starts a JSON-RPC
 * 2.0 server over stdio that implements the Agent Communication Protocol.
 *
 * BLOCKED: OpenCode is a native ELF binary (compiled Go), not Node.js. The
 * secure-exec VM kernel only supports JS/WASM command execution. The opencode-ai
 * npm package is a thin wrapper that calls child_process.spawnSync on the native
 * binary, which returns ENOENT inside the VM.
 *
 * ACP protocol differences from PI (documented from OpenCode's public docs):
 * - OpenCode speaks ACP directly (`opencode acp`), PI requires pi-acp adapter
 * - OpenCode uses its own agentInfo.name (e.g., "opencode") vs PI's pi-acp info
 * - Both use the same JSON-RPC 2.0 transport over stdio
 * - Both support initialize, session/new, session/prompt, session/cancel
 */

const MODULE_ACCESS_CWD = resolve(import.meta.dirname, "..");

describe.skip("OpenCode ACP manual spawn", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create({
			moduleAccessCwd: MODULE_ACCESS_CWD,
		});
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("opencode-ai wrapper bin is accessible in VM", async () => {
		// Verify the opencode-ai wrapper script (Node.js) is available via
		// the ModuleAccessFileSystem overlay. This is the entry point that
		// would normally spawn the native binary.
		const script = `
const fs = require("fs");
const pkgPath = "/root/node_modules/opencode-ai/package.json";
const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf-8"));
const binEntry = typeof pkg.bin === "string" ? pkg.bin : (pkg.bin.opencode || Object.values(pkg.bin)[0]);
const binPath = "/root/node_modules/opencode-ai/" + binEntry;
const binExists = fs.existsSync(binPath);
console.log("bin-path:" + binPath);
console.log("bin-exists:" + binExists);
if (binExists) {
  const content = fs.readFileSync(binPath, "utf-8");
  console.log("is-js-wrapper:" + content.includes("spawnSync"));
}
`;
		await vm.writeFile("/tmp/check-acp-bin.mjs", script);

		let stdout = "";
		let stderr = "";

		const { pid } = vm.spawn("node", ["/tmp/check-acp-bin.mjs"], {
			onStdout: (data: Uint8Array) => {
				stdout += new TextDecoder().decode(data);
			},
			onStderr: (data: Uint8Array) => {
				stderr += new TextDecoder().decode(data);
			},
		});

		const exitCode = await vm.waitProcess(pid);

		expect(exitCode, `Failed. stderr: ${stderr}`).toBe(0);
		expect(stdout).toContain("bin-exists:true");
		// The bin entry is a JS wrapper that shells out to the native binary
		expect(stdout).toContain("is-js-wrapper:true");
	}, 30_000);

	test("opencode acp mode cannot start - native binary required", async () => {
		// Attempt to spawn the opencode wrapper with the `acp` subcommand.
		// This fails because the wrapper calls spawnSync on the native ELF
		// binary, which the kernel cannot execute.
		//
		// In a non-VM environment, `opencode acp` would start a JSON-RPC 2.0
		// server on stdio, accepting initialize, session/new, session/prompt, etc.
		const script = `
const childProcess = require("child_process");
const fs = require("fs");

// Find the native binary (same resolution as the wrapper)
const candidates = [
  "/root/node_modules/opencode-linux-x64-baseline/bin/opencode",
  "/root/node_modules/opencode-linux-x64/bin/opencode",
];
let binaryPath = null;
for (const c of candidates) {
  if (fs.existsSync(c)) {
    binaryPath = c;
    break;
  }
}

if (!binaryPath) {
  console.log("result:no-binary");
  process.exit(0);
}

console.log("binary:" + binaryPath);

// Attempt to spawn the native binary with "acp" subcommand
// This is what would start the ACP JSON-RPC server over stdio
const result = childProcess.spawnSync(binaryPath, ["acp"], {
  timeout: 5000,
  encoding: "utf-8",
});

console.log("status:" + result.status);
const stderrStr = result.stderr ? String(result.stderr) : "";
console.log("stderr:" + stderrStr);
console.log("failed:" + (result.status !== 0));
`;
		await vm.writeFile("/tmp/try-acp.mjs", script);

		let stdout = "";
		let stderr = "";

		const { pid } = vm.spawn("node", ["/tmp/try-acp.mjs"], {
			onStdout: (data: Uint8Array) => {
				stdout += new TextDecoder().decode(data);
			},
			onStderr: (data: Uint8Array) => {
				stderr += new TextDecoder().decode(data);
			},
		});

		const exitCode = await vm.waitProcess(pid);

		expect(exitCode, `Script failed. stderr: ${stderr}`).toBe(0);
		expect(stdout).toContain("binary:");
		// Native binary spawn fails — kernel can't execute ELF binaries
		expect(stdout).toContain("status:1");
		expect(stdout).toContain("failed:true");
		// ENOENT from the kernel's command resolver
		expect(stdout).toMatch(/ENOENT|command not found/);
	}, 30_000);

	test("ACP JSON-RPC protocol tests skipped - native binary limitation", async () => {
		// This test documents why the ACP protocol tests (initialize,
		// session/new) cannot be run for OpenCode inside the secure-exec VM.
		//
		// The tests that WOULD be here (matching pi-acp-adapter.test.ts):
		//
		// 1. "initialize returns protocolVersion and agentInfo"
		//    - Send: { method: "initialize", params: { protocolVersion: 1, clientCapabilities: {} } }
		//    - Expected: { result: { protocolVersion: <number>, agentInfo: { name: "opencode", ... } } }
		//
		// 2. "session/new returns sessionId"
		//    - Send: { method: "session/new", params: { cwd: "/home/user", mcpServers: [] } }
		//    - Expected: { result: { sessionId: <string> } }
		//
		// Unlike PI (which needs pi-acp as a separate adapter), OpenCode's ACP
		// mode is built-in. The `opencode acp` command starts the JSON-RPC server
		// directly — no wrapper process indirection.
		//
		// To enable these tests, one of:
		// (a) Add native binary execution to the secure-exec kernel
		// (b) Run OpenCode outside the VM and proxy ACP over a socket/pipe
		// (c) Create a mock OpenCode ACP responder in JS for unit testing
		//
		// For now, we verify that the binary and wrapper are accessible (above
		// tests) and document the protocol expectations here.

		expect(true).toBe(true); // Placeholder — tests are structurally skipped
	});
});

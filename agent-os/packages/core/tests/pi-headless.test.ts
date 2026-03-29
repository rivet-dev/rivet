import { resolve } from "node:path";
import type { LLMock } from "@copilotkit/llmock";
import {
	afterAll,
	afterEach,
	beforeAll,
	beforeEach,
	describe,
	expect,
	test,
} from "vitest";
import { AgentOs } from "../src/index.js";
import {
	DEFAULT_TEXT_FIXTURE,
	startLlmock,
	stopLlmock,
} from "./helpers/llmock-helper.js";

/**
 * Use the workspace root as module access CWD. With shamefully-hoist=true
 * in .npmrc, all transitive dependencies are hoisted to the root node_modules,
 * making them accessible via the ModuleAccessFileSystem overlay.
 */
const MODULE_ACCESS_CWD = resolve(import.meta.dirname, "..");

describe("PI headless mode", () => {
	let vm: AgentOs;
	let mock: LLMock;
	let mockUrl: string;
	let mockPort: number;

	beforeAll(async () => {
		const result = await startLlmock([DEFAULT_TEXT_FIXTURE]);
		mock = result.mock;
		mockUrl = result.url;
		mockPort = Number(new URL(result.url).port);
	});

	afterAll(async () => {
		await stopLlmock(mock);
	});

	beforeEach(async () => {
		vm = await AgentOs.create({
			loopbackExemptPorts: [mockPort],
			moduleAccessCwd: MODULE_ACCESS_CWD,
		});
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("mock LLM server responds to API calls from inside VM", async () => {
		// Write a script that calls the mock Anthropic API via fetch
		const apiScript = `
const response = await fetch("${mockUrl}/v1/messages", {
  method: "POST",
  headers: { "Content-Type": "application/json", "x-api-key": "mock-key" },
  body: JSON.stringify({
    model: "claude-sonnet-4-20250514",
    max_tokens: 100,
    messages: [{ role: "user", content: "say hello" }],
  }),
});
const data = await response.json();
console.log(data.content[0].text);
`;
		await vm.writeFile("/tmp/api-test.mjs", apiScript);

		let stdout = "";
		let stderr = "";

		const { pid } = vm.spawn("node", ["/tmp/api-test.mjs"], {
			onStdout: (data: Uint8Array) => {
				stdout += new TextDecoder().decode(data);
			},
			onStderr: (data: Uint8Array) => {
				stderr += new TextDecoder().decode(data);
			},
			env: {
				HOME: "/home/user",
				ANTHROPIC_API_KEY: "mock-key",
			},
		});

		const exitCode = await vm.waitProcess(pid);

		expect(exitCode, `API test failed. stderr: ${stderr}`).toBe(0);
		expect(stdout).toContain("Hello from llmock");
	}, 30_000);

	test("PI main module loads inside VM via CJS require", async () => {
		// Verify PI's main module can be loaded (CJS path handles export * correctly)
		const loadScript = `
const pi = globalThis._requireFrom("/root/node_modules/@mariozechner/pi-coding-agent/dist/main.js", "/");
console.log("main:" + typeof pi.main);
const args = globalThis._requireFrom("/root/node_modules/@mariozechner/pi-coding-agent/dist/cli/args.js", "/");
const parsed = args.parseArgs(["-p", "--no-session", "hello"]);
console.log("print:" + parsed.print);
console.log("messages:" + JSON.stringify(parsed.messages));
`;
		await vm.writeFile("/tmp/pi-load-test.mjs", loadScript);

		let stdout = "";
		let stderr = "";

		const { pid } = vm.spawn("node", ["/tmp/pi-load-test.mjs"], {
			onStdout: (data: Uint8Array) => {
				stdout += new TextDecoder().decode(data);
			},
			onStderr: (data: Uint8Array) => {
				stderr += new TextDecoder().decode(data);
			},
			env: {
				HOME: "/home/user",
				PI_OFFLINE: "1",
			},
		});

		const exitCode = await vm.waitProcess(pid);

		expect(exitCode, `PI load failed. stderr: ${stderr}`).toBe(0);
		expect(stdout).toContain("main:function");
		expect(stdout).toContain("print:true");
		expect(stdout).toContain('messages:["hello"]');
	}, 30_000);

	// TODO: Full PI headless execution is blocked by two secure-exec VM limitations:
	// 1. ESM module linking: V8 Rust runtime doesn't forward named exports from
	//    host-loaded modules (ModuleAccessFileSystem overlay). VFS modules work fine.
	//    PI's CLI must run as ESM (has async top-level main()), but ESM mode can't
	//    load host modules with named exports.
	// 2. CJS mode: Works for loading PI's modules, but the V8 session doesn't
	//    process the event loop after synchronous code finishes, so async main()
	//    never completes.
	// Fix: Either fix V8 module linking for overlay modules, or add event loop
	// processing to CJS session mode.
});

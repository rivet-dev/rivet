import { resolve } from "node:path";
import type { KernelSpawnOptions, ManagedProcess } from "@secure-exec/core";
import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { AgentOs } from "../src/agent-os.js";
import { AGENT_CONFIGS } from "../src/agents.js";
import { getOsInstructions } from "../src/os-instructions.js";
import {
	REGISTRY_SOFTWARE,
	registrySkipReason,
} from "./helpers/registry-commands.js";

/**
 * Workspace root has shamefully-hoisted node_modules with pi-acp available.
 */
const MODULE_ACCESS_CWD = resolve(import.meta.dirname, "..");

// ── getOsInstructions unit tests ───────────────────────────────────────

describe("getOsInstructions", () => {
	test("returns non-empty string from fixture", () => {
		const result = getOsInstructions();
		expect(result).toBeTruthy();
		expect(typeof result).toBe("string");
		expect(result.length).toBeGreaterThan(0);
	});

	test("appends additional text", () => {
		const base = getOsInstructions();
		const additional = "Custom agent-specific instructions here.";
		const result = getOsInstructions(additional);
		expect(result).toContain(base);
		expect(result).toContain(additional);
		// Additional text comes after base, separated by newline
		expect(result).toBe(`${base}\n${additional}`);
	});
});

// ── /etc/agentos/ boot-time tests ─────────────────────────────────────

describe("/etc/agentos/ setup at boot", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create();
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("/etc/agentos/instructions.md exists after AgentOs.create()", async () => {
		const fileExists = await vm.exists("/etc/agentos/instructions.md");
		expect(fileExists).toBe(true);
	});

	test("content matches getOsInstructions() output", async () => {
		const data = await vm.readFile("/etc/agentos/instructions.md");
		const content = new TextDecoder().decode(data);
		const expected = getOsInstructions();
		expect(content).toBe(expected);
	});

	test("additionalInstructions option appends to file content", async () => {
		await vm.dispose();
		const additional = "CUSTOM_MARKER: project-specific rules";
		vm = await AgentOs.create({ additionalInstructions: additional });

		const data = await vm.readFile("/etc/agentos/instructions.md");
		const content = new TextDecoder().decode(data);
		const expected = getOsInstructions(additional);
		expect(content).toBe(expected);
		expect(content).toContain(additional);
	});
});

// ── /etc/agentos/ read-only mount tests ──────────────────────────────

describe("/etc/agentos/ read-only mount", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create();
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("read from /etc/agentos/instructions.md succeeds", async () => {
		const data = await vm.readFile("/etc/agentos/instructions.md");
		const content = new TextDecoder().decode(data);
		expect(content).toBeTruthy();
		expect(content.length).toBeGreaterThan(0);
	});

	test("write to /etc/agentos/ throws EROFS", async () => {
		await expect(
			vm.writeFile("/etc/agentos/tampered.md", "malicious content"),
		).rejects.toThrow("EROFS");
	});

	test("delete /etc/agentos/instructions.md throws EROFS", async () => {
		await expect(vm.delete("/etc/agentos/instructions.md")).rejects.toThrow(
			"EROFS",
		);
	});
});

describe.skipIf(registrySkipReason)("/etc/agentos/ exec from inside VM", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create({ software: REGISTRY_SOFTWARE });
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("exec('cat /etc/agentos/instructions.md') returns the instructions content", async () => {
		const result = await vm.exec("cat /etc/agentos/instructions.md");
		expect(result.exitCode).toBe(0);
		const expected = getOsInstructions();
		// WasmVM stdout can duplicate lines; use toContain
		expect(result.stdout).toContain(expected);
	});
});

// ── prepareInstructions unit tests (agent configs) ─────────────────────

describe("PI prepareInstructions", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create();
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("reads /etc/agentos/instructions.md and returns --append-system-prompt in args", async () => {
		const config = AGENT_CONFIGS.pi;
		const prepare = config.prepareInstructions as NonNullable<
			typeof config.prepareInstructions
		>;
		const result = await prepare(vm.kernel, "/home/user");

		expect(result.args).toBeDefined();
		expect(result.args).toContain("--append-system-prompt");
		// The instruction text is the file content from /etc/agentos/instructions.md
		const argIdx = (result.args as string[]).indexOf(
			"--append-system-prompt",
		);
		const instructionsArg = (result.args as string[])[argIdx + 1];
		expect(instructionsArg).toBeTruthy();
		expect(instructionsArg.length).toBeGreaterThan(0);
		// PI does not set env vars
		expect(result.env).toBeUndefined();
	});

	test("appends additionalInstructions to file content", async () => {
		const config = AGENT_CONFIGS.pi;
		const prepare = config.prepareInstructions as NonNullable<
			typeof config.prepareInstructions
		>;
		const additional = "CUSTOM_MARKER: extra instructions";
		const result = await prepare(vm.kernel, "/home/user", additional);

		const argIdx = (result.args as string[]).indexOf(
			"--append-system-prompt",
		);
		const instructionsArg = (result.args as string[])[argIdx + 1];
		expect(instructionsArg).toContain(additional);
	});
});

describe("OpenCode prepareInstructions", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create();
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("sets OPENCODE_CONTEXTPATHS with absolute /etc/agentos/instructions.md path", async () => {
		const config = AGENT_CONFIGS.opencode;
		const cwd = "/home/user";

		const prepare = config.prepareInstructions as NonNullable<
			typeof config.prepareInstructions
		>;
		const result = await prepare(vm.kernel, cwd);

		// Verify env var is set
		expect(result.env).toBeDefined();
		expect(result.env?.OPENCODE_CONTEXTPATHS).toBeDefined();

		// Verify OPENCODE_CONTEXTPATHS includes default paths + absolute instructions path
		const contextPaths = JSON.parse(
			result.env?.OPENCODE_CONTEXTPATHS as string,
		);
		expect(contextPaths).toContain("/etc/agentos/instructions.md");
		expect(contextPaths).toContain("CLAUDE.md");
		expect(contextPaths).toContain("opencode.md");
		// No longer uses relative .agent-os/ path
		expect(contextPaths).not.toContain(".agent-os/instructions.md");

		// OpenCode does not set extra args
		expect(result.args).toBeUndefined();
	});

	test("does not write .agent-os/instructions.md to cwd", async () => {
		const config = AGENT_CONFIGS.opencode;
		const cwd = "/home/user";

		const prepare = config.prepareInstructions as NonNullable<
			typeof config.prepareInstructions
		>;
		await prepare(vm.kernel, cwd);

		// Verify no .agent-os/ directory was created in cwd
		const cwdExists = await vm.exists(`${cwd}/.agent-os`);
		expect(cwdExists).toBe(false);
	});

	test("writes additionalInstructions to /tmp/ and adds path to OPENCODE_CONTEXTPATHS", async () => {
		const config = AGENT_CONFIGS.opencode;
		const cwd = "/home/user";
		const additional = "CUSTOM_MARKER: extra instructions";

		const prepare = config.prepareInstructions as NonNullable<
			typeof config.prepareInstructions
		>;
		const result = await prepare(vm.kernel, cwd, additional);

		// Verify additional instructions written to /tmp/
		const data = await vm.readFile(
			"/tmp/agentos-additional-instructions.md",
		);
		const content = new TextDecoder().decode(data);
		expect(content).toBe(additional);

		// Verify OPENCODE_CONTEXTPATHS includes the additional file
		const contextPaths = JSON.parse(
			result.env?.OPENCODE_CONTEXTPATHS as string,
		);
		expect(contextPaths).toContain(
			"/tmp/agentos-additional-instructions.md",
		);
		// Base instructions path is still included
		expect(contextPaths).toContain("/etc/agentos/instructions.md");

		// /etc/agentos/instructions.md is NOT modified (it's read-only)
		const baseData = await vm.readFile("/etc/agentos/instructions.md");
		const baseContent = new TextDecoder().decode(baseData);
		expect(baseContent).not.toContain(additional);
	});
});

// ── createSession integration tests ────────────────────────────────────

/**
 * Mock ACP adapter that responds to initialize/session/new.
 * Echoes process.env in agentInfo for env var verification.
 */
const MOCK_ACP_ADAPTER = `
let buffer = '';
process.stdin.resume();
process.stdin.on('data', (chunk) => {
  const str = chunk instanceof Uint8Array ? new TextDecoder().decode(chunk) : String(chunk);
  buffer += str;

  while (true) {
    const idx = buffer.indexOf('\\n');
    if (idx === -1) break;
    const line = buffer.substring(0, idx);
    buffer = buffer.substring(idx + 1);
    if (!line.trim()) continue;

    try {
      const msg = JSON.parse(line);
      if (msg.id === undefined) continue;

      let result;
      switch (msg.method) {
        case 'initialize':
          result = {
            protocolVersion: 1,
            agentInfo: {
              name: 'mock-adapter',
              version: '1.0',
            },
          };
          break;
        case 'session/new':
          result = { sessionId: 'mock-session-1' };
          break;
        case 'session/cancel':
          result = {};
          break;
        default:
          process.stdout.write(JSON.stringify({
            jsonrpc: '2.0', id: msg.id,
            error: { code: -32601, message: 'Method not found' },
          }) + '\\n');
          continue;
      }

      process.stdout.write(JSON.stringify({
        jsonrpc: '2.0', id: msg.id, result,
      }) + '\\n');
    } catch (e) {}
  }
});
`;

/** Captured spawn call info for kernel.spawn spy. */
interface SpawnCapture {
	command: string;
	args: string[];
	options: KernelSpawnOptions | undefined;
}

describe("createSession OS instructions integration", () => {
	let vm: AgentOs;
	let spawnCaptures: SpawnCapture[];

	beforeEach(async () => {
		vm = await AgentOs.create({ moduleAccessCwd: MODULE_ACCESS_CWD });
		spawnCaptures = [];

		// Spy on kernel.spawn to capture args while delegating to the real impl
		const origSpawn = vm.kernel.spawn.bind(vm.kernel);
		vm.kernel.spawn = (
			command: string,
			args: string[],
			options?: KernelSpawnOptions,
		): ManagedProcess => {
			spawnCaptures.push({ command, args, options });
			return origSpawn(command, args, options);
		};
	});

	afterEach(async () => {
		await vm.dispose();
	});

	/**
	 * Patch _resolveAdapterBin to return a mock script path instead of
	 * resolving the real adapter from node_modules.
	 */
	function useMockAdapterBin(scriptPath: string): () => void {
		const origResolve = (
			vm as unknown as { _resolveAdapterBin: (pkg: string) => string }
		)._resolveAdapterBin;
		(
			vm as unknown as { _resolveAdapterBin: (pkg: string) => string }
		)._resolveAdapterBin = (_pkg: string) => scriptPath;

		return () => {
			(
				vm as unknown as { _resolveAdapterBin: (pkg: string) => string }
			)._resolveAdapterBin = origResolve;
		};
	}

	test("createSession with PI passes --append-system-prompt in spawn args", async () => {
		const scriptPath = "/tmp/mock-adapter.mjs";
		await vm.writeFile(scriptPath, MOCK_ACP_ADAPTER);
		const restore = useMockAdapterBin(scriptPath);

		try {
			const { sessionId } = await vm.createSession("pi");

			// Verify kernel.spawn was called with --append-system-prompt in args
			expect(spawnCaptures.length).toBeGreaterThan(0);
			const spawnCall = spawnCaptures[0];
			expect(spawnCall.args).toContain("--append-system-prompt");
			// The instruction text follows --append-system-prompt
			const argIdx = spawnCall.args.indexOf("--append-system-prompt");
			const instructionsArg = spawnCall.args[argIdx + 1];
			expect(instructionsArg).toBeTruthy();
			expect(instructionsArg.length).toBeGreaterThan(0);

			vm.closeSession(sessionId);
		} finally {
			restore();
		}
	});

	test("createSession with OpenCode sets OPENCODE_CONTEXTPATHS with absolute /etc/agentos/ path", async () => {
		const scriptPath = "/tmp/mock-adapter.mjs";
		await vm.writeFile(scriptPath, MOCK_ACP_ADAPTER);
		const restore = useMockAdapterBin(scriptPath);

		try {
			const { sessionId } = await vm.createSession("opencode");

			// Verify OPENCODE_CONTEXTPATHS was passed as env var to spawn
			expect(spawnCaptures.length).toBeGreaterThan(0);
			const spawnCall = spawnCaptures[0];
			const envPaths = spawnCall.options?.env?.OPENCODE_CONTEXTPATHS;
			expect(envPaths).toBeTruthy();
			const contextPaths = JSON.parse(envPaths as string);
			expect(contextPaths).toContain("/etc/agentos/instructions.md");
			// No longer uses relative .agent-os/ path
			expect(contextPaths).not.toContain(".agent-os/instructions.md");

			// No .agent-os/ directory created in cwd
			const agentOsDirExists = await vm.exists("/home/user/.agent-os");
			expect(agentOsDirExists).toBe(false);

			vm.closeSession(sessionId);
		} finally {
			restore();
		}
	});

	test("createSession with skipOsInstructions:true does not inject args or env", async () => {
		const scriptPath = "/tmp/mock-adapter.mjs";
		await vm.writeFile(scriptPath, MOCK_ACP_ADAPTER);
		const restore = useMockAdapterBin(scriptPath);

		try {
			const { sessionId } = await vm.createSession("pi", {
				skipOsInstructions: true,
			});

			// Verify kernel.spawn was NOT called with --append-system-prompt
			expect(spawnCaptures.length).toBeGreaterThan(0);
			const spawnCall = spawnCaptures[0];
			expect(spawnCall.args).not.toContain("--append-system-prompt");

			vm.closeSession(sessionId);
		} finally {
			restore();
		}
	});

	test("user-provided env vars override instruction env vars", async () => {
		const scriptPath = "/tmp/mock-adapter.mjs";
		await vm.writeFile(scriptPath, MOCK_ACP_ADAPTER);
		const restore = useMockAdapterBin(scriptPath);

		try {
			const userContextPaths = '["my-custom-paths.md"]';
			const { sessionId } = await vm.createSession("opencode", {
				env: { OPENCODE_CONTEXTPATHS: userContextPaths },
			});

			// Verify user's OPENCODE_CONTEXTPATHS wins over prepareInstructions
			expect(spawnCaptures.length).toBeGreaterThan(0);
			const spawnCall = spawnCaptures[0];
			expect(spawnCall.options?.env?.OPENCODE_CONTEXTPATHS).toBe(
				userContextPaths,
			);

			vm.closeSession(sessionId);
		} finally {
			restore();
		}
	});

	test("additionalInstructions content appears in injected text", async () => {
		const scriptPath = "/tmp/mock-adapter.mjs";
		await vm.writeFile(scriptPath, MOCK_ACP_ADAPTER);
		const restore = useMockAdapterBin(scriptPath);

		const additionalText =
			"CUSTOM_MARKER: Always use pnpm for this project.";

		try {
			const { sessionId } = await vm.createSession("pi", {
				additionalInstructions: additionalText,
			});

			// Verify the --append-system-prompt value contains additional text
			expect(spawnCaptures.length).toBeGreaterThan(0);
			const spawnCall = spawnCaptures[0];
			const argIdx = spawnCall.args.indexOf("--append-system-prompt");
			expect(argIdx).toBeGreaterThan(-1);
			const instructionsArg = spawnCall.args[argIdx + 1];
			expect(instructionsArg).toContain(additionalText);

			vm.closeSession(sessionId);
		} finally {
			restore();
		}
	});
});

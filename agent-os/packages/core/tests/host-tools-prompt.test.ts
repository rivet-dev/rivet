import { existsSync } from "node:fs";
import { resolve } from "node:path";
import type { KernelSpawnOptions, ManagedProcess } from "@secure-exec/core";
import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { z } from "zod";
import { AgentOs, generateToolReference, hostTool, toolKit } from "../src/index.js";
import { AGENT_CONFIGS } from "../src/agents.js";

const MODULE_ACCESS_CWD = resolve(import.meta.dirname, "..");

// ── generateToolReference unit tests ─────────────────────────────────────

const mathToolKit = toolKit({
	name: "math",
	description: "Math utilities",
	tools: {
		add: hostTool({
			description: "Add two numbers",
			inputSchema: z.object({
				a: z.number(),
				b: z.number(),
			}),
			execute: ({ a, b }) => ({ sum: a + b }),
			examples: [
				{
					description: "Add 1 and 2",
					input: { a: 1, b: 2 },
				},
			],
		}),
		multiply: hostTool({
			description: "Multiply two numbers",
			inputSchema: z.object({
				x: z.number(),
				y: z.number(),
			}),
			execute: ({ x, y }) => ({ product: x * y }),
		}),
	},
});

const textToolKit = toolKit({
	name: "text",
	description: "Text processing utilities",
	tools: {
		upper: hostTool({
			description: "Convert text to uppercase",
			inputSchema: z.object({
				input: z.string().describe("Text to convert"),
				trim: z.boolean().optional(),
			}),
			execute: ({ input, trim }) => {
				const text = trim ? input.trim() : input;
				return { output: text.toUpperCase() };
			},
			examples: [
				{
					description: "Uppercase hello",
					input: { input: "hello" },
				},
				{
					description: "Uppercase and trim",
					input: { input: "  hello  ", trim: true },
				},
			],
		}),
	},
});

describe("generateToolReference", () => {
	test("returns empty string for empty toolkits array", () => {
		const result = generateToolReference([]);
		expect(result).toBe("");
	});

	test("includes header and agentos list-tools instruction", () => {
		const result = generateToolReference([mathToolKit]);
		expect(result).toContain("## Available Host Tools");
		expect(result).toContain(
			"Run `agentos list-tools` to see all available tools.",
		);
	});

	test("includes toolkit name and description", () => {
		const result = generateToolReference([mathToolKit]);
		expect(result).toContain("### math");
		expect(result).toContain("Math utilities");
	});

	test("includes tool names with CLI signatures", () => {
		const result = generateToolReference([mathToolKit]);
		expect(result).toContain("agentos-math add");
		expect(result).toContain("agentos-math multiply");
		expect(result).toContain("Add two numbers");
		expect(result).toContain("Multiply two numbers");
	});

	test("includes flag signatures with types", () => {
		const result = generateToolReference([mathToolKit]);
		expect(result).toContain("--a <number>");
		expect(result).toContain("--b <number>");
	});

	test("marks optional flags with brackets", () => {
		const result = generateToolReference([textToolKit]);
		expect(result).toContain("[--trim <boolean>]");
	});

	test("includes help instruction per toolkit", () => {
		const result = generateToolReference([mathToolKit]);
		expect(result).toContain(
			"Run `agentos-math <tool> --help` for details.",
		);
	});

	test("includes examples when defined", () => {
		const result = generateToolReference([mathToolKit]);
		expect(result).toContain("**Examples:**");
		expect(result).toContain("Add 1 and 2");
		expect(result).toContain("agentos-math add --a 1 --b 2");
	});

	test("does not include examples section when no tools have examples", () => {
		const noExamplesKit = toolKit({
			name: "plain",
			description: "No examples",
			tools: {
				noop: hostTool({
					description: "Does nothing",
					inputSchema: z.object({}),
					execute: () => ({}),
				}),
			},
		});
		const result = generateToolReference([noExamplesKit]);
		expect(result).not.toContain("**Examples:**");
	});

	test("handles multiple toolkits", () => {
		const result = generateToolReference([mathToolKit, textToolKit]);
		expect(result).toContain("### math");
		expect(result).toContain("### text");
		expect(result).toContain("agentos-math add");
		expect(result).toContain("agentos-text upper");
	});

	test("includes multiple examples from the same toolkit", () => {
		const result = generateToolReference([textToolKit]);
		expect(result).toContain("Uppercase hello");
		expect(result).toContain("Uppercase and trim");
	});
});

// ── prepareInstructions tool reference tests ─────────────────────────────

describe("PI prepareInstructions with toolReference", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create();
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("appends tool reference after OS instructions", async () => {
		const config = AGENT_CONFIGS.pi;
		const prepare = config.prepareInstructions as NonNullable<
			typeof config.prepareInstructions
		>;
		const toolRef = generateToolReference([mathToolKit]);
		const result = await prepare(vm.kernel, "/home/user", undefined, {
			toolReference: toolRef,
		});

		const argIdx = (result.args as string[]).indexOf(
			"--append-system-prompt",
		);
		const instructionsArg = (result.args as string[])[argIdx + 1];
		expect(instructionsArg).toContain("## Available Host Tools");
		expect(instructionsArg).toContain("agentos-math add");
	});

	test("appends tool reference after additionalInstructions", async () => {
		const config = AGENT_CONFIGS.pi;
		const prepare = config.prepareInstructions as NonNullable<
			typeof config.prepareInstructions
		>;
		const additional = "CUSTOM_MARKER_123";
		const toolRef = generateToolReference([mathToolKit]);
		const result = await prepare(vm.kernel, "/home/user", additional, {
			toolReference: toolRef,
		});

		const argIdx = (result.args as string[]).indexOf(
			"--append-system-prompt",
		);
		const instructionsArg = (result.args as string[])[argIdx + 1];
		// Both additional and tool ref are present, in order
		expect(instructionsArg).toContain("CUSTOM_MARKER_123");
		expect(instructionsArg).toContain("## Available Host Tools");
		const additionalIdx = instructionsArg.indexOf("CUSTOM_MARKER_123");
		const toolRefIdx = instructionsArg.indexOf("## Available Host Tools");
		expect(toolRefIdx).toBeGreaterThan(additionalIdx);
	});

	test("skipBase returns only tool reference", async () => {
		const config = AGENT_CONFIGS.pi;
		const prepare = config.prepareInstructions as NonNullable<
			typeof config.prepareInstructions
		>;
		const toolRef = generateToolReference([mathToolKit]);
		const result = await prepare(vm.kernel, "/home/user", undefined, {
			toolReference: toolRef,
			skipBase: true,
		});

		const argIdx = (result.args as string[]).indexOf(
			"--append-system-prompt",
		);
		const instructionsArg = (result.args as string[])[argIdx + 1];
		// Tool reference is present
		expect(instructionsArg).toContain("## Available Host Tools");
		// OS base instructions are NOT present (check for fixture content)
		expect(instructionsArg).not.toContain("# agentOS");
	});

	test("skipBase with no tool reference returns empty result", async () => {
		const config = AGENT_CONFIGS.pi;
		const prepare = config.prepareInstructions as NonNullable<
			typeof config.prepareInstructions
		>;
		const result = await prepare(vm.kernel, "/home/user", undefined, {
			skipBase: true,
		});

		// No args or env when there's nothing to inject
		expect(result.args).toBeUndefined();
		expect(result.env).toBeUndefined();
	});
});

describe("OpenCode prepareInstructions with toolReference", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create();
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("writes tool reference to /tmp/ and adds to OPENCODE_CONTEXTPATHS", async () => {
		const config = AGENT_CONFIGS.opencode;
		const prepare = config.prepareInstructions as NonNullable<
			typeof config.prepareInstructions
		>;
		const toolRef = generateToolReference([mathToolKit]);
		const result = await prepare(vm.kernel, "/home/user", undefined, {
			toolReference: toolRef,
		});

		// Verify tool reference written to /tmp/
		const data = await vm.readFile("/tmp/agentos-tool-reference.md");
		const content = new TextDecoder().decode(data);
		expect(content).toContain("## Available Host Tools");

		// Verify OPENCODE_CONTEXTPATHS includes the tool ref file
		const contextPaths = JSON.parse(
			result.env?.OPENCODE_CONTEXTPATHS as string,
		);
		expect(contextPaths).toContain("/tmp/agentos-tool-reference.md");
		// Base instructions still included
		expect(contextPaths).toContain("/etc/agentos/instructions.md");
	});

	test("skipBase excludes OS context paths but includes tool reference", async () => {
		const config = AGENT_CONFIGS.opencode;
		const prepare = config.prepareInstructions as NonNullable<
			typeof config.prepareInstructions
		>;
		const toolRef = generateToolReference([mathToolKit]);
		const result = await prepare(vm.kernel, "/home/user", undefined, {
			toolReference: toolRef,
			skipBase: true,
		});

		const contextPaths = JSON.parse(
			result.env?.OPENCODE_CONTEXTPATHS as string,
		);
		// Tool reference file is present
		expect(contextPaths).toContain("/tmp/agentos-tool-reference.md");
		// Base OS instructions NOT present
		expect(contextPaths).not.toContain("/etc/agentos/instructions.md");
		expect(contextPaths).not.toContain("CLAUDE.md");
	});
});

// ── createSession tool reference integration ─────────────────────────────

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

interface SpawnCapture {
	command: string;
	args: string[];
	options: KernelSpawnOptions | undefined;
}

// Integration tests for tool reference injection via createSession.
// Uses a single VM per test to avoid session.close() corruption (see codebase patterns).

describe("createSession with toolkits injects tool reference", () => {
	function useMockAdapterBin(
		vm: AgentOs,
		scriptPath: string,
	): () => void {
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

	function spyOnSpawn(vm: AgentOs): SpawnCapture[] {
		const captures: SpawnCapture[] = [];
		const origSpawn = vm.kernel.spawn.bind(vm.kernel);
		vm.kernel.spawn = (
			command: string,
			args: string[],
			options?: KernelSpawnOptions,
		): ManagedProcess => {
			captures.push({ command, args, options });
			return origSpawn(command, args, options);
		};
		return captures;
	}

	test("prompt includes tool reference section with names, descriptions, and examples", async () => {
		const vm = await AgentOs.create({
			moduleAccessCwd: MODULE_ACCESS_CWD,
			toolKits: [mathToolKit],
		});
		const spawnCaptures = spyOnSpawn(vm);

		try {
			const scriptPath = "/tmp/mock-adapter.mjs";
			await vm.writeFile(scriptPath, MOCK_ACP_ADAPTER);
			const restore = useMockAdapterBin(vm, scriptPath);

			try {
				const { sessionId } = await vm.createSession("pi");

				expect(spawnCaptures.length).toBeGreaterThan(0);
				const spawnCall = spawnCaptures[0];
				const argIdx = spawnCall.args.indexOf("--append-system-prompt");
				expect(argIdx).toBeGreaterThan(-1);
				const instructionsArg = spawnCall.args[argIdx + 1];
				// Tool reference section is present
				expect(instructionsArg).toContain("## Available Host Tools");
				expect(instructionsArg).toContain("agentos-math add");
				expect(instructionsArg).toContain("Add two numbers");
				// Examples are present
				expect(instructionsArg).toContain("Add 1 and 2");
				expect(instructionsArg).toContain("agentos-math add --a 1 --b 2");

				vm.closeSession(sessionId);
			} finally {
				restore();
			}
		} finally {
			await vm.dispose();
		}
	});

	// The skipOsInstructions + toolReference path is verified by the unit test
	// "skipBase returns only tool reference" above. A full integration test for
	// this case is skipped because creating two VMs with sessions in the same
	// file causes resource leakage (see codebase pattern about VM corruption
	// after session.close).
});

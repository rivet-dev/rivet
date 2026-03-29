import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { z } from "zod";
import { AgentOs, hostTool, toolKit } from "../src/index.js";
import {
	REGISTRY_SOFTWARE,
	hasRegistryCommands,
} from "./helpers/registry-commands.js";

const testToolKit = toolKit({
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
		fail: hostTool({
			description: "Always throws",
			inputSchema: z.object({}),
			execute: () => {
				throw new Error("intentional failure");
			},
		}),
		slow: hostTool({
			description: "Takes too long",
			inputSchema: z.object({}),
			timeout: 100,
			execute: () =>
				new Promise((resolve) =>
					setTimeout(() => resolve("done"), 5000),
				),
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
		}),
		repeat: hostTool({
			description: "Repeat text N times",
			inputSchema: z.object({
				text: z.string(),
				count: z.number(),
				separator: z.enum(["space", "newline", "none"]).optional(),
			}),
			execute: ({ text, count, separator }) => {
				const sep =
					separator === "newline"
						? "\n"
						: separator === "space"
							? " "
							: "";
				return { output: Array(count).fill(text).join(sep) };
			},
		}),
	},
});

describe("host tools RPC server", () => {
	let vm: AgentOs;
	let port: number;

	beforeEach(async () => {
		vm = await AgentOs.create({
			toolKits: [testToolKit],
		});
		port = Number(vm.kernel.env.AGENTOS_TOOLS_PORT);
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("AGENTOS_TOOLS_PORT is set in kernel env", () => {
		expect(port).toBeGreaterThan(0);
	});

	test("POST /call executes tool and returns success", async () => {
		const res = await fetch(`http://127.0.0.1:${port}/call`, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				toolkit: "math",
				tool: "add",
				input: { a: 2, b: 3 },
			}),
		});
		expect(res.status).toBe(200);
		const body = await res.json();
		expect(body).toEqual({ ok: true, result: { sum: 5 } });
	});

	test("TOOLKIT_NOT_FOUND with available names", async () => {
		const res = await fetch(`http://127.0.0.1:${port}/call`, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				toolkit: "nonexistent",
				tool: "add",
				input: {},
			}),
		});
		const body = await res.json();
		expect(body.ok).toBe(false);
		expect(body.error).toBe("TOOLKIT_NOT_FOUND");
		expect(body.message).toContain("nonexistent");
		expect(body.message).toContain("math");
	});

	test("TOOL_NOT_FOUND with available tools", async () => {
		const res = await fetch(`http://127.0.0.1:${port}/call`, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				toolkit: "math",
				tool: "nonexistent",
				input: {},
			}),
		});
		const body = await res.json();
		expect(body.ok).toBe(false);
		expect(body.error).toBe("TOOL_NOT_FOUND");
		expect(body.message).toContain("nonexistent");
		expect(body.message).toContain("add");
	});

	test("VALIDATION_ERROR with zod details", async () => {
		const res = await fetch(`http://127.0.0.1:${port}/call`, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				toolkit: "math",
				tool: "add",
				input: { a: "not a number", b: 3 },
			}),
		});
		const body = await res.json();
		expect(body.ok).toBe(false);
		expect(body.error).toBe("VALIDATION_ERROR");
		expect(body.message.length).toBeGreaterThan(0);
	});

	test("EXECUTION_ERROR when execute() throws", async () => {
		const res = await fetch(`http://127.0.0.1:${port}/call`, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				toolkit: "math",
				tool: "fail",
				input: {},
			}),
		});
		const body = await res.json();
		expect(body.ok).toBe(false);
		expect(body.error).toBe("EXECUTION_ERROR");
		expect(body.message).toBe("intentional failure");
	});

	test("TIMEOUT when execute() exceeds timeout", async () => {
		const res = await fetch(`http://127.0.0.1:${port}/call`, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				toolkit: "math",
				tool: "slow",
				input: {},
			}),
		});
		const body = await res.json();
		expect(body.ok).toBe(false);
		expect(body.error).toBe("TIMEOUT");
		expect(body.message).toContain("slow");
		expect(body.message).toContain("100ms");
	});

	test("all responses are HTTP 200", async () => {
		const errorRes = await fetch(`http://127.0.0.1:${port}/call`, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				toolkit: "nonexistent",
				tool: "whatever",
				input: {},
			}),
		});
		expect(errorRes.status).toBe(200);
	});

	test("server closes on dispose", async () => {
		const savedPort = port;
		await vm.dispose();

		try {
			await fetch(`http://127.0.0.1:${savedPort}/call`, {
				method: "POST",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify({
					toolkit: "math",
					tool: "add",
					input: { a: 1, b: 1 },
				}),
			});
			expect.unreachable("fetch should have failed after dispose");
		} catch {
			// Expected: connection refused
		}

		// Recreate VM so afterEach dispose doesn't double-dispose
		vm = await AgentOs.create({
			toolKits: [testToolKit],
		});
	});

	test("no server started when toolKits is empty", async () => {
		const vmNoTools = await AgentOs.create();
		expect(vmNoTools.kernel.env.AGENTOS_TOOLS_PORT).toBeUndefined();
		await vmNoTools.dispose();
	});
});

describe("host tools list and describe endpoints", () => {
	let vm: AgentOs;
	let port: number;

	beforeEach(async () => {
		vm = await AgentOs.create({
			toolKits: [testToolKit, textToolKit],
		});
		port = Number(vm.kernel.env.AGENTOS_TOOLS_PORT);
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("GET /list returns all toolkits with descriptions and tool names", async () => {
		const res = await fetch(`http://127.0.0.1:${port}/list`);
		expect(res.status).toBe(200);
		const body = await res.json();
		expect(body.ok).toBe(true);
		const toolkits = body.result.toolkits;
		expect(toolkits).toHaveLength(2);

		const math = toolkits.find((t: any) => t.name === "math");
		expect(math).toBeDefined();
		expect(math.description).toBe("Math utilities");
		expect(math.tools).toContain("add");
		expect(math.tools).toContain("fail");

		const text = toolkits.find((t: any) => t.name === "text");
		expect(text).toBeDefined();
		expect(text.description).toBe("Text processing utilities");
		expect(text.tools).toContain("upper");
		expect(text.tools).toContain("repeat");
	});

	test("GET /list/<toolkit> returns tools with descriptions and flag details", async () => {
		const res = await fetch(`http://127.0.0.1:${port}/list/text`);
		expect(res.status).toBe(200);
		const body = await res.json();
		expect(body.ok).toBe(true);
		const result = body.result;
		expect(result.name).toBe("text");
		expect(result.description).toBe("Text processing utilities");

		// Check upper tool
		const upper = result.tools.upper;
		expect(upper.description).toBe("Convert text to uppercase");
		expect(upper.flags).toContainEqual({
			flag: "--input",
			type: "string",
			required: true,
			description: "Text to convert",
		});
		expect(upper.flags).toContainEqual({
			flag: "--trim",
			type: "boolean",
			required: false,
		});

		// Check repeat tool
		const repeat = result.tools.repeat;
		expect(repeat.description).toBe("Repeat text N times");
		const countFlag = repeat.flags.find((f: any) => f.flag === "--count");
		expect(countFlag).toEqual({
			flag: "--count",
			type: "number",
			required: true,
		});
		const sepFlag = repeat.flags.find((f: any) => f.flag === "--separator");
		expect(sepFlag).toBeDefined();
		expect(sepFlag.type).toBe("space|newline|none");
		expect(sepFlag.required).toBe(false);
	});

	test("GET /list/<toolkit> returns TOOLKIT_NOT_FOUND for unknown toolkit", async () => {
		const res = await fetch(`http://127.0.0.1:${port}/list/nope`);
		const body = await res.json();
		expect(body.ok).toBe(false);
		expect(body.error).toBe("TOOLKIT_NOT_FOUND");
		expect(body.message).toContain("nope");
	});

	test("GET /describe/<toolkit> returns all tools with flags and examples", async () => {
		const res = await fetch(`http://127.0.0.1:${port}/describe/math`);
		expect(res.status).toBe(200);
		const body = await res.json();
		expect(body.ok).toBe(true);
		const result = body.result;
		expect(result.name).toBe("math");
		expect(result.description).toBe("Math utilities");

		const add = result.tools.add;
		expect(add.description).toBe("Add two numbers");
		expect(add.flags).toHaveLength(2);
		expect(add.flags).toContainEqual({
			flag: "--a",
			type: "number",
			required: true,
		});
		expect(add.flags).toContainEqual({
			flag: "--b",
			type: "number",
			required: true,
		});
		expect(add.examples).toHaveLength(1);
		expect(add.examples[0].description).toBe("Add 1 and 2");
		expect(add.examples[0].input).toEqual({ a: 1, b: 2 });
	});

	test("GET /describe/<toolkit>/<tool> returns full tool schema", async () => {
		const res = await fetch(`http://127.0.0.1:${port}/describe/text/upper`);
		expect(res.status).toBe(200);
		const body = await res.json();
		expect(body.ok).toBe(true);
		const result = body.result;
		expect(result.toolkit).toBe("text");
		expect(result.tool).toBe("upper");
		expect(result.description).toBe("Convert text to uppercase");
		expect(result.flags).toContainEqual({
			flag: "--input",
			type: "string",
			required: true,
			description: "Text to convert",
		});
		expect(result.flags).toContainEqual({
			flag: "--trim",
			type: "boolean",
			required: false,
		});
	});

	test("GET /describe/<toolkit>/<tool> returns TOOL_NOT_FOUND for unknown tool", async () => {
		const res = await fetch(
			`http://127.0.0.1:${port}/describe/math/nonexistent`,
		);
		const body = await res.json();
		expect(body.ok).toBe(false);
		expect(body.error).toBe("TOOL_NOT_FOUND");
		expect(body.message).toContain("nonexistent");
		expect(body.message).toContain("add");
	});

	test("GET /describe/<toolkit>/<tool> returns TOOLKIT_NOT_FOUND for unknown toolkit", async () => {
		const res = await fetch(
			`http://127.0.0.1:${port}/describe/nope/whatever`,
		);
		const body = await res.json();
		expect(body.ok).toBe(false);
		expect(body.error).toBe("TOOLKIT_NOT_FOUND");
	});
});

describe.skipIf(!hasRegistryCommands)("host tools RPC server (VM integration)", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create({
			software: REGISTRY_SOFTWARE,
			toolKits: [testToolKit],
		});
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("exec node script from inside VM to call RPC server", async () => {
		const script = `
const http = require('http');
const port = process.env.AGENTOS_TOOLS_PORT;
const body = JSON.stringify({ toolkit: 'math', tool: 'add', input: { a: 10, b: 20 } });
const req = http.request({
  hostname: '127.0.0.1',
  port: Number(port),
  path: '/call',
  method: 'POST',
  headers: { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(body) },
}, (res) => {
  let data = '';
  res.on('data', (chunk) => data += chunk);
  res.on('end', () => process.stdout.write(data));
});
req.write(body);
req.end();
`;
		await vm.writeFile("/tmp/call-tool.js", script);
		const result = await vm.exec("node /tmp/call-tool.js");
		expect(result.exitCode).toBe(0);
		const body = JSON.parse(result.stdout);
		expect(body).toEqual({ ok: true, result: { sum: 30 } });
	});
});

describe.skipIf(!hasRegistryCommands)("host tools CLI commands (VM integration)", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create({
			software: REGISTRY_SOFTWARE,
			toolKits: [testToolKit, textToolKit],
		});
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("agentos list-tools shows both toolkits", async () => {
		const result = await vm.exec("agentos list-tools");
		expect(result.exitCode).toBe(0);
		const body = JSON.parse(result.stdout);
		expect(body.ok).toBe(true);
		const names = body.result.toolkits.map((t: any) => t.name);
		expect(names).toContain("math");
		expect(names).toContain("text");
	});

	test("agentos list-tools <name> shows tools with flag details", async () => {
		const result = await vm.exec("agentos list-tools text");
		expect(result.exitCode).toBe(0);
		const body = JSON.parse(result.stdout);
		expect(body.ok).toBe(true);
		expect(body.result.name).toBe("text");
		expect(body.result.tools.upper).toBeDefined();
		expect(body.result.tools.upper.flags.length).toBeGreaterThan(0);
	});

	test("agentos-{name} --help shows toolkit description", async () => {
		const result = await vm.exec("agentos-math --help");
		expect(result.exitCode).toBe(0);
		const body = JSON.parse(result.stdout);
		expect(body.ok).toBe(true);
		expect(body.result.name).toBe("math");
		expect(body.result.tools.add).toBeDefined();
	});

	test("agentos-{name} <tool> --help shows tool flags", async () => {
		const result = await vm.exec("agentos-text upper --help");
		expect(result.exitCode).toBe(0);
		const body = JSON.parse(result.stdout);
		expect(body.ok).toBe(true);
		expect(body.result.tool).toBe("upper");
		expect(body.result.description).toBe("Convert text to uppercase");
		const flags = body.result.flags;
		expect(flags.find((f: any) => f.flag === "--input")).toBeDefined();
	});
});

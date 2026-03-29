import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { z } from "zod";
import {
	AgentOs,
	generateMasterShim,
	generateToolkitShim,
	hostTool,
	toolKit,
} from "../src/index.js";
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
		}),
	},
});

const secondToolKit = toolKit({
	name: "text",
	description: "Text utilities",
	tools: {
		upper: hostTool({
			description: "Convert to uppercase",
			inputSchema: z.object({ text: z.string() }),
			execute: ({ text }) => ({ result: text.toUpperCase() }),
		}),
	},
});

describe("shim script generation", () => {
	test("generateToolkitShim includes toolkit name", () => {
		const shim = generateToolkitShim("math");
		expect(shim).toContain("#!/bin/sh");
		expect(shim).toContain('TOOLKIT="math"');
		expect(shim).toContain("AGENTOS_TOOLS_PORT");
	});

	test("generateMasterShim includes list-tools", () => {
		const shim = generateMasterShim();
		expect(shim).toContain("#!/bin/sh");
		expect(shim).toContain("list-tools");
		expect(shim).toContain("AGENTOS_TOOLS_PORT");
	});
});

describe.skipIf(!hasRegistryCommands)("CLI shims (VM integration)", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create({
			software: REGISTRY_SOFTWARE,
			toolKits: [testToolKit, secondToolKit],
		});
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("shim files exist at /usr/local/bin/", async () => {
		expect(await vm.exists("/usr/local/bin/agentos")).toBe(true);
		expect(await vm.exists("/usr/local/bin/agentos-math")).toBe(true);
		expect(await vm.exists("/usr/local/bin/agentos-text")).toBe(true);
	});

	test("shim files are executable", async () => {
		const stat = await vm.stat("/usr/local/bin/agentos-math");
		// Check that execute bit is set (mode & 0o111)
		expect(stat.mode & 0o111).toBeGreaterThan(0);
	});

	test("agentos-math add --json executes tool and returns result", async () => {
		const result = await vm.exec(
			'agentos-math add --json \'{"a":2,"b":3}\'',
		);
		expect(result.exitCode).toBe(0);
		const body = JSON.parse(result.stdout.trim());
		expect(body).toEqual({ ok: true, result: { sum: 5 } });
	});

	test("agentos-text upper --json executes tool and returns result", async () => {
		const result = await vm.exec(
			'agentos-text upper --json \'{"text":"hello"}\'',
		);
		expect(result.exitCode).toBe(0);
		const body = JSON.parse(result.stdout.trim());
		expect(body).toEqual({ ok: true, result: { result: "HELLO" } });
	});

	test("--json-file reads input from file", async () => {
		await vm.writeFile(
			"/tmp/input.json",
			JSON.stringify({ a: 10, b: 20 }),
		);
		const result = await vm.exec(
			"agentos-math add --json-file /tmp/input.json",
		);
		expect(result.exitCode).toBe(0);
		const body = JSON.parse(result.stdout.trim());
		expect(body).toEqual({ ok: true, result: { sum: 30 } });
	});

	test("stdin pipe sends input", async () => {
		const result = await vm.exec(
			'echo \'{"a":5,"b":7}\' | agentos-math add',
		);
		expect(result.exitCode).toBe(0);
		const body = JSON.parse(result.stdout.trim());
		expect(body).toEqual({ ok: true, result: { sum: 12 } });
	});

	test("missing AGENTOS_TOOLS_PORT returns INTERNAL_ERROR", async () => {
		// Run the shim with AGENTOS_TOOLS_PORT unset
		const result = await vm.exec(
			'AGENTOS_TOOLS_PORT= agentos-math add --json \'{"a":1,"b":2}\'',
		);
		expect(result.exitCode).toBe(1);
		const body = JSON.parse(result.stdout.trim());
		expect(body.ok).toBe(false);
		expect(body.error).toBe("INTERNAL_ERROR");
		expect(body.message).toContain("AGENTOS_TOOLS_PORT");
	});

	test("unreachable server returns INTERNAL_ERROR", async () => {
		// Use a bogus port that nothing listens on
		const result = await vm.exec(
			'AGENTOS_TOOLS_PORT=1 agentos-math add --json \'{"a":1,"b":2}\'',
		);
		const body = JSON.parse(result.stdout.trim());
		expect(body.ok).toBe(false);
		expect(body.error).toBe("INTERNAL_ERROR");
	});

	test("master agentos --help prints usage", async () => {
		const result = await vm.exec("agentos --help");
		expect(result.exitCode).toBe(0);
		expect(result.stdout).toContain("list-tools");
	});
});

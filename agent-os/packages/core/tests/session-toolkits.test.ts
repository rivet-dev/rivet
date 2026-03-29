import { afterEach, describe, expect, test } from "vitest";
import { z } from "zod";
import { AgentOs, generateToolReference, hostTool, toolKit } from "../src/index.js";

const toolkitAlpha = toolKit({
	name: "alpha",
	description: "Alpha toolkit",
	tools: {
		greet: hostTool({
			description: "Greet someone",
			inputSchema: z.object({ name: z.string() }),
			execute: ({ name }) => ({ greeting: `Hello, ${name}!` }),
		}),
	},
});

const toolkitBeta = toolKit({
	name: "beta",
	description: "Beta toolkit",
	tools: {
		add: hostTool({
			description: "Add two numbers",
			inputSchema: z.object({ a: z.number(), b: z.number() }),
			execute: ({ a, b }) => ({ sum: a + b }),
		}),
	},
});

// Toolkit that collides with alpha by name but has different behavior
const toolkitAlphaOverride = toolKit({
	name: "alpha",
	description: "Overridden alpha toolkit",
	tools: {
		greet: hostTool({
			description: "Greet someone (override)",
			inputSchema: z.object({ name: z.string() }),
			execute: ({ name }) => ({ greeting: `Hi, ${name}!` }),
		}),
	},
});

describe("session-level toolkits", () => {
	let vm: AgentOs;

	afterEach(async () => {
		await vm.dispose();
	});

	test("create VM without toolkits, add session toolkit, tool is callable", async () => {
		vm = await AgentOs.create();

		// No server running initially
		expect((vm as any)._env.AGENTOS_TOOLS_PORT).toBeUndefined();

		// Set up session toolkits (calls the same logic as createSession)
		const combined = await (vm as any)._prepareSessionToolkits([
			toolkitAlpha,
		]);

		// Server should now be running
		const port = Number((vm as any)._env.AGENTOS_TOOLS_PORT);
		expect(port).toBeGreaterThan(0);

		// Combined list should contain only the session toolkit
		expect(combined).toHaveLength(1);
		expect(combined[0].name).toBe("alpha");

		// Tool should be callable via RPC
		const res = await fetch(`http://127.0.0.1:${port}/call`, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				toolkit: "alpha",
				tool: "greet",
				input: { name: "World" },
			}),
		});
		const body = await res.json();
		expect(body).toEqual({
			ok: true,
			result: { greeting: "Hello, World!" },
		});
	});

	test("create VM with toolkit A, add session toolkit B, both accessible", async () => {
		vm = await AgentOs.create({ toolKits: [toolkitAlpha] });
		const port = Number((vm as any)._env.AGENTOS_TOOLS_PORT);
		expect(port).toBeGreaterThan(0);

		// Add session toolkit B
		const combined = await (vm as any)._prepareSessionToolkits([
			toolkitBeta,
		]);

		// Combined list should contain both toolkits
		expect(combined).toHaveLength(2);
		const names = combined.map((tk: any) => tk.name);
		expect(names).toContain("alpha");
		expect(names).toContain("beta");

		// Toolkit A should still be callable
		let res = await fetch(`http://127.0.0.1:${port}/call`, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				toolkit: "alpha",
				tool: "greet",
				input: { name: "Test" },
			}),
		});
		let body = await res.json();
		expect(body.ok).toBe(true);
		expect(body.result).toEqual({ greeting: "Hello, Test!" });

		// Toolkit B should also be callable on the same server
		res = await fetch(`http://127.0.0.1:${port}/call`, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				toolkit: "beta",
				tool: "add",
				input: { a: 3, b: 4 },
			}),
		});
		body = await res.json();
		expect(body.ok).toBe(true);
		expect(body.result).toEqual({ sum: 7 });
	});

	test("session toolkit overrides VM-level toolkit on name collision", async () => {
		vm = await AgentOs.create({ toolKits: [toolkitAlpha] });
		const port = Number((vm as any)._env.AGENTOS_TOOLS_PORT);

		// Original alpha returns "Hello, X!"
		let res = await fetch(`http://127.0.0.1:${port}/call`, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				toolkit: "alpha",
				tool: "greet",
				input: { name: "Before" },
			}),
		});
		let body = await res.json();
		expect(body.result).toEqual({ greeting: "Hello, Before!" });

		// Override with session toolkit that returns "Hi, X!"
		const combined = await (vm as any)._prepareSessionToolkits([
			toolkitAlphaOverride,
		]);

		// Combined list should have only one alpha (the override)
		expect(combined).toHaveLength(1);
		expect(combined[0].description).toBe("Overridden alpha toolkit");

		// Now alpha should use the overridden implementation
		res = await fetch(`http://127.0.0.1:${port}/call`, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				toolkit: "alpha",
				tool: "greet",
				input: { name: "After" },
			}),
		});
		body = await res.json();
		expect(body.result).toEqual({ greeting: "Hi, After!" });
	});

	test("prompt injection includes both VM-level and session-level toolkits", async () => {
		vm = await AgentOs.create({ toolKits: [toolkitAlpha] });

		const combined = await (vm as any)._prepareSessionToolkits([
			toolkitBeta,
		]);

		const reference = generateToolReference(combined);
		expect(reference).toContain("alpha");
		expect(reference).toContain("Greet someone");
		expect(reference).toContain("beta");
		expect(reference).toContain("Add two numbers");
		expect(reference).toContain("agentos-alpha");
		expect(reference).toContain("agentos-beta");
	});

	test("no-op when session has no toolkits", async () => {
		vm = await AgentOs.create();

		const combined = await (vm as any)._prepareSessionToolkits([]);

		expect(combined).toHaveLength(0);
		expect((vm as any)._env.AGENTOS_TOOLS_PORT).toBeUndefined();
	});

	test("VM with no toolkits returns VM toolkits when session has none", async () => {
		vm = await AgentOs.create({ toolKits: [toolkitAlpha] });

		const combined = await (vm as any)._prepareSessionToolkits([]);

		expect(combined).toHaveLength(1);
		expect(combined[0].name).toBe("alpha");
	});
});

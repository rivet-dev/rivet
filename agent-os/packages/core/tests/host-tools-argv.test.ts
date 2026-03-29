import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { z } from "zod";
import { AgentOs, hostTool, parseArgv, toolKit } from "../src/index.js";

// ── Unit tests for parseArgv ──

describe("parseArgv", () => {
	test("string fields: --name value", () => {
		const schema = z.object({ name: z.string(), path: z.string() });
		const result = parseArgv(schema, [
			"--name",
			"hello",
			"--path",
			"/tmp/shot.png",
		]);
		expect(result).toEqual({
			ok: true,
			input: { name: "hello", path: "/tmp/shot.png" },
		});
	});

	test("number fields: --limit 5", () => {
		const schema = z.object({ a: z.number(), b: z.number() });
		const result = parseArgv(schema, ["--a", "2", "--b", "3"]);
		expect(result).toEqual({ ok: true, input: { a: 2, b: 3 } });
	});

	test("boolean fields: --full-page", () => {
		const schema = z.object({ fullPage: z.boolean() });
		const result = parseArgv(schema, ["--full-page"]);
		expect(result).toEqual({ ok: true, input: { fullPage: true } });
	});

	test("boolean fields with --no- prefix: --no-full-page", () => {
		const schema = z.object({ fullPage: z.boolean() });
		const result = parseArgv(schema, ["--no-full-page"]);
		expect(result).toEqual({ ok: true, input: { fullPage: false } });
	});

	test("enum fields: --format json", () => {
		const schema = z.object({ format: z.enum(["json", "text", "html"]) });
		const result = parseArgv(schema, ["--format", "json"]);
		expect(result).toEqual({ ok: true, input: { format: "json" } });
	});

	test("array fields: --tags foo --tags bar", () => {
		const schema = z.object({ tags: z.array(z.string()) });
		const result = parseArgv(schema, ["--tags", "foo", "--tags", "bar"]);
		expect(result).toEqual({ ok: true, input: { tags: ["foo", "bar"] } });
	});

	test("optional fields omitted from argv are undefined", () => {
		const schema = z.object({
			name: z.string(),
			description: z.string().optional(),
		});
		const result = parseArgv(schema, ["--name", "test"]);
		expect(result).toEqual({ ok: true, input: { name: "test" } });
	});

	test("camelCase to kebab-case mapping", () => {
		const schema = z.object({
			fullPage: z.boolean(),
			maxRetries: z.number(),
		});
		const result = parseArgv(schema, ["--full-page", "--max-retries", "3"]);
		expect(result).toEqual({
			ok: true,
			input: { fullPage: true, maxRetries: 3 },
		});
	});

	test("unknown flags return error", () => {
		const schema = z.object({ name: z.string() });
		const result = parseArgv(schema, [
			"--name",
			"test",
			"--unknown",
			"value",
		]);
		expect(result.ok).toBe(false);
		if (!result.ok) {
			expect(result.message).toContain("Unknown flag");
			expect(result.message).toContain("--unknown");
		}
	});

	test("missing required fields return error with field name", () => {
		const schema = z.object({ name: z.string(), path: z.string() });
		const result = parseArgv(schema, ["--name", "test"]);
		expect(result.ok).toBe(false);
		if (!result.ok) {
			expect(result.message).toContain("Missing required flag");
			expect(result.message).toContain("--path");
		}
	});

	test("empty argv with empty schema succeeds", () => {
		const schema = z.object({});
		const result = parseArgv(schema, []);
		expect(result).toEqual({ ok: true, input: {} });
	});

	test("number array fields", () => {
		const schema = z.object({ scores: z.array(z.number()) });
		const result = parseArgv(schema, [
			"--scores",
			"1",
			"--scores",
			"2",
			"--scores",
			"3",
		]);
		expect(result).toEqual({ ok: true, input: { scores: [1, 2, 3] } });
	});

	test("invalid number returns error", () => {
		const schema = z.object({ count: z.number() });
		const result = parseArgv(schema, ["--count", "abc"]);
		expect(result.ok).toBe(false);
		if (!result.ok) {
			expect(result.message).toContain("expects a number");
		}
	});

	test("flag without value returns error", () => {
		const schema = z.object({ name: z.string() });
		const result = parseArgv(schema, ["--name"]);
		expect(result.ok).toBe(false);
		if (!result.ok) {
			expect(result.message).toContain("requires a value");
		}
	});

	test("positional argument returns error", () => {
		const schema = z.object({ name: z.string() });
		const result = parseArgv(schema, ["hello"]);
		expect(result.ok).toBe(false);
		if (!result.ok) {
			expect(result.message).toContain("positional argument");
		}
	});

	test("mixed types in one command", () => {
		const schema = z.object({
			url: z.string(),
			fullPage: z.boolean().optional(),
			width: z.number().optional(),
			format: z.enum(["png", "jpg"]).optional(),
		});
		const result = parseArgv(schema, [
			"--url",
			"https://example.com",
			"--full-page",
			"--width",
			"1920",
			"--format",
			"png",
		]);
		expect(result).toEqual({
			ok: true,
			input: {
				url: "https://example.com",
				fullPage: true,
				width: 1920,
				format: "png",
			},
		});
	});
});

// ── Integration tests: argv via RPC server ──

describe("host tools RPC server (argv)", () => {
	const browserToolKit = toolKit({
		name: "browser",
		description: "Browser automation tools",
		tools: {
			screenshot: hostTool({
				description: "Take a screenshot",
				inputSchema: z.object({
					url: z.string(),
					fullPage: z.boolean().optional(),
					width: z.number().optional(),
					format: z.enum(["png", "jpg"]).optional(),
					tags: z.array(z.string()).optional(),
				}),
				execute: (input) => ({ captured: true, ...input }),
			}),
		},
	});

	let vm: AgentOs;
	let port: number;

	beforeEach(async () => {
		vm = await AgentOs.create({
			toolKits: [browserToolKit],
		});
		port = Number(vm.kernel.env.AGENTOS_TOOLS_PORT);
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("call tool via flags, verify execute() receives correct parsed input", async () => {
		const res = await fetch(`http://127.0.0.1:${port}/call`, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				toolkit: "browser",
				tool: "screenshot",
				argv: [
					"--url",
					"https://example.com",
					"--full-page",
					"--width",
					"1920",
				],
			}),
		});
		const body = await res.json();
		expect(body.ok).toBe(true);
		expect(body.result).toEqual({
			captured: true,
			url: "https://example.com",
			fullPage: true,
			width: 1920,
		});
	});

	test("boolean flags with --no- prefix via RPC", async () => {
		const res = await fetch(`http://127.0.0.1:${port}/call`, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				toolkit: "browser",
				tool: "screenshot",
				argv: ["--url", "https://example.com", "--no-full-page"],
			}),
		});
		const body = await res.json();
		expect(body.ok).toBe(true);
		expect(body.result.fullPage).toBe(false);
	});

	test("repeated flags for arrays via RPC", async () => {
		const res = await fetch(`http://127.0.0.1:${port}/call`, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				toolkit: "browser",
				tool: "screenshot",
				argv: [
					"--url",
					"https://example.com",
					"--tags",
					"hero",
					"--tags",
					"landing",
				],
			}),
		});
		const body = await res.json();
		expect(body.ok).toBe(true);
		expect(body.result.tags).toEqual(["hero", "landing"]);
	});

	test("missing required flag returns VALIDATION_ERROR", async () => {
		const res = await fetch(`http://127.0.0.1:${port}/call`, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				toolkit: "browser",
				tool: "screenshot",
				argv: ["--full-page"],
			}),
		});
		const body = await res.json();
		expect(body.ok).toBe(false);
		expect(body.error).toBe("VALIDATION_ERROR");
		expect(body.message).toContain("--url");
	});

	test("unknown flag returns VALIDATION_ERROR", async () => {
		const res = await fetch(`http://127.0.0.1:${port}/call`, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				toolkit: "browser",
				tool: "screenshot",
				argv: ["--url", "https://example.com", "--nonexistent", "val"],
			}),
		});
		const body = await res.json();
		expect(body.ok).toBe(false);
		expect(body.error).toBe("VALIDATION_ERROR");
		expect(body.message).toContain("Unknown flag");
	});

	test("input takes precedence when both input and argv absent", async () => {
		const res = await fetch(`http://127.0.0.1:${port}/call`, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				toolkit: "browser",
				tool: "screenshot",
				input: { url: "https://example.com" },
			}),
		});
		const body = await res.json();
		expect(body.ok).toBe(true);
		expect(body.result.url).toBe("https://example.com");
	});
});

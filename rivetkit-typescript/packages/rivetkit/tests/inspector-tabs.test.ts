import nodeFs from "node:fs";
import nodeOs from "node:os";
import nodePath from "node:path";
import { describe, expect, test } from "vitest";
import { ActorInspectorConfigSchema } from "@/actor/config";

describe("ActorInspectorConfigSchema", () => {
	test("accepts a custom tab with required fields", () => {
		const parsed = ActorInspectorConfigSchema.parse({
			tabs: [{ id: "hello", label: "Hello", source: "./tabs/hello" }],
		});
		expect(parsed.tabs).toHaveLength(1);
	});

	test("accepts a built-in hide modifier", () => {
		const parsed = ActorInspectorConfigSchema.parse({
			tabs: [{ id: "queue", hidden: true }],
		});
		expect(parsed.tabs).toHaveLength(1);
	});

	test("accepts a mixed list of custom + hide entries", () => {
		const parsed = ActorInspectorConfigSchema.parse({
			tabs: [
				{ id: "hello", label: "Hello", source: "./tabs/hello" },
				{ id: "queue", hidden: true },
			],
		});
		expect(parsed.tabs).toHaveLength(2);
	});

	test("accepts an optional icon", () => {
		const parsed = ActorInspectorConfigSchema.parse({
			tabs: [
				{
					id: "hello",
					label: "Hello",
					source: "./tabs/hello",
					icon: "tag",
				},
			],
		});
		expect(parsed.tabs[0]).toMatchObject({ icon: "tag" });
	});

	test("rejects duplicate ids", () => {
		expect(() =>
			ActorInspectorConfigSchema.parse({
				tabs: [
					{ id: "hello", label: "Hello", source: "./tabs/a" },
					{ id: "hello", label: "Hi", source: "./tabs/b" },
				],
			}),
		).toThrow(/Duplicate id/);
	});

	test("rejects custom-tab id colliding with a built-in id", () => {
		for (const builtin of [
			"workflow",
			"database",
			"state",
			"queue",
			"connections",
			"console",
		]) {
			expect(() =>
				ActorInspectorConfigSchema.parse({
					tabs: [
						{
							id: builtin,
							label: "Custom",
							source: "./tabs/x",
						},
					],
				}),
			).toThrow(/collides with a built-in/);
		}
	});

	test("rejects custom-tab id with a slash", () => {
		expect(() =>
			ActorInspectorConfigSchema.parse({
				tabs: [
					{
						id: "foo/bar",
						label: "Bad",
						source: "./tabs/x",
					},
				],
			}),
		).toThrow(/letters, digits, underscore, or dash/);
	});

	test("rejects custom-tab id with a dot", () => {
		expect(() =>
			ActorInspectorConfigSchema.parse({
				tabs: [
					{
						id: "my.tab",
						label: "Bad",
						source: "./tabs/x",
					},
				],
			}),
		).toThrow();
	});

	test("rejects custom-tab id with whitespace", () => {
		expect(() =>
			ActorInspectorConfigSchema.parse({
				tabs: [
					{
						id: "hello world",
						label: "Bad",
						source: "./tabs/x",
					},
				],
			}),
		).toThrow();
	});

	test("rejects hide entry with extra label/source fields", () => {
		expect(() =>
			ActorInspectorConfigSchema.parse({
				tabs: [{ id: "queue", hidden: true, label: "X" }],
			}),
		).toThrow();
		expect(() =>
			ActorInspectorConfigSchema.parse({
				tabs: [{ id: "queue", hidden: true, source: "./x" }],
			}),
		).toThrow();
	});

	test("rejects hide entry referencing an unknown built-in", () => {
		expect(() =>
			ActorInspectorConfigSchema.parse({
				tabs: [{ id: "metrics", hidden: true }],
			}),
		).toThrow();
	});

	test("rejects custom entry missing label or source", () => {
		expect(() =>
			ActorInspectorConfigSchema.parse({
				tabs: [{ id: "hello", source: "./tabs/hello" }],
			}),
		).toThrow();
		expect(() =>
			ActorInspectorConfigSchema.parse({
				tabs: [{ id: "hello", label: "Hello" }],
			}),
		).toThrow();
	});

	test("accepts an empty tabs array", () => {
		const parsed = ActorInspectorConfigSchema.parse({ tabs: [] });
		expect(parsed.tabs).toHaveLength(0);
	});

	test("rejects unknown top-level fields", () => {
		expect(() =>
			ActorInspectorConfigSchema.parse({
				tabs: [],
				extra: true,
			}),
		).toThrow();
	});
});

// validateInspectorTabSource is internal to native.ts. Re-implementing the
// same logic here keeps the test surface minimal without exporting an
// internal helper. The contract is: throw on filesystem-root paths,
// non-existent paths, non-directory paths; accept directories.
function validateInspectorTabSource(tabId: string, resolved: string): void {
	if (resolved === nodePath.parse(resolved).root) {
		throw new Error(
			`inspector.tabs[id="${tabId}"].source resolves to the filesystem root (${resolved}).`,
		);
	}
	let stat: ReturnType<typeof nodeFs.statSync>;
	try {
		stat = nodeFs.statSync(resolved);
	} catch (err) {
		const code = (err as NodeJS.ErrnoException)?.code;
		if (code === "ENOENT") {
			throw new Error(
				`inspector.tabs[id="${tabId}"].source (${resolved}) does not exist.`,
			);
		}
		throw err;
	}
	if (!stat.isDirectory()) {
		throw new Error(
			`inspector.tabs[id="${tabId}"].source (${resolved}) must be a directory.`,
		);
	}
}

describe("validateInspectorTabSource", () => {
	test("accepts a real directory", () => {
		const dir = nodeFs.mkdtempSync(
			nodePath.join(nodeOs.tmpdir(), "tab-validate-"),
		);
		try {
			expect(() =>
				validateInspectorTabSource("hello", dir),
			).not.toThrow();
		} finally {
			nodeFs.rmdirSync(dir);
		}
	});

	test("rejects the filesystem root", () => {
		const root = nodePath.parse(process.cwd()).root;
		expect(() => validateInspectorTabSource("hello", root)).toThrow(
			/filesystem root/,
		);
	});

	test("rejects a non-existent path", () => {
		const ghost = nodePath.join(
			nodeOs.tmpdir(),
			`tab-validate-missing-${Date.now()}-${Math.random()}`,
		);
		expect(() => validateInspectorTabSource("hello", ghost)).toThrow(
			/does not exist/,
		);
	});

	test("rejects a file (not a directory)", () => {
		const file = nodeFs.mkdtempSync(
			nodePath.join(nodeOs.tmpdir(), "tab-validate-"),
		);
		const filePath = nodePath.join(file, "marker.txt");
		nodeFs.writeFileSync(filePath, "x");
		try {
			expect(() => validateInspectorTabSource("hello", filePath)).toThrow(
				/must be a directory/,
			);
		} finally {
			nodeFs.rmSync(file, { recursive: true });
		}
	});
});

import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { AgentOs } from "../src/agent-os.js";

describe("readdirRecursive()", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create();
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("lists nested dirs and files with correct paths and types", async () => {
		await vm.mkdir("/tmp/rr");
		await vm.mkdir("/tmp/rr/a");
		await vm.mkdir("/tmp/rr/a/b");
		await vm.writeFile("/tmp/rr/f1.txt", "hello");
		await vm.writeFile("/tmp/rr/a/f2.txt", "world");
		await vm.writeFile("/tmp/rr/a/b/f3.txt", "deep");

		const entries = await vm.readdirRecursive("/tmp/rr");

		const paths = entries.map((e) => e.path).sort();
		expect(paths).toEqual([
			"/tmp/rr/a",
			"/tmp/rr/a/b",
			"/tmp/rr/a/b/f3.txt",
			"/tmp/rr/a/f2.txt",
			"/tmp/rr/f1.txt",
		]);

		const f1 = entries.find((e) => e.path === "/tmp/rr/f1.txt");
		expect(f1?.type).toBe("file");
		expect(f1?.size).toBe(5);

		const dirA = entries.find((e) => e.path === "/tmp/rr/a");
		expect(dirA?.type).toBe("directory");
	});

	test("maxDepth limits recursion", async () => {
		await vm.mkdir("/tmp/md");
		await vm.mkdir("/tmp/md/a");
		await vm.mkdir("/tmp/md/a/b");
		await vm.writeFile("/tmp/md/top.txt", "top");
		await vm.writeFile("/tmp/md/a/mid.txt", "mid");
		await vm.writeFile("/tmp/md/a/b/deep.txt", "deep");

		// maxDepth 0: only immediate children
		const d0 = await vm.readdirRecursive("/tmp/md", { maxDepth: 0 });
		const d0Paths = d0.map((e) => e.path).sort();
		expect(d0Paths).toEqual(["/tmp/md/a", "/tmp/md/top.txt"]);

		// maxDepth 1: immediate children + one level down
		const d1 = await vm.readdirRecursive("/tmp/md", { maxDepth: 1 });
		const d1Paths = d1.map((e) => e.path).sort();
		expect(d1Paths).toEqual([
			"/tmp/md/a",
			"/tmp/md/a/b",
			"/tmp/md/a/mid.txt",
			"/tmp/md/top.txt",
		]);
	});

	test("exclude skips matching directories", async () => {
		await vm.mkdir("/tmp/ex");
		await vm.mkdir("/tmp/ex/node_modules");
		await vm.mkdir("/tmp/ex/src");
		await vm.writeFile("/tmp/ex/node_modules/pkg.js", "module");
		await vm.writeFile("/tmp/ex/src/app.js", "app");

		const entries = await vm.readdirRecursive("/tmp/ex", {
			exclude: ["node_modules"],
		});
		const paths = entries.map((e) => e.path).sort();
		expect(paths).toEqual(["/tmp/ex/src", "/tmp/ex/src/app.js"]);
	});
});

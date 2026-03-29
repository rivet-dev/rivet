import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { AgentOs } from "../src/index.js";

describe("filesystem move and delete", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create();
	});

	afterEach(async () => {
		await vm.dispose();
	});

	test("move file to new path", async () => {
		await vm.writeFile("/tmp/move-src.txt", "move me");
		await vm.move("/tmp/move-src.txt", "/tmp/move-dst.txt");
		expect(await vm.exists("/tmp/move-src.txt")).toBe(false);
		const data = await vm.readFile("/tmp/move-dst.txt");
		expect(new TextDecoder().decode(data)).toBe("move me");
	});

	test("move directory", async () => {
		await vm.mkdir("/tmp/movedir");
		await vm.writeFile("/tmp/movedir/child.txt", "child");
		await vm.move("/tmp/movedir", "/tmp/movedir-new");
		expect(await vm.exists("/tmp/movedir")).toBe(false);
		const entries = await vm.readdir("/tmp/movedir-new");
		expect(entries).toContain("child.txt");
		const data = await vm.readFile("/tmp/movedir-new/child.txt");
		expect(new TextDecoder().decode(data)).toBe("child");
	});

	test("delete file", async () => {
		await vm.writeFile("/tmp/delfile.txt", "delete me");
		await vm.delete("/tmp/delfile.txt");
		expect(await vm.exists("/tmp/delfile.txt")).toBe(false);
	});

	test("delete directory recursively", async () => {
		await vm.mkdir("/tmp/deldir");
		await vm.mkdir("/tmp/deldir/sub");
		await vm.writeFile("/tmp/deldir/a.txt", "a");
		await vm.writeFile("/tmp/deldir/sub/b.txt", "b");
		await vm.delete("/tmp/deldir", { recursive: true });
		expect(await vm.exists("/tmp/deldir")).toBe(false);
	});

	test("delete non-existent path throws", async () => {
		await expect(vm.delete("/tmp/no-such-file.txt")).rejects.toThrow();
	});
});

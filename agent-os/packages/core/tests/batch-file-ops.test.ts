import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { AgentOs } from "../src/agent-os.js";

describe("batch file operations", () => {
	let vm: AgentOs;

	beforeEach(async () => {
		vm = await AgentOs.create();
	});

	afterEach(async () => {
		await vm.dispose();
	});

	describe("writeFiles()", () => {
		test("batch write 3 files, all succeed", async () => {
			const results = await vm.writeFiles([
				{ path: "/tmp/batch/a.txt", content: "aaa" },
				{ path: "/tmp/batch/b.txt", content: "bbb" },
				{ path: "/tmp/batch/c.txt", content: "ccc" },
			]);

			expect(results).toHaveLength(3);
			for (const r of results) {
				expect(r.success).toBe(true);
				expect(r.error).toBeUndefined();
			}

			// Verify files exist with correct content
			const a = await vm.readFile("/tmp/batch/a.txt");
			expect(new TextDecoder().decode(a)).toBe("aaa");
			const c = await vm.readFile("/tmp/batch/c.txt");
			expect(new TextDecoder().decode(c)).toBe("ccc");
		});

		test("creates parent directories as needed", async () => {
			const results = await vm.writeFiles([
				{ path: "/tmp/deep/nested/dir/file.txt", content: "deep" },
			]);

			expect(results[0].success).toBe(true);
			const content = await vm.readFile("/tmp/deep/nested/dir/file.txt");
			expect(new TextDecoder().decode(content)).toBe("deep");
		});

		test("partial failure: one bad path still writes others", async () => {
			// Write to /dev/null dir (not writable as a file) should fail
			// but /proc is read-only, so writing there should fail
			const results = await vm.writeFiles([
				{ path: "/tmp/ok.txt", content: "ok" },
				{ path: "/proc/fake-file", content: "fail" },
				{ path: "/tmp/also-ok.txt", content: "also ok" },
			]);

			expect(results[0].success).toBe(true);
			expect(results[1].success).toBe(false);
			expect(results[1].error).toBeDefined();
			expect(results[2].success).toBe(true);
		});
	});

	describe("readFiles()", () => {
		test("batch read existing files", async () => {
			await vm.writeFile("/tmp/r1.txt", "one");
			await vm.writeFile("/tmp/r2.txt", "two");

			const results = await vm.readFiles(["/tmp/r1.txt", "/tmp/r2.txt"]);

			expect(results).toHaveLength(2);
			expect(
				new TextDecoder().decode(results[0].content as Uint8Array),
			).toBe("one");
			expect(
				new TextDecoder().decode(results[1].content as Uint8Array),
			).toBe("two");
			expect(results[0].error).toBeUndefined();
		});

		test("missing file returns null content with error", async () => {
			await vm.writeFile("/tmp/exists.txt", "yes");

			const results = await vm.readFiles([
				"/tmp/exists.txt",
				"/tmp/does-not-exist.txt",
			]);

			expect(results[0].content).not.toBeNull();
			expect(results[1].content).toBeNull();
			expect(results[1].error).toBeDefined();
		});
	});
});

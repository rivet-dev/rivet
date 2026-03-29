import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { AgentOs } from "../src/index.js";

describe("HostDirBackend", () => {
	let vm: AgentOs;
	let tmpDir: string;

	beforeEach(() => {
		tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "host-dir-test-"));
		// Create some known files
		fs.writeFileSync(path.join(tmpDir, "hello.txt"), "hello from host");
		fs.mkdirSync(path.join(tmpDir, "subdir"));
		fs.writeFileSync(
			path.join(tmpDir, "subdir", "nested.txt"),
			"nested content",
		);
	});

	afterEach(async () => {
		if (vm) await vm.dispose();
		fs.rmSync(tmpDir, { recursive: true, force: true });
	});

	test("read file from host directory through backend", async () => {
		vm = await AgentOs.create({
			mounts: [{ path: "/hostmnt", type: "host", hostPath: tmpDir }],
		});
		const data = await vm.readFile("/hostmnt/hello.txt");
		expect(new TextDecoder().decode(data)).toBe("hello from host");
	});

	test("readdir lists host directory contents", async () => {
		vm = await AgentOs.create({
			mounts: [{ path: "/hostmnt", type: "host", hostPath: tmpDir }],
		});
		const entries = await vm.readdir("/hostmnt");
		const filtered = entries.filter((e) => e !== "." && e !== "..");
		expect(filtered).toContain("hello.txt");
		expect(filtered).toContain("subdir");
	});

	test("path traversal attempt (../../etc/passwd) is blocked", async () => {
		vm = await AgentOs.create({
			mounts: [{ path: "/hostmnt", type: "host", hostPath: tmpDir }],
		});
		await expect(
			vm.readFile("/hostmnt/../../etc/passwd"),
		).rejects.toThrow();
	});

	test("symlink escape attempt is blocked", async () => {
		// Create a symlink inside tmpDir that points outside
		const escapePath = path.join(tmpDir, "escape");
		fs.symlinkSync("/etc", escapePath);

		vm = await AgentOs.create({
			mounts: [{ path: "/hostmnt", type: "host", hostPath: tmpDir }],
		});
		await expect(vm.readFile("/hostmnt/escape/hostname")).rejects.toThrow(
			"EACCES",
		);
	});

	test("write blocked when readOnly (default)", async () => {
		vm = await AgentOs.create({
			mounts: [{ path: "/hostmnt", type: "host", hostPath: tmpDir }],
		});
		await expect(
			vm.writeFile("/hostmnt/new.txt", "should fail"),
		).rejects.toThrow("EROFS");
	});

	test("write works when readOnly: false", async () => {
		vm = await AgentOs.create({
			mounts: [
				{
					path: "/hostmnt",
					type: "host",
					hostPath: tmpDir,
					readOnly: false,
				},
			],
		});
		await vm.writeFile("/hostmnt/writable.txt", "written from VM");

		// Verify on host
		const content = fs.readFileSync(
			path.join(tmpDir, "writable.txt"),
			"utf-8",
		);
		expect(content).toBe("written from VM");
	});
});

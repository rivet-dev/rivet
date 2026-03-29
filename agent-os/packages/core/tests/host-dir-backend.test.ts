import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { createHostDirBackend } from "../src/backends/host-dir-backend.js";
import { defineFsDriverTests } from "../src/test/file-system.js";
import { AgentOs } from "../src/index.js";

// ---------------------------------------------------------------------------
// Shared VFS conformance tests
// ---------------------------------------------------------------------------

let conformanceTmpDir: string;

defineFsDriverTests({
	name: "HostDirBackend",
	createFs: () => {
		conformanceTmpDir = fs.mkdtempSync(
			path.join(os.tmpdir(), "host-dir-test-"),
		);
		return createHostDirBackend({
			hostPath: conformanceTmpDir,
			readOnly: false,
		});
	},
	cleanup: () => {
		if (conformanceTmpDir)
			fs.rmSync(conformanceTmpDir, { recursive: true, force: true });
	},
	capabilities: {
		symlinks: false,
		hardLinks: false,
		permissions: true,
		utimes: true,
		truncate: true,
		pread: true,
		mkdir: true,
		removeDir: true,
	},
});

// ---------------------------------------------------------------------------
// Host-dir-specific tests (security, read-only)
// ---------------------------------------------------------------------------

describe("HostDirBackend (security)", () => {
	let vm: AgentOs;
	let tmpDir: string;

	beforeEach(() => {
		tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "host-dir-test-"));
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

	test("path traversal attempt (../../etc/passwd) is blocked", async () => {
		vm = await AgentOs.create({
			mounts: [{ path: "/hostmnt", driver: createHostDirBackend({ hostPath: tmpDir }) }],
		});
		await expect(
			vm.readFile("/hostmnt/../../etc/passwd"),
		).rejects.toThrow();
	});

	test("symlink escape attempt is blocked", async () => {
		const escapePath = path.join(tmpDir, "escape");
		fs.symlinkSync("/etc", escapePath);

		vm = await AgentOs.create({
			mounts: [{ path: "/hostmnt", driver: createHostDirBackend({ hostPath: tmpDir }) }],
		});
		await expect(vm.readFile("/hostmnt/escape/hostname")).rejects.toThrow(
			"EACCES",
		);
	});

	test("write blocked when readOnly", async () => {
		vm = await AgentOs.create({
			mounts: [{ path: "/hostmnt", driver: createHostDirBackend({ hostPath: tmpDir }), readOnly: true }],
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
					driver: createHostDirBackend({ hostPath: tmpDir, readOnly: false }),
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

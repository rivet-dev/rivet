import type { VirtualFileSystem } from "@secure-exec/core";
import { createInMemoryFileSystem } from "@secure-exec/core";
import { beforeEach, describe, expect, test } from "vitest";
import { createOverlayBackend } from "../src/backends/overlay-backend.js";
import { defineFsDriverTests } from "../src/test/file-system.js";

// ---------------------------------------------------------------------------
// Shared VFS conformance tests
// ---------------------------------------------------------------------------

defineFsDriverTests({
	name: "OverlayBackend",
	createFs: () => {
		const lower = createInMemoryFileSystem();
		return createOverlayBackend({ lower });
	},
	capabilities: {
		symlinks: true,
		hardLinks: true,
		permissions: true,
		utimes: false,
		truncate: true,
		pread: true,
		mkdir: true,
		removeDir: true,
	},
});

// ---------------------------------------------------------------------------
// Overlay-specific tests (layer isolation, whiteouts)
// ---------------------------------------------------------------------------

describe("OverlayBackend (layer behavior)", () => {
	let lower: VirtualFileSystem;
	let upper: VirtualFileSystem;
	let overlay: VirtualFileSystem;

	beforeEach(async () => {
		lower = createInMemoryFileSystem();
		upper = createInMemoryFileSystem();

		await lower.mkdir("/data", { recursive: true });
		await lower.writeFile("/data/base.txt", "base content");
		await lower.writeFile("/data/shared.txt", "from lower");
		await lower.mkdir("/data/subdir", { recursive: true });
		await lower.writeFile("/data/subdir/nested.txt", "nested in lower");

		overlay = createOverlayBackend({ lower, upper });
	});

	test("read from lower when upper doesn't have file", async () => {
		const data = await overlay.readFile("/data/base.txt");
		expect(new TextDecoder().decode(data)).toBe("base content");
	});

	test("read text from lower when upper doesn't have file", async () => {
		const text = await overlay.readTextFile("/data/base.txt");
		expect(text).toBe("base content");
	});

	test("write goes to upper, subsequent read comes from upper", async () => {
		await overlay.writeFile("/data/shared.txt", "from upper");
		const text = await overlay.readTextFile("/data/shared.txt");
		expect(text).toBe("from upper");
	});

	test("write to upper doesn't modify lower", async () => {
		await overlay.writeFile("/data/shared.txt", "overwritten");

		const overlayText = await overlay.readTextFile("/data/shared.txt");
		expect(overlayText).toBe("overwritten");

		const lowerText = await lower.readTextFile("/data/shared.txt");
		expect(lowerText).toBe("from lower");
	});

	test("delete creates whiteout, file no longer visible via exists", async () => {
		expect(await overlay.exists("/data/base.txt")).toBe(true);
		await overlay.removeFile("/data/base.txt");
		expect(await overlay.exists("/data/base.txt")).toBe(false);
		expect(await lower.exists("/data/base.txt")).toBe(true);
	});

	test("delete creates whiteout, readFile throws ENOENT", async () => {
		await overlay.removeFile("/data/base.txt");
		await expect(overlay.readFile("/data/base.txt")).rejects.toThrow(
			"ENOENT",
		);
	});

	test("delete creates whiteout, stat throws ENOENT", async () => {
		await overlay.removeFile("/data/base.txt");
		await expect(overlay.stat("/data/base.txt")).rejects.toThrow("ENOENT");
	});

	test("readdir merges both layers and excludes whiteouts", async () => {
		await overlay.mkdir("/data", { recursive: true });
		await overlay.writeFile("/data/upper-only.txt", "upper only");
		await overlay.removeFile("/data/base.txt");

		const entries = await overlay.readDir("/data");

		expect(entries).toContain("shared.txt");
		expect(entries).toContain("subdir");
		expect(entries).toContain("upper-only.txt");
		expect(entries).not.toContain("base.txt");
	});

	test("readDirWithTypes merges both layers", async () => {
		await overlay.writeFile("/data/extra.txt", "extra");
		const entries = await overlay.readDirWithTypes("/data");

		const names = entries.map((e) => e.name);
		expect(names).toContain("base.txt");
		expect(names).toContain("shared.txt");
		expect(names).toContain("subdir");
		expect(names).toContain("extra.txt");

		const subdirEntry = entries.find((e) => e.name === "subdir");
		expect(subdirEntry?.isDirectory).toBe(true);
	});

	test("write after delete (whiteout) restores visibility", async () => {
		await overlay.removeFile("/data/base.txt");
		expect(await overlay.exists("/data/base.txt")).toBe(false);

		await overlay.writeFile("/data/base.txt", "resurrected");
		expect(await overlay.exists("/data/base.txt")).toBe(true);

		const text = await overlay.readTextFile("/data/base.txt");
		expect(text).toBe("resurrected");
	});

	test("pread falls through to lower", async () => {
		const chunk = await overlay.pread("/data/base.txt", 5, 6);
		expect(new TextDecoder().decode(chunk)).toBe("conten");
	});

	test("defaults upper to in-memory filesystem", async () => {
		const overlayDefault = createOverlayBackend({ lower });
		await overlayDefault.writeFile("/data/new.txt", "written");
		const text = await overlayDefault.readTextFile("/data/new.txt");
		expect(text).toBe("written");

		expect(await lower.exists("/data/new.txt")).toBe(false);
	});
});

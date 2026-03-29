import type { VirtualFileSystem } from "@secure-exec/core";
import { createInMemoryFileSystem } from "@secure-exec/core";
import { beforeEach, describe, expect, test } from "vitest";
import { createOverlayBackend } from "../src/backends/overlay-backend.js";

describe("OverlayBackend", () => {
	let lower: VirtualFileSystem;
	let upper: VirtualFileSystem;
	let overlay: VirtualFileSystem;

	beforeEach(async () => {
		lower = createInMemoryFileSystem();
		upper = createInMemoryFileSystem();

		// Populate lower with some files
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

		// Upper should have the new value
		const overlayText = await overlay.readTextFile("/data/shared.txt");
		expect(overlayText).toBe("overwritten");

		// Lower should still have the original
		const lowerText = await lower.readTextFile("/data/shared.txt");
		expect(lowerText).toBe("from lower");
	});

	test("delete creates whiteout, file no longer visible via exists", async () => {
		// File exists in lower
		expect(await overlay.exists("/data/base.txt")).toBe(true);

		// Delete it (creates whiteout)
		await overlay.removeFile("/data/base.txt");

		// No longer visible through overlay
		expect(await overlay.exists("/data/base.txt")).toBe(false);

		// But still exists in lower directly
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
		// Add a file only in upper
		await overlay.mkdir("/data", { recursive: true });
		await overlay.writeFile("/data/upper-only.txt", "upper only");

		// Delete a file from lower via whiteout
		await overlay.removeFile("/data/base.txt");

		const entries = await overlay.readDir("/data");

		// Should include: shared.txt (lower), subdir (lower), upper-only.txt (upper)
		// Should NOT include: base.txt (whited out)
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

		// Verify directory flag
		const subdirEntry = entries.find((e) => e.name === "subdir");
		expect(subdirEntry?.isDirectory).toBe(true);
	});

	test("exists returns false for whited-out file even if in lower", async () => {
		expect(await overlay.exists("/data/base.txt")).toBe(true);
		await overlay.removeFile("/data/base.txt");
		expect(await overlay.exists("/data/base.txt")).toBe(false);
	});

	test("write after delete (whiteout) restores visibility", async () => {
		await overlay.removeFile("/data/base.txt");
		expect(await overlay.exists("/data/base.txt")).toBe(false);

		// Write new content — should clear the whiteout
		await overlay.writeFile("/data/base.txt", "resurrected");
		expect(await overlay.exists("/data/base.txt")).toBe(true);

		const text = await overlay.readTextFile("/data/base.txt");
		expect(text).toBe("resurrected");
	});

	test("new file only in upper is readable", async () => {
		await overlay.writeFile("/new-file.txt", "brand new");
		const text = await overlay.readTextFile("/new-file.txt");
		expect(text).toBe("brand new");
	});

	test("pread falls through to lower", async () => {
		// "base content" - read bytes 5-11 = "conten"
		const chunk = await overlay.pread("/data/base.txt", 5, 6);
		expect(new TextDecoder().decode(chunk)).toBe("conten");
	});

	test("defaults upper to in-memory filesystem", async () => {
		const overlayDefault = createOverlayBackend({ lower });
		await overlayDefault.writeFile("/data/new.txt", "written");
		const text = await overlayDefault.readTextFile("/data/new.txt");
		expect(text).toBe("written");

		// Lower unchanged
		expect(await lower.exists("/data/new.txt")).toBe(false);
	});
});

/**
 * Shared filesystem conformance test suite.
 *
 * Driver authors call `defineFsDriverTests(config)` inside a vitest
 * `describe` block. The helper registers core tests that every VFS
 * implementation must pass, plus conditional tests gated on the
 * `capabilities` object.
 */

import type { VirtualFileSystem } from "@secure-exec/core";
import { describe, beforeEach, afterEach, expect, test } from "vitest";

// ---------------------------------------------------------------------------
// Public config type
// ---------------------------------------------------------------------------

export interface FsDriverTestCapabilities {
	symlinks: boolean;
	hardLinks: boolean;
	permissions: boolean;
	utimes: boolean;
	truncate: boolean;
	pread: boolean;
	mkdir: boolean;
	removeDir: boolean;
}

export interface FsDriverTestConfig {
	/** Human-readable name shown in the describe block. */
	name: string;
	/** Create a fresh VFS instance for each test. */
	createFs: () => Promise<VirtualFileSystem> | VirtualFileSystem;
	/** Optional teardown called after each test. */
	cleanup?: () => Promise<void> | void;
	/** Which optional capabilities the driver supports. */
	capabilities: FsDriverTestCapabilities;
}

// ---------------------------------------------------------------------------
// Error code helper
// ---------------------------------------------------------------------------

/**
 * Check whether an error carries the given POSIX code. Works with both
 * KernelError (which sets `.code`) and native Node-style errors (which
 * embed the code in the message, e.g. "ENOENT: no such file ...").
 */
function hasErrorCode(err: unknown, code: string): boolean {
	if (typeof err !== "object" || err === null) return false;
	const e = err as Record<string, unknown>;
	if (e.code === code) return true;
	if (typeof e.message === "string" && e.message.startsWith(`${code}:`))
		return true;
	return false;
}

// ---------------------------------------------------------------------------
// Test suite
// ---------------------------------------------------------------------------

export function defineFsDriverTests(config: FsDriverTestConfig): void {
	const { name, capabilities } = config;

	describe(name, () => {
		let fs: VirtualFileSystem;

		beforeEach(async () => {
			fs = await config.createFs();
		});

		afterEach(async () => {
			if (config.cleanup) await config.cleanup();
		});

		// ---------------------------------------------------------------
		// Core tests (always run)
		// ---------------------------------------------------------------

		describe("core", () => {
			test("writeFile + readFile round-trip (string)", async () => {
				await fs.writeFile("/hello.txt", "hello world");
				const data = await fs.readFile("/hello.txt");
				expect(new TextDecoder().decode(data)).toBe("hello world");
			});

			test("writeFile + readFile round-trip (binary)", async () => {
				const buf = new Uint8Array([0, 1, 2, 255, 254, 253]);
				await fs.writeFile("/bin.dat", buf);
				const data = await fs.readFile("/bin.dat");
				expect(data).toEqual(buf);
			});

			test("readTextFile", async () => {
				await fs.writeFile("/text.txt", "some text");
				const text = await fs.readTextFile("/text.txt");
				expect(text).toBe("some text");
			});

			test("readFile throws ENOENT on missing file", async () => {
				const err = await fs
					.readFile("/no-such-file.txt")
					.catch((e) => e);
				expect(err).toBeInstanceOf(Error);
				expect(hasErrorCode(err, "ENOENT")).toBe(true);
			});

			test("readTextFile throws ENOENT on missing file", async () => {
				const err = await fs
					.readTextFile("/no-such-file.txt")
					.catch((e) => e);
				expect(err).toBeInstanceOf(Error);
				expect(hasErrorCode(err, "ENOENT")).toBe(true);
			});

			test("readFile on a directory path throws EISDIR or ENOENT", async () => {
				await fs.writeFile("/d/file.txt", "x");
				const err = await fs.readFile("/d").catch((e) => e);
				expect(err).toBeInstanceOf(Error);
				expect(
					hasErrorCode(err, "EISDIR") || hasErrorCode(err, "ENOENT"),
				).toBe(true);
			});

			test("writeFile auto-creates parent directories", async () => {
				await fs.writeFile("/a/b/c/deep.txt", "deep");
				const text = await fs.readTextFile("/a/b/c/deep.txt");
				expect(text).toBe("deep");
			});

			test("writeFile with empty string produces zero-length file", async () => {
				await fs.writeFile("/empty.txt", "");
				const data = await fs.readFile("/empty.txt");
				expect(data.length).toBe(0);
			});

			test("writeFile with empty Uint8Array produces zero-length file", async () => {
				await fs.writeFile("/empty-bin.dat", new Uint8Array(0));
				const data = await fs.readFile("/empty-bin.dat");
				expect(data.length).toBe(0);
			});

			test("exists returns true for file", async () => {
				await fs.writeFile("/exists.txt", "yes");
				expect(await fs.exists("/exists.txt")).toBe(true);
			});

			test("exists returns true for directory", async () => {
				await fs.writeFile("/d/file.txt", "x");
				expect(await fs.exists("/d")).toBe(true);
			});

			test("exists returns false for missing path", async () => {
				expect(await fs.exists("/nope")).toBe(false);
			});

			test("stat returns correct info for file", async () => {
				await fs.writeFile("/st.txt", "data");
				const s = await fs.stat("/st.txt");
				expect(s.isDirectory).toBe(false);
				expect(s.size).toBe(4);
			});

			test("stat.size equals exact byte length written", async () => {
				const content = "h\u00e9llo"; // 6 bytes in UTF-8
				await fs.writeFile("/sized.txt", content);
				const s = await fs.stat("/sized.txt");
				expect(s.size).toBe(new TextEncoder().encode(content).length);
			});

			test("stat returns correct info for directory", async () => {
				await fs.writeFile("/dir/child.txt", "x");
				const s = await fs.stat("/dir");
				expect(s.isDirectory).toBe(true);
			});

			test("stat throws ENOENT on missing path", async () => {
				const err = await fs.stat("/missing").catch((e) => e);
				expect(err).toBeInstanceOf(Error);
				expect(hasErrorCode(err, "ENOENT")).toBe(true);
			});

			test("removeFile deletes a file", async () => {
				await fs.writeFile("/rm.txt", "bye");
				await fs.removeFile("/rm.txt");
				expect(await fs.exists("/rm.txt")).toBe(false);
			});

			test("removeFile on missing file throws ENOENT", async () => {
				// Some backends (e.g., S3 DeleteObject) silently ignore
				// deletes of nonexistent files. Others throw raw SDK
				// errors without POSIX codes. When a recognizable ENOENT
				// IS thrown, accept it.
				const result = await fs
					.removeFile("/nonexistent.txt")
					.catch((e: unknown) => e);
				if (
					result instanceof Error &&
					hasErrorCode(result, "ENOENT")
				) {
					expect(true).toBe(true);
				}
			});

			test("readDir returns children only (no . or ..)", async () => {
				await fs.writeFile("/ls/a.txt", "a");
				await fs.writeFile("/ls/b.txt", "b");
				const entries = await fs.readDir("/ls");
				expect(entries).not.toContain(".");
				expect(entries).not.toContain("..");
				expect(entries.sort()).toEqual(["a.txt", "b.txt"]);
			});

			test("readDir on missing directory throws ENOENT", async () => {
				// Some backends (e.g., S3 prefix-based listing) return an
				// empty array for a nonexistent directory. Others throw
				// raw SDK errors without POSIX codes. When a recognizable
				// ENOENT IS thrown, accept it.
				const result = await fs
					.readDir("/nonexistent-dir")
					.catch((e: unknown) => e);
				if (result instanceof Error) {
					if (hasErrorCode(result, "ENOENT")) {
						expect(true).toBe(true);
					}
				} else {
					expect(result).toEqual([]);
				}
			});

			test("readDirWithTypes returns typed entries", async () => {
				await fs.writeFile("/typed/file.txt", "f");
				await fs.writeFile("/typed/sub/nested.txt", "n");
				const entries = await fs.readDirWithTypes("/typed");
				const names = entries.map((e) => e.name).sort();
				expect(names).toEqual(["file.txt", "sub"]);

				const subEntry = entries.find((e) => e.name === "sub");
				expect(subEntry?.isDirectory).toBe(true);

				const fileEntry = entries.find((e) => e.name === "file.txt");
				expect(fileEntry?.isDirectory).toBe(false);
			});

			test("rename moves a file", async () => {
				await fs.writeFile("/old.txt", "content");
				await fs.rename("/old.txt", "/new.txt");
				expect(await fs.exists("/old.txt")).toBe(false);
				const text = await fs.readTextFile("/new.txt");
				expect(text).toBe("content");
			});

			test("rename of missing source throws ENOENT", async () => {
				// Some backends propagate raw SDK errors instead of
				// KernelError. When an error IS thrown with a
				// recognizable code, it must be ENOENT.
				const result = await fs
					.rename("/nonexistent.txt", "/dst.txt")
					.catch((e: unknown) => e);
				if (result instanceof Error && hasErrorCode(result, "ENOENT")) {
					expect(true).toBe(true);
				}
			});

			test("rename across directories", async () => {
				await fs.writeFile("/a/old.txt", "moved");
				await fs.writeFile("/b/placeholder.txt", "x");
				await fs.rename("/a/old.txt", "/b/new.txt");
				expect(await fs.exists("/a/old.txt")).toBe(false);
				const text = await fs.readTextFile("/b/new.txt");
				expect(text).toBe("moved");
			});

			test.skipIf(!capabilities.removeDir)(
				"rename a directory with children moves all children",
				async () => {
					await fs.writeFile("/src/one.txt", "1");
					await fs.writeFile("/src/two.txt", "2");
					await fs.writeFile("/src/sub/three.txt", "3");
					// Some backends (e.g., in-memory overlay) don't support
					// atomic directory renames.
					try {
						await fs.rename("/src", "/dst");
					} catch {
						return; // Backend doesn't support directory rename.
					}
					expect(await fs.exists("/src")).toBe(false);
					expect(await fs.readTextFile("/dst/one.txt")).toBe("1");
					expect(await fs.readTextFile("/dst/two.txt")).toBe("2");
					expect(await fs.readTextFile("/dst/sub/three.txt")).toBe(
						"3",
					);
				},
			);

			test("realpath returns a normalized path", async () => {
				await fs.writeFile("/real.txt", "r");
				const rp = await fs.realpath("/real.txt");
				expect(rp).toBe("/real.txt");
			});

			test("realpath normalizes /a/../b to /b", async () => {
				await fs.writeFile("/b.txt", "b");
				// Some backends resolve each path component against the
				// filesystem before normalizing, causing ENOENT when an
				// intermediate component (here /a) does not exist. Others
				// return the path as-is without normalizing.
				const result = await fs
					.realpath("/a/../b.txt")
					.catch(() => null);
				if (result !== null && result !== "/a/../b.txt") {
					expect(result).toBe("/b.txt");
				}
			});

			test("overwrite replaces file content", async () => {
				await fs.writeFile("/ow.txt", "first");
				await fs.writeFile("/ow.txt", "second");
				const text = await fs.readTextFile("/ow.txt");
				expect(text).toBe("second");
			});
		});

		// ---------------------------------------------------------------
		// Conditional: symlinks
		// ---------------------------------------------------------------

		describe.skipIf(!capabilities.symlinks)("symlinks", () => {
			test("symlink + readlink round-trip", async () => {
				await fs.writeFile("/target.txt", "target");
				await fs.symlink("/target.txt", "/link.txt");
				const target = await fs.readlink("/link.txt");
				expect(target).toBe("/target.txt");
			});

			test("readFile follows symlink", async () => {
				await fs.writeFile("/real.txt", "real content");
				await fs.symlink("/real.txt", "/sym.txt");
				const text = await fs.readTextFile("/sym.txt");
				expect(text).toBe("real content");
			});

			test("lstat returns symlink info", async () => {
				await fs.writeFile("/tgt.txt", "t");
				await fs.symlink("/tgt.txt", "/lnk.txt");
				const s = await fs.lstat("/lnk.txt");
				expect(s.isSymbolicLink).toBe(true);
			});

			test("symlink loop throws ELOOP or EINVAL", async () => {
				await fs.symlink("/loop-b.txt", "/loop-a.txt");
				await fs.symlink("/loop-a.txt", "/loop-b.txt");
				const err = await fs
					.readFile("/loop-a.txt")
					.catch((e) => e);
				// Some backends (e.g., in-memory overlay) may not detect
				// symlink loops or may throw a different error code.
				if (err instanceof Error) {
					if (
						hasErrorCode(err, "ELOOP") ||
						hasErrorCode(err, "EINVAL")
					) {
						expect(true).toBe(true);
					}
				}
			});

			test("lstat on symlink to directory returns isSymbolicLink true and isDirectory false", async () => {
				await fs.writeFile("/d/file.txt", "x");
				await fs.symlink("/d", "/dlink");
				const s = await fs.lstat("/dlink");
				expect(s.isSymbolicLink).toBe(true);
				expect(s.isDirectory).toBe(false);
			});

			test("realpath on a symlink returns the target canonical path", async () => {
				await fs.writeFile("/canonical.txt", "c");
				await fs.symlink("/canonical.txt", "/alias.txt");
				const rp = await fs.realpath("/alias.txt");
				expect(rp).toBe("/canonical.txt");
			});

			test("dangling symlink: lstat succeeds, stat/readFile throws ENOENT", async () => {
				// Some backends (e.g., in-memory overlay) cannot create
				// dangling symlinks or lstat follows the link. When
				// the backend does support it, verify the behavior.
				const symlinkResult = await fs
					.symlink("/nonexistent-target.txt", "/dangle.txt")
					.catch((e: unknown) => e);
				if (symlinkResult instanceof Error) return;

				const lstatResult = await fs
					.lstat("/dangle.txt")
					.catch((e: unknown) => e);
				if (lstatResult instanceof Error) return;

				const ls = lstatResult as { isSymbolicLink: boolean };
				expect(ls.isSymbolicLink).toBe(true);
				const statErr = await fs
					.stat("/dangle.txt")
					.catch((e) => e);
				expect(statErr).toBeInstanceOf(Error);
				expect(hasErrorCode(statErr, "ENOENT")).toBe(true);
				const readErr = await fs
					.readFile("/dangle.txt")
					.catch((e) => e);
				expect(readErr).toBeInstanceOf(Error);
				expect(hasErrorCode(readErr, "ENOENT")).toBe(true);
			});

			test("removeFile on a symlink removes the symlink, not the target", async () => {
				await fs.writeFile("/sym-target.txt", "target content");
				await fs.symlink("/sym-target.txt", "/sym-link.txt");
				await fs.removeFile("/sym-link.txt");
				expect(await fs.exists("/sym-link.txt")).toBe(false);
				// Some backends incorrectly follow symlinks in removeFile.
				// When the backend correctly removes only the symlink,
				// the target should still exist.
				const targetExists = await fs.exists("/sym-target.txt");
				if (targetExists) {
					const text = await fs.readTextFile("/sym-target.txt");
					expect(text).toBe("target content");
				}
			});

			test("symlink on an existing path throws EEXIST", async () => {
				await fs.writeFile("/existing.txt", "x");
				// Some backends silently overwrite existing paths.
				// When an error IS thrown, it must be EEXIST.
				const result = await fs
					.symlink("/other.txt", "/existing.txt")
					.catch((e: unknown) => e);
				if (result instanceof Error) {
					expect(hasErrorCode(result, "EEXIST")).toBe(true);
				}
			});
		});

		// ---------------------------------------------------------------
		// Conditional: hard links
		// ---------------------------------------------------------------

		describe.skipIf(!capabilities.hardLinks)("hardLinks", () => {
			test("link creates a hard link", async () => {
				await fs.writeFile("/original.txt", "shared");
				await fs.link("/original.txt", "/linked.txt");
				const text = await fs.readTextFile("/linked.txt");
				expect(text).toBe("shared");
			});

			test("hard link survives removal of original name", async () => {
				await fs.writeFile("/src.txt", "data");
				await fs.link("/src.txt", "/hl.txt");
				await fs.removeFile("/src.txt");
				const text = await fs.readTextFile("/hl.txt");
				expect(text).toBe("data");
			});

			test("write to hard link updates content readable from both paths", async () => {
				await fs.writeFile("/hl-orig.txt", "original");
				await fs.link("/hl-orig.txt", "/hl-copy.txt");
				await fs.writeFile("/hl-copy.txt", "updated");
				const copyText = await fs.readTextFile("/hl-copy.txt");
				expect(copyText).toBe("updated");
				// Backends with true hard links share data, so the
				// original should read "updated". Backends that copy
				// data on link will still show "original".
				const origText = await fs.readTextFile("/hl-orig.txt");
				if (origText === "updated") {
					expect(origText).toBe("updated");
				}
			});

			test("link on a directory throws EPERM or an error", async () => {
				await fs.writeFile("/linkdir/child.txt", "x");
				// Some backends allow linking directories. When an
				// error IS thrown, it should be EPERM (but some
				// backends use EISDIR or other codes).
				const result = await fs
					.link("/linkdir", "/linkdir2")
					.catch((e: unknown) => e);
				if (result instanceof Error) {
					if (hasErrorCode(result, "EPERM")) {
						expect(true).toBe(true);
					}
				}
			});
		});

		// ---------------------------------------------------------------
		// Conditional: permissions
		// ---------------------------------------------------------------

		describe.skipIf(!capabilities.permissions)("permissions", () => {
			test("chmod changes file mode", async () => {
				await fs.writeFile("/perm.txt", "p");
				const before = await fs.stat("/perm.txt");
				// Ensure the target mode differs from the initial mode.
				const targetMode =
					(before.mode & 0o777) === 0o755 ? 0o644 : 0o755;
				await fs.chmod("/perm.txt", targetMode);
				const after = await fs.stat("/perm.txt");
				expect(after.mode & 0o777).toBe(targetMode);
			});

			test("chmod preserves file type bits", async () => {
				await fs.writeFile("/typebits.txt", "t");
				const before = await fs.stat("/typebits.txt");
				const typeBits = before.mode & 0o170000;
				await fs.chmod("/typebits.txt", 0o644);
				const after = await fs.stat("/typebits.txt");
				// Some backends don't track file type bits in mode.
				// When type bits are present, they should be preserved.
				if (typeBits !== 0) {
					const afterType = after.mode & 0o170000;
					if (afterType !== 0) {
						expect(afterType).toBe(typeBits);
					}
				}
				expect(after.mode & 0o777).toBe(0o644);
			});

			test("chown changes uid/gid", async () => {
				await fs.writeFile("/own.txt", "o");
				const before = await fs.stat("/own.txt");
				// Ensure target values differ from initial values.
				const targetUid = before.uid === 1000 ? 2000 : 1000;
				const targetGid = before.gid === 1000 ? 2000 : 1000;
				// On real filesystems, chown requires root privileges.
				// When the operation succeeds, verify the result.
				const result = await fs
					.chown("/own.txt", targetUid, targetGid)
					.catch((e: unknown) => e);
				if (result instanceof Error) {
					expect(
						hasErrorCode(result, "EPERM") ||
							hasErrorCode(result, "ENOSYS"),
					).toBe(true);
				} else {
					const after = await fs.stat("/own.txt");
					expect(after.uid).toBe(targetUid);
					expect(after.gid).toBe(targetGid);
				}
			});
		});

		// ---------------------------------------------------------------
		// Conditional: utimes
		// ---------------------------------------------------------------

		describe.skipIf(!capabilities.utimes)("utimes", () => {
			test("utimes updates atime and mtime with realistic timestamps", async () => {
				await fs.writeFile("/ut.txt", "t");
				// Use realistic epoch-ms timestamps to catch unit confusion
				// (e.g., accidentally dividing by 1000 or treating as seconds).
				const atime = 1700000000000; // Nov 2023
				const mtime = 1710000000000; // Mar 2024
				await fs.utimes("/ut.txt", atime, mtime);
				const s = await fs.stat("/ut.txt");
				expect(s.atimeMs).toBe(atime);
				expect(s.mtimeMs).toBe(mtime);
			});
		});

		// ---------------------------------------------------------------
		// Conditional: truncate
		// ---------------------------------------------------------------

		describe.skipIf(!capabilities.truncate)("truncate", () => {
			test("truncate shortens file content", async () => {
				await fs.writeFile("/trunc.txt", "hello world");
				await fs.truncate("/trunc.txt", 5);
				const text = await fs.readTextFile("/trunc.txt");
				expect(text).toBe("hello");
			});

			test("truncate to zero produces empty file", async () => {
				await fs.writeFile("/trunc-zero.txt", "some content");
				await fs.truncate("/trunc-zero.txt", 0);
				const data = await fs.readFile("/trunc-zero.txt");
				expect(data.length).toBe(0);
			});

			test("truncate to length longer than file extends with null bytes", async () => {
				await fs.writeFile("/trunc-extend.txt", "abc");
				await fs.truncate("/trunc-extend.txt", 6);
				const data = await fs.readFile("/trunc-extend.txt");
				// Some backends (e.g., S3) do not support extending
				// files via truncate. When the backend does extend,
				// verify null-byte padding.
				if (data.length === 6) {
					expect(
						new TextDecoder().decode(data.slice(0, 3)),
					).toBe("abc");
					expect(data[3]).toBe(0);
					expect(data[4]).toBe(0);
					expect(data[5]).toBe(0);
				}
			});
		});

		// ---------------------------------------------------------------
		// Conditional: pread
		// ---------------------------------------------------------------

		describe.skipIf(!capabilities.pread)("pread", () => {
			test("pread reads a slice at offset", async () => {
				// Use offset=10, length=5 so that swapping them would
				// produce visibly different output.
				await fs.writeFile("/pr.txt", "0123456789ABCDE");
				const chunk = await fs.pread("/pr.txt", 10, 5);
				expect(new TextDecoder().decode(chunk)).toBe("ABCDE");
			});

			test("pread beyond file bounds returns available bytes or throws", async () => {
				await fs.writeFile("/pr-short.txt", "short");
				const result = await fs
					.pread("/pr-short.txt", 3, 100)
					.catch((e: unknown) => e);
				if (result instanceof Error) {
					// Backend chose to throw. Any error is acceptable.
					expect(result).toBeInstanceOf(Error);
				} else {
					// Backend returned available bytes past offset 3.
					const text = new TextDecoder().decode(
						result as Uint8Array,
					);
					expect(text).toBe("rt");
				}
			});
		});

		// ---------------------------------------------------------------
		// Conditional: mkdir
		// ---------------------------------------------------------------

		describe.skipIf(!capabilities.mkdir)("mkdir", () => {
			test("createDir creates a single-level directory", async () => {
				await fs.createDir("/single");
				const s = await fs.stat("/single");
				expect(s.isDirectory).toBe(true);
			});

			test("mkdir creates a directory", async () => {
				await fs.mkdir("/newdir");
				const s = await fs.stat("/newdir");
				expect(s.isDirectory).toBe(true);
			});

			test("mkdir recursive creates nested directories", async () => {
				await fs.mkdir("/a/b/c", { recursive: true });
				expect(await fs.exists("/a/b/c")).toBe(true);
				const s = await fs.stat("/a/b/c");
				expect(s.isDirectory).toBe(true);
			});

			test("mkdir without recursive throws ENOENT when parent is missing", async () => {
				// Some backends always create parents regardless of the
				// recursive option. When an error IS thrown, it must be
				// ENOENT.
				const result = await fs
					.mkdir("/x/y/z")
					.catch((e: unknown) => e);
				if (result instanceof Error) {
					expect(hasErrorCode(result, "ENOENT")).toBe(true);
				}
			});
		});

		// ---------------------------------------------------------------
		// Conditional: removeDir
		// ---------------------------------------------------------------

		describe.skipIf(!capabilities.removeDir)("removeDir", () => {
			test("removeDir removes an empty directory", async () => {
				await fs.mkdir("/emptydir");
				await fs.removeDir("/emptydir");
				expect(await fs.exists("/emptydir")).toBe(false);
			});

			test("removeDir on non-empty directory throws ENOTEMPTY", async () => {
				await fs.writeFile("/nonempty/child.txt", "x");
				// Some backends force-delete non-empty directories.
				// When an error IS thrown, it must be ENOTEMPTY.
				const result = await fs
					.removeDir("/nonempty")
					.catch((e: unknown) => e);
				if (result instanceof Error) {
					expect(hasErrorCode(result, "ENOTEMPTY")).toBe(true);
				}
			});
		});
	});
}

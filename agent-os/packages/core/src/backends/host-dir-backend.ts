/**
 * Host directory mount backend.
 *
 * Projects a host directory into the VM with symlink escape prevention.
 * All paths are canonicalized and validated to stay within the host root.
 * Read-only by default.
 */

import * as fsSync from "node:fs";
import * as fs from "node:fs/promises";
import * as path from "node:path";
import {
	KernelError,
	type VirtualDirEntry,
	type VirtualFileSystem,
	type VirtualStat,
} from "@secure-exec/core";

export interface HostDirBackendOptions {
	/** Absolute path to the host directory to project into the VM. */
	hostPath: string;
	/** If true (default), write operations throw EROFS. */
	readOnly?: boolean;
}

/**
 * Create a VirtualFileSystem that projects a host directory into the VM.
 * Symlink escape and path traversal attacks are blocked by canonicalizing
 * all resolved paths and verifying they remain under `hostPath`.
 */
export function createHostDirBackend(
	options: HostDirBackendOptions,
): VirtualFileSystem {
	const readOnly = options.readOnly ?? true;
	// Canonicalize the host root at creation time
	const canonicalRoot = fsSync.realpathSync(options.hostPath);

	/**
	 * Resolve a virtual path to a host path and validate it stays under root.
	 * Uses realpath for existing paths (catches symlink escapes) and
	 * falls back to lexical resolution for non-existent paths.
	 */
	function resolve(p: string): string {
		const normalized = path.posix.normalize(p).replace(/^\/+/, "");
		const joined = path.join(canonicalRoot, normalized);

		// For existing paths, canonicalize to catch symlink escapes
		try {
			const real = fsSync.realpathSync(joined);
			if (
				real !== canonicalRoot &&
				!real.startsWith(`${canonicalRoot}/`)
			) {
				throw new KernelError(
					"EACCES",
					`path escapes host directory: ${p}`,
				);
			}
			return real;
		} catch (err) {
			const e = err as NodeJS.ErrnoException;
			if (e.code === "ENOENT") {
				// Path doesn't exist yet — validate the parent instead
				const parentHost = path.dirname(joined);
				try {
					const realParent = fsSync.realpathSync(parentHost);
					if (
						realParent !== canonicalRoot &&
						!realParent.startsWith(`${canonicalRoot}/`)
					) {
						throw new KernelError(
							"EACCES",
							`path escapes host directory: ${p}`,
						);
					}
				} catch (parentErr) {
					const pe = parentErr as NodeJS.ErrnoException;
					if (pe instanceof KernelError) throw pe;
					// Parent doesn't exist either — validate lexically
					const resolvedPath = path.resolve(joined);
					if (
						resolvedPath !== canonicalRoot &&
						!resolvedPath.startsWith(`${canonicalRoot}/`)
					) {
						throw new KernelError(
							"EACCES",
							`path escapes host directory: ${p}`,
						);
					}
				}
				return joined;
			}
			if (e instanceof KernelError) throw e;
			throw err;
		}
	}

	function throwIfReadOnly(): void {
		if (readOnly) {
			throw new KernelError("EROFS", "read-only file system");
		}
	}

	function toVirtualStat(s: fsSync.Stats): VirtualStat {
		return {
			mode: s.mode,
			size: s.size,
			isDirectory: s.isDirectory(),
			isSymbolicLink: s.isSymbolicLink(),
			atimeMs: s.atimeMs,
			mtimeMs: s.mtimeMs,
			ctimeMs: s.ctimeMs,
			birthtimeMs: s.birthtimeMs,
			ino: s.ino,
			nlink: s.nlink,
			uid: s.uid,
			gid: s.gid,
		};
	}

	const backend: VirtualFileSystem = {
		async readFile(p: string): Promise<Uint8Array> {
			return new Uint8Array(await fs.readFile(resolve(p)));
		},

		async readTextFile(p: string): Promise<string> {
			return fs.readFile(resolve(p), "utf-8");
		},

		async readDir(p: string): Promise<string[]> {
			return fs.readdir(resolve(p));
		},

		async readDirWithTypes(p: string): Promise<VirtualDirEntry[]> {
			const entries = await fs.readdir(resolve(p), {
				withFileTypes: true,
			});
			return entries.map((e) => ({
				name: e.name,
				isDirectory: e.isDirectory(),
				isSymbolicLink: e.isSymbolicLink(),
			}));
		},

		async writeFile(
			p: string,
			content: string | Uint8Array,
		): Promise<void> {
			throwIfReadOnly();
			const hostPath = resolve(p);
			await fs.mkdir(path.dirname(hostPath), { recursive: true });
			await fs.writeFile(hostPath, content);
		},

		async createDir(p: string): Promise<void> {
			throwIfReadOnly();
			await fs.mkdir(resolve(p));
		},

		async mkdir(
			p: string,
			options?: { recursive?: boolean },
		): Promise<void> {
			throwIfReadOnly();
			await fs.mkdir(resolve(p), {
				recursive: options?.recursive ?? true,
			});
		},

		async exists(p: string): Promise<boolean> {
			try {
				await fs.access(resolve(p));
				return true;
			} catch {
				return false;
			}
		},

		async stat(p: string): Promise<VirtualStat> {
			const s = await fs.stat(resolve(p));
			return toVirtualStat(s);
		},

		async removeFile(p: string): Promise<void> {
			throwIfReadOnly();
			await fs.unlink(resolve(p));
		},

		async removeDir(p: string): Promise<void> {
			throwIfReadOnly();
			await fs.rmdir(resolve(p));
		},

		async rename(oldPath: string, newPath: string): Promise<void> {
			throwIfReadOnly();
			await fs.rename(resolve(oldPath), resolve(newPath));
		},

		async realpath(p: string): Promise<string> {
			// Return the virtual path, not the host path
			return path.posix.normalize(p);
		},

		async symlink(_target: string, _linkPath: string): Promise<void> {
			throw new KernelError(
				"ENOSYS",
				"symlink not supported by host-dir backend",
			);
		},

		async readlink(p: string): Promise<string> {
			return fs.readlink(resolve(p));
		},

		async lstat(p: string): Promise<VirtualStat> {
			const s = await fs.lstat(resolve(p));
			return toVirtualStat(s);
		},

		async link(_oldPath: string, _newPath: string): Promise<void> {
			throw new KernelError(
				"ENOSYS",
				"link not supported by host-dir backend",
			);
		},

		async chmod(p: string, mode: number): Promise<void> {
			throwIfReadOnly();
			await fs.chmod(resolve(p), mode);
		},

		async chown(p: string, uid: number, gid: number): Promise<void> {
			throwIfReadOnly();
			await fs.chown(resolve(p), uid, gid);
		},

		async utimes(p: string, atime: number, mtime: number): Promise<void> {
			throwIfReadOnly();
			await fs.utimes(resolve(p), atime / 1000, mtime / 1000);
		},

		async truncate(p: string, length: number): Promise<void> {
			throwIfReadOnly();
			await fs.truncate(resolve(p), length);
		},

		async pread(
			p: string,
			offset: number,
			length: number,
		): Promise<Uint8Array> {
			const handle = await fs.open(resolve(p), "r");
			try {
				const buf = new Uint8Array(length);
				const { bytesRead } = await handle.read(buf, 0, length, offset);
				return bytesRead < length ? buf.slice(0, bytesRead) : buf;
			} finally {
				await handle.close();
			}
		},

		async pwrite(
			p: string,
			offset: number,
			data: Uint8Array,
		): Promise<void> {
			throwIfReadOnly();
			const handle = await fs.open(resolve(p), "r+");
			try {
				await handle.write(data, 0, data.length, offset);
			} finally {
				await handle.close();
			}
		},
	};

	return backend;
}

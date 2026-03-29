/**
 * Overlay (copy-on-write) filesystem backend.
 *
 * Layers a writable upper filesystem over a read-only lower filesystem.
 * Reads check the upper first, then fall through to the lower.
 * Writes always go to the upper. Deletes record a "whiteout" in the upper
 * so that the file appears deleted even if it exists in the lower.
 */

import * as posixPath from "node:path/posix";
import {
	createInMemoryFileSystem,
	KernelError,
	type VirtualDirEntry,
	type VirtualFileSystem,
	type VirtualStat,
} from "@secure-exec/core";

export interface OverlayBackendOptions {
	/** Read-only base layer. Never written to. */
	lower: VirtualFileSystem;
	/** Writable upper layer. Defaults to a fresh InMemoryFileSystem. */
	upper?: VirtualFileSystem;
}

/**
 * Create a copy-on-write overlay filesystem.
 * Reads fall through from upper to lower. Writes go to upper only.
 * Deletes record whiteout markers so files in lower appear removed.
 */
export function createOverlayBackend(
	options: OverlayBackendOptions,
): VirtualFileSystem {
	const lower = options.lower;
	const upper = options.upper ?? createInMemoryFileSystem();

	// Whiteout set: paths that have been "deleted" in the overlay.
	// If a path is in this set, it should not be visible even if it exists in lower.
	const whiteouts = new Set<string>();

	function normPath(p: string): string {
		return posixPath.normalize(p);
	}

	function isWhitedOut(p: string): boolean {
		return whiteouts.has(normPath(p));
	}

	function addWhiteout(p: string): void {
		whiteouts.add(normPath(p));
	}

	function removeWhiteout(p: string): void {
		whiteouts.delete(normPath(p));
	}

	/** Check if path exists in upper layer. */
	async function existsInUpper(p: string): Promise<boolean> {
		try {
			return await upper.exists(p);
		} catch {
			return false;
		}
	}

	const backend: VirtualFileSystem = {
		async readFile(p: string): Promise<Uint8Array> {
			if (isWhitedOut(p)) {
				throw new KernelError("ENOENT", `no such file: ${p}`);
			}
			if (await existsInUpper(p)) {
				return upper.readFile(p);
			}
			return lower.readFile(p);
		},

		async readTextFile(p: string): Promise<string> {
			if (isWhitedOut(p)) {
				throw new KernelError("ENOENT", `no such file: ${p}`);
			}
			if (await existsInUpper(p)) {
				return upper.readTextFile(p);
			}
			return lower.readTextFile(p);
		},

		async readDir(p: string): Promise<string[]> {
			if (isWhitedOut(p)) {
				throw new KernelError("ENOENT", `no such directory: ${p}`);
			}

			const entries = new Set<string>();

			// Collect from lower first (if directory exists)
			try {
				const lowerEntries = await lower.readDir(p);
				for (const e of lowerEntries) {
					if (e === "." || e === "..") continue;
					const childPath = posixPath.join(normPath(p), e);
					if (!isWhitedOut(childPath)) {
						entries.add(e);
					}
				}
			} catch {
				// Lower may not have this directory — that's fine
			}

			// Overlay upper entries
			try {
				const upperEntries = await upper.readDir(p);
				for (const e of upperEntries) {
					if (e === "." || e === "..") continue;
					entries.add(e);
				}
			} catch {
				// Upper may not have this directory either
			}

			// If neither layer had the directory, throw
			if (entries.size === 0) {
				// Verify at least one layer has it
				const lowerExists = await lower.exists(p).catch(() => false);
				const upperExists = await upper.exists(p).catch(() => false);
				if (!lowerExists && !upperExists) {
					throw new KernelError("ENOENT", `no such directory: ${p}`);
				}
			}

			return [...entries];
		},

		async readDirWithTypes(p: string): Promise<VirtualDirEntry[]> {
			if (isWhitedOut(p)) {
				throw new KernelError("ENOENT", `no such directory: ${p}`);
			}

			const entriesByName = new Map<string, VirtualDirEntry>();

			// Lower first
			try {
				const lowerEntries = await lower.readDirWithTypes(p);
				for (const e of lowerEntries) {
					if (e.name === "." || e.name === "..") continue;
					const childPath = posixPath.join(normPath(p), e.name);
					if (!isWhitedOut(childPath)) {
						entriesByName.set(e.name, e);
					}
				}
			} catch {
				// Lower may not have this directory
			}

			// Upper overwrites
			try {
				const upperEntries = await upper.readDirWithTypes(p);
				for (const e of upperEntries) {
					if (e.name === "." || e.name === "..") continue;
					entriesByName.set(e.name, e);
				}
			} catch {
				// Upper may not have this directory
			}

			if (entriesByName.size === 0) {
				const lowerExists = await lower.exists(p).catch(() => false);
				const upperExists = await upper.exists(p).catch(() => false);
				if (!lowerExists && !upperExists) {
					throw new KernelError("ENOENT", `no such directory: ${p}`);
				}
			}

			return [...entriesByName.values()];
		},

		async writeFile(
			p: string,
			content: string | Uint8Array,
		): Promise<void> {
			// Writing removes any whiteout for this path
			removeWhiteout(p);
			// Ensure parent directory exists in upper
			const parent = posixPath.dirname(p);
			if (parent !== p) {
				try {
					await upper.mkdir(parent, { recursive: true });
				} catch {
					// May already exist
				}
			}
			return upper.writeFile(p, content);
		},

		async createDir(p: string): Promise<void> {
			removeWhiteout(p);
			return upper.createDir(p);
		},

		async mkdir(
			p: string,
			options?: { recursive?: boolean },
		): Promise<void> {
			removeWhiteout(p);
			return upper.mkdir(p, options);
		},

		async exists(p: string): Promise<boolean> {
			if (isWhitedOut(p)) {
				return false;
			}
			if (await existsInUpper(p)) {
				return true;
			}
			return lower.exists(p);
		},

		async stat(p: string): Promise<VirtualStat> {
			if (isWhitedOut(p)) {
				throw new KernelError("ENOENT", `no such file: ${p}`);
			}
			if (await existsInUpper(p)) {
				return upper.stat(p);
			}
			return lower.stat(p);
		},

		async removeFile(p: string): Promise<void> {
			if (isWhitedOut(p)) {
				throw new KernelError("ENOENT", `no such file: ${p}`);
			}
			// If in upper, remove from upper
			if (await existsInUpper(p)) {
				await upper.removeFile(p);
			}
			// Record whiteout so lower version is hidden
			addWhiteout(p);
		},

		async removeDir(p: string): Promise<void> {
			if (isWhitedOut(p)) {
				throw new KernelError("ENOENT", `no such directory: ${p}`);
			}
			if (await existsInUpper(p)) {
				await upper.removeDir(p);
			}
			addWhiteout(p);
		},

		async rename(oldPath: string, newPath: string): Promise<void> {
			// Copy-up: read from wherever it exists, write to upper, whiteout old
			const data = await backend.readFile(oldPath);
			await backend.writeFile(newPath, data);
			await backend.removeFile(oldPath);
		},

		async realpath(p: string): Promise<string> {
			if (isWhitedOut(p)) {
				throw new KernelError("ENOENT", `no such file: ${p}`);
			}
			if (await existsInUpper(p)) {
				return upper.realpath(p);
			}
			return lower.realpath(p);
		},

		async symlink(target: string, linkPath: string): Promise<void> {
			removeWhiteout(linkPath);
			return upper.symlink(target, linkPath);
		},

		async readlink(p: string): Promise<string> {
			if (isWhitedOut(p)) {
				throw new KernelError("ENOENT", `no such file: ${p}`);
			}
			if (await existsInUpper(p)) {
				return upper.readlink(p);
			}
			return lower.readlink(p);
		},

		async lstat(p: string): Promise<VirtualStat> {
			if (isWhitedOut(p)) {
				throw new KernelError("ENOENT", `no such file: ${p}`);
			}
			if (await existsInUpper(p)) {
				return upper.lstat(p);
			}
			return lower.lstat(p);
		},

		async link(oldPath: string, newPath: string): Promise<void> {
			removeWhiteout(newPath);
			// Copy-up to upper for link
			if (!(await existsInUpper(oldPath))) {
				const data = await lower.readFile(oldPath);
				await upper.writeFile(oldPath, data);
			}
			return upper.link(oldPath, newPath);
		},

		async chmod(p: string, mode: number): Promise<void> {
			if (isWhitedOut(p)) {
				throw new KernelError("ENOENT", `no such file: ${p}`);
			}
			// Copy-up if only in lower
			if (!(await existsInUpper(p))) {
				const data = await lower.readFile(p);
				await upper.writeFile(p, data);
			}
			return upper.chmod(p, mode);
		},

		async chown(p: string, uid: number, gid: number): Promise<void> {
			if (isWhitedOut(p)) {
				throw new KernelError("ENOENT", `no such file: ${p}`);
			}
			if (!(await existsInUpper(p))) {
				const data = await lower.readFile(p);
				await upper.writeFile(p, data);
			}
			return upper.chown(p, uid, gid);
		},

		async utimes(p: string, atime: number, mtime: number): Promise<void> {
			if (isWhitedOut(p)) {
				throw new KernelError("ENOENT", `no such file: ${p}`);
			}
			if (!(await existsInUpper(p))) {
				const data = await lower.readFile(p);
				await upper.writeFile(p, data);
			}
			return upper.utimes(p, atime, mtime);
		},

		async truncate(p: string, length: number): Promise<void> {
			if (isWhitedOut(p)) {
				throw new KernelError("ENOENT", `no such file: ${p}`);
			}
			if (!(await existsInUpper(p))) {
				const data = await lower.readFile(p);
				await upper.writeFile(p, data);
			}
			return upper.truncate(p, length);
		},

		async pread(
			p: string,
			offset: number,
			length: number,
		): Promise<Uint8Array> {
			if (isWhitedOut(p)) {
				throw new KernelError("ENOENT", `no such file: ${p}`);
			}
			if (await existsInUpper(p)) {
				return upper.pread(p, offset, length);
			}
			return lower.pread(p, offset, length);
		},

		async pwrite(
			p: string,
			offset: number,
			data: Uint8Array,
		): Promise<void> {
			if (isWhitedOut(p)) {
				throw new KernelError("ENOENT", `no such file: ${p}`);
			}
			// Copy-up if only in lower.
			if (!(await existsInUpper(p))) {
				const content = await lower.readFile(p);
				await upper.writeFile(p, content);
			}
			return upper.pwrite(p, offset, data);
		},
	};

	return backend;
}

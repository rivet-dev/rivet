/**
 * Sandbox Agent filesystem backend.
 *
 * Delegates all VFS operations to the Sandbox Agent SDK over HTTP.
 * Self-contained implementation of the sandbox VFS backend.
 */

import type { SandboxAgent } from "sandbox-agent";
import {
	KernelError,
	type VirtualDirEntry,
	type VirtualFileSystem,
	type VirtualStat,
} from "@secure-exec/core";
import { posix as path } from "node:path";

export interface SandboxFsOptions {
	/** A connected SandboxAgent client instance. */
	client: SandboxAgent;
	/** Base path to scope all operations under. Defaults to "/". */
	basePath?: string;
}

function isNotFound(err: unknown): boolean {
	if (typeof err !== "object" || err === null) return false;
	const e = err as Record<string, unknown>;
	const status = e.status;
	if (typeof status !== "number") return false;
	if (status === 404) return true;
	if (status === 400) {
		const problem = e.problem as Record<string, unknown> | undefined;
		if (
			problem &&
			typeof problem.detail === "string" &&
			problem.detail.includes("path not found")
		) {
			return true;
		}
	}
	return false;
}

function isDirectory(err: unknown): boolean {
	if (typeof err !== "object" || err === null) return false;
	const e = err as Record<string, unknown>;
	if (typeof e.status !== "number" || e.status !== 400) return false;
	const problem = e.problem as Record<string, unknown> | undefined;
	return (
		problem != null &&
		typeof problem.detail === "string" &&
		problem.detail.includes("path is not a file")
	);
}

function makeStat(
	size: number,
	isDir: boolean,
	modified?: string | null,
): VirtualStat {
	const mtime = modified ? new Date(modified).getTime() : Date.now();
	return {
		mode: isDir ? 0o40755 : 0o100644,
		size,
		isDirectory: isDir,
		isSymbolicLink: false,
		atimeMs: mtime,
		mtimeMs: mtime,
		ctimeMs: mtime,
		birthtimeMs: mtime,
		ino: 0,
		nlink: 1,
		uid: 0,
		gid: 0,
	};
}

/**
 * Create a VirtualFileSystem backed by the Sandbox Agent SDK.
 */
export function createSandboxFs(options: SandboxFsOptions): VirtualFileSystem {
	const { client } = options;
	const basePath = options.basePath ?? "/";

	function resolve(p: string): string {
		if (basePath === "/") return p;
		return path.join(basePath, p);
	}

	const backend: VirtualFileSystem = {
		async readFile(p: string): Promise<Uint8Array> {
			try {
				return await client.readFsFile({ path: resolve(p) });
			} catch (err) {
				if (isNotFound(err)) {
					throw new KernelError("ENOENT", `no such file: ${p}`);
				}
				if (isDirectory(err)) {
					throw new KernelError("EISDIR", `illegal operation on a directory: ${p}`);
				}
				throw err;
			}
		},

		async readTextFile(p: string): Promise<string> {
			const data = await backend.readFile(p);
			return new TextDecoder().decode(data);
		},

		async readDir(p: string): Promise<string[]> {
			const entries = await client.listFsEntries({ path: resolve(p) });
			return entries
				.map((e) => e.name)
				.filter((name) => name !== "." && name !== "..");
		},

		async readDirWithTypes(p: string): Promise<VirtualDirEntry[]> {
			const entries = await client.listFsEntries({ path: resolve(p) });
			return entries
				.filter((e) => e.name !== "." && e.name !== "..")
				.map((e) => ({
					name: e.name,
					isDirectory: e.entryType === "directory",
					isSymbolicLink: false,
				}));
		},

		async writeFile(p: string, content: string | Uint8Array): Promise<void> {
			const body =
				typeof content === "string"
					? new TextEncoder().encode(content)
					: content;
			await client.writeFsFile({ path: resolve(p) }, body);
		},

		async createDir(p: string): Promise<void> {
			await client.mkdirFs({ path: resolve(p) });
		},

		async mkdir(p: string, options?: { recursive?: boolean }): Promise<void> {
			if (options?.recursive) {
				const parts = p.split("/").filter(Boolean);
				let current = "/";
				for (const part of parts) {
					current = path.join(current, part);
					const dirExists = await backend.exists(current);
					if (!dirExists) {
						await client.mkdirFs({ path: resolve(current) });
					}
				}
			} else {
				const parent = path.dirname(p);
				if (parent !== "/" && parent !== ".") {
					const parentExists = await backend.exists(parent);
					if (!parentExists) {
						throw new KernelError("ENOENT", `no such directory: ${parent}`);
					}
				}
				await client.mkdirFs({ path: resolve(p) });
			}
		},

		async exists(p: string): Promise<boolean> {
			try {
				await client.statFs({ path: resolve(p) });
				return true;
			} catch (err) {
				if (isNotFound(err)) {
					return false;
				}
				throw err;
			}
		},

		async stat(p: string): Promise<VirtualStat> {
			try {
				const s = await client.statFs({ path: resolve(p) });
				return makeStat(s.size, s.entryType === "directory", s.modified);
			} catch (err) {
				if (isNotFound(err)) {
					throw new KernelError("ENOENT", `no such file or directory: ${p}`);
				}
				throw err;
			}
		},

		async removeFile(p: string): Promise<void> {
			await client.deleteFsEntry({ path: resolve(p) });
		},

		async removeDir(p: string): Promise<void> {
			const entries = await client.listFsEntries({ path: resolve(p) });
			const children = entries.filter((e) => e.name !== "." && e.name !== "..");
			if (children.length > 0) {
				throw new KernelError("ENOTEMPTY", `directory not empty: ${p}`);
			}
			await client.deleteFsEntry({ path: resolve(p) });
		},

		async rename(oldPath: string, newPath: string): Promise<void> {
			await client.moveFs({ from: resolve(oldPath), to: resolve(newPath), overwrite: true });
		},

		async realpath(p: string): Promise<string> {
			return path.normalize(p.startsWith("/") ? p : `/${p}`);
		},

		async symlink(_target: string, _linkPath: string): Promise<void> {
			throw new KernelError("ENOSYS", "symlink not supported by sandbox backend");
		},

		async readlink(_p: string): Promise<string> {
			throw new KernelError("ENOSYS", "readlink not supported by sandbox backend");
		},

		async lstat(p: string): Promise<VirtualStat> {
			return backend.stat(p);
		},

		async link(_oldPath: string, _newPath: string): Promise<void> {
			throw new KernelError("ENOSYS", "link not supported by sandbox backend");
		},

		async chmod(_p: string, _mode: number): Promise<void> {
			throw new KernelError("ENOSYS", "chmod not supported by sandbox backend");
		},

		async chown(_p: string, _uid: number, _gid: number): Promise<void> {
			throw new KernelError("ENOSYS", "chown not supported by sandbox backend");
		},

		async utimes(_p: string, _atime: number, _mtime: number): Promise<void> {
			throw new KernelError("ENOSYS", "utimes not supported by sandbox backend");
		},

		async truncate(p: string, length: number): Promise<void> {
			if (length === 0) {
				await backend.writeFile(p, new Uint8Array(0));
				return;
			}
			const data = await backend.readFile(p);
			if (length <= data.length) {
				await backend.writeFile(p, data.slice(0, length));
			} else {
				const extended = new Uint8Array(length);
				extended.set(data);
				await backend.writeFile(p, extended);
			}
		},

		async pread(
			p: string,
			offset: number,
			length: number,
		): Promise<Uint8Array> {
			const data = await backend.readFile(p);
			return data.slice(offset, offset + length);
		},

		async pwrite(
			p: string,
			offset: number,
			data: Uint8Array,
		): Promise<void> {
			const content = await backend.readFile(p);
			const end = offset + data.length;
			const newSize = Math.max(content.length, end);
			const buf = new Uint8Array(newSize);
			buf.set(content);
			buf.set(data, offset);
			await backend.writeFile(p, buf);
		},
	};

	return backend;
}

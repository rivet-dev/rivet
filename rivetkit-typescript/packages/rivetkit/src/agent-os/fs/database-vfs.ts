/**
 * SQLite-backed VirtualFileSystem implementation.
 *
 * Stores file content, metadata (mode, timestamps), and directory structure
 * in SQLite tables managed by the RivetKit actor's database. This allows VM
 * filesystem state to persist across sleep/wake cycles.
 *
 * We use SQLite instead of actor KV because SQLite handles bulk write
 * optimizations (transactions, WAL mode, page caching) under the hood. With KV
 * we would need to manually chunk writes to stay under batch size limits,
 * implement our own indexing for directory listing queries, and handle
 * consistency across multiple KV operations. SQLite gives us all of this for
 * free.
 *
 * All paths are normalized to POSIX form (forward slashes, rooted at "/").
 */

import * as posixPath from "node:path/posix";
import type { RawAccess } from "@/db/config";

// Infer VirtualFileSystem from PlainMountConfig.driver since
// @secure-exec/core is not a direct dependency of this package.
type VirtualFileSystem =
	import("@rivet-dev/agent-os-core").PlainMountConfig["driver"];

// Infer VirtualStat from AgentOs.stat() return type.
type VirtualStat = Awaited<
	ReturnType<import("@rivet-dev/agent-os-core").AgentOs["stat"]>
>;

// Infer VirtualDirEntry from readDirWithTypes.
// VirtualDirEntry has: name, isDirectory, isSymbolicLink?, ino?
interface VirtualDirEntry {
	name: string;
	isDirectory: boolean;
	isSymbolicLink?: boolean;
	ino?: number;
}

// POSIX mode constants.
const S_IFDIR = 0o040000;
const S_IFREG = 0o100000;
const S_IFLNK = 0o120000;
const DEFAULT_FILE_MODE = S_IFREG | 0o644;
const DEFAULT_DIR_MODE = S_IFDIR | 0o755;

interface FsRow extends Record<string, unknown> {
	path: string;
	is_directory: number;
	content: Uint8Array | null;
	mode: number;
	uid: number;
	gid: number;
	size: number;
	atime_ms: number;
	mtime_ms: number;
	ctime_ms: number;
	birthtime_ms: number;
	symlink_target: string | null;
	nlink: number;
}

function normPath(p: string): string {
	const normalized = posixPath.normalize(`/${p}`);
	// Remove trailing slash unless it's the root.
	if (normalized.length > 1 && normalized.endsWith("/")) {
		return normalized.slice(0, -1);
	}
	return normalized;
}

function parentPath(p: string): string {
	const parent = posixPath.dirname(p);
	return parent;
}

function throwENOENT(path: string): never {
	const err = new Error(`ENOENT: no such file or directory: ${path}`);
	err.name = "ENOENT";
	throw err;
}

function throwEEXIST(path: string): never {
	const err = new Error(`EEXIST: file already exists: ${path}`);
	err.name = "EEXIST";
	throw err;
}

function throwENOTDIR(path: string): never {
	const err = new Error(`ENOTDIR: not a directory: ${path}`);
	err.name = "ENOTDIR";
	throw err;
}

function throwEISDIR(path: string): never {
	const err = new Error(`EISDIR: illegal operation on a directory: ${path}`);
	err.name = "EISDIR";
	throw err;
}

function throwENOTEMPTY(path: string): never {
	const err = new Error(`ENOTEMPTY: directory not empty: ${path}`);
	err.name = "ENOTEMPTY";
	throw err;
}

function throwENOSYS(op: string): never {
	const err = new Error(`ENOSYS: function not implemented: ${op}`);
	err.name = "ENOSYS";
	throw err;
}

function rowToStat(row: FsRow): VirtualStat {
	return {
		mode: row.mode,
		size: row.size,
		isDirectory: row.is_directory === 1,
		isSymbolicLink: row.symlink_target !== null,
		atimeMs: row.atime_ms,
		mtimeMs: row.mtime_ms,
		ctimeMs: row.ctime_ms,
		birthtimeMs: row.birthtime_ms,
		ino: 0,
		nlink: row.nlink,
		uid: row.uid,
		gid: row.gid,
	};
}

export interface DatabaseVfsOptions {
	/** The RawAccess database handle from the actor's db provider. */
	db: RawAccess;
}

/**
 * Create a VirtualFileSystem backed by SQLite.
 *
 * The returned filesystem stores all content and metadata in the
 * `agent_os_fs_entries` table. The table must be created beforehand
 * via `migrateAgentOsTables()`.
 */
export function createDatabaseVfs(
	options: DatabaseVfsOptions,
): VirtualFileSystem {
	const { db } = options;

	async function getEntry(path: string): Promise<FsRow | undefined> {
		const rows = await db.execute<FsRow>(
			"SELECT * FROM agent_os_fs_entries WHERE path = ?",
			path,
		);
		return rows[0];
	}

	async function getEntryOrThrow(path: string): Promise<FsRow> {
		const entry = await getEntry(path);
		if (!entry) {
			throwENOENT(path);
		}
		return entry;
	}

	async function ensureParentExists(path: string): Promise<void> {
		const parent = parentPath(path);
		if (parent === path) return; // root
		const entry = await getEntry(parent);
		if (!entry) {
			throwENOENT(parent);
		}
		if (entry.is_directory !== 1) {
			throwENOTDIR(parent);
		}
	}

	async function getChildEntries(dirPath: string): Promise<FsRow[]> {
		// Find direct children by matching paths that are one level deeper.
		// A direct child of "/foo" has path like "/foo/bar" but NOT "/foo/bar/baz".
		const prefix = dirPath === "/" ? "/" : `${dirPath}/`;
		const rows = await db.execute<FsRow>(
			"SELECT * FROM agent_os_fs_entries WHERE path LIKE ? AND path != ?",
			`${prefix}%`,
			dirPath,
		);
		// Filter to direct children only.
		return rows.filter((row) => {
			const relative = row.path.slice(prefix.length);
			return relative.length > 0 && !relative.includes("/");
		});
	}

	// Ensure root directory exists.
	const rootInit = (async () => {
		const root = await getEntry("/");
		if (!root) {
			const now = Date.now();
			await db.execute(
				`INSERT OR IGNORE INTO agent_os_fs_entries (path, is_directory, content, mode, uid, gid, size, atime_ms, mtime_ms, ctime_ms, birthtime_ms, symlink_target, nlink) VALUES (?, 1, NULL, ?, 0, 0, 0, ?, ?, ?, ?, NULL, 2)`,
				"/",
				DEFAULT_DIR_MODE,
				now,
				now,
				now,
				now,
			);
		}
	})();

	const backend: VirtualFileSystem = {
		async readFile(p: string): Promise<Uint8Array> {
			await rootInit;
			const path = normPath(p);
			const entry = await getEntryOrThrow(path);
			if (entry.is_directory === 1) {
				throwEISDIR(path);
			}
			return entry.content ?? new Uint8Array(0);
		},

		async readTextFile(p: string): Promise<string> {
			const data = await backend.readFile(p);
			return new TextDecoder().decode(data);
		},

		async readDir(p: string): Promise<string[]> {
			await rootInit;
			const path = normPath(p);
			const entry = await getEntryOrThrow(path);
			if (entry.is_directory !== 1) {
				throwENOTDIR(path);
			}
			const children = await getChildEntries(path);
			return children.map((child) => posixPath.basename(child.path));
		},

		async readDirWithTypes(p: string): Promise<VirtualDirEntry[]> {
			await rootInit;
			const path = normPath(p);
			const entry = await getEntryOrThrow(path);
			if (entry.is_directory !== 1) {
				throwENOTDIR(path);
			}
			const children = await getChildEntries(path);
			return children.map((child) => ({
				name: posixPath.basename(child.path),
				isDirectory: child.is_directory === 1,
				isSymbolicLink: child.symlink_target !== null,
				ino: 0,
			}));
		},

		async writeFile(
			p: string,
			content: string | Uint8Array,
		): Promise<void> {
			await rootInit;
			const path = normPath(p);
			await ensureParentExists(path);

			const existing = await getEntry(path);
			if (existing && existing.is_directory === 1) {
				throwEISDIR(path);
			}

			const data =
				typeof content === "string"
					? new TextEncoder().encode(content)
					: content;
			const now = Date.now();

			if (existing) {
				await db.execute(
					`UPDATE agent_os_fs_entries SET content = ?, size = ?, mtime_ms = ?, ctime_ms = ?, atime_ms = ? WHERE path = ?`,
					data,
					data.byteLength,
					now,
					now,
					now,
					path,
				);
			} else {
				await db.execute(
					`INSERT INTO agent_os_fs_entries (path, is_directory, content, mode, uid, gid, size, atime_ms, mtime_ms, ctime_ms, birthtime_ms, symlink_target, nlink) VALUES (?, 0, ?, ?, 0, 0, ?, ?, ?, ?, ?, NULL, 1)`,
					path,
					data,
					DEFAULT_FILE_MODE,
					data.byteLength,
					now,
					now,
					now,
					now,
				);
			}
		},

		async createDir(p: string): Promise<void> {
			await rootInit;
			const path = normPath(p);
			await ensureParentExists(path);

			const existing = await getEntry(path);
			if (existing) {
				throwEEXIST(path);
			}

			const now = Date.now();
			await db.execute(
				`INSERT INTO agent_os_fs_entries (path, is_directory, content, mode, uid, gid, size, atime_ms, mtime_ms, ctime_ms, birthtime_ms, symlink_target, nlink) VALUES (?, 1, NULL, ?, 0, 0, 0, ?, ?, ?, ?, NULL, 2)`,
				path,
				DEFAULT_DIR_MODE,
				now,
				now,
				now,
				now,
			);
		},

		async mkdir(
			p: string,
			options?: { recursive?: boolean },
		): Promise<void> {
			await rootInit;
			const path = normPath(p);

			if (options?.recursive) {
				const parts = path.split("/").filter(Boolean);
				let current = "";
				for (const part of parts) {
					current += `/${part}`;
					const existing = await getEntry(current);
					if (!existing) {
						const now = Date.now();
						await db.execute(
							`INSERT INTO agent_os_fs_entries (path, is_directory, content, mode, uid, gid, size, atime_ms, mtime_ms, ctime_ms, birthtime_ms, symlink_target, nlink) VALUES (?, 1, NULL, ?, 0, 0, 0, ?, ?, ?, ?, NULL, 2)`,
							current,
							DEFAULT_DIR_MODE,
							now,
							now,
							now,
							now,
						);
					} else if (existing.is_directory !== 1) {
						throwENOTDIR(current);
					}
				}
			} else {
				await backend.createDir(p);
			}
		},

		async exists(p: string): Promise<boolean> {
			await rootInit;
			const path = normPath(p);
			const entry = await getEntry(path);
			return entry !== undefined;
		},

		async stat(p: string): Promise<VirtualStat> {
			await rootInit;
			const path = normPath(p);
			const entry = await getEntryOrThrow(path);
			return rowToStat(entry);
		},

		async removeFile(p: string): Promise<void> {
			await rootInit;
			const path = normPath(p);
			const entry = await getEntryOrThrow(path);
			if (entry.is_directory === 1) {
				throwEISDIR(path);
			}
			await db.execute(
				"DELETE FROM agent_os_fs_entries WHERE path = ?",
				path,
			);
		},

		async removeDir(p: string): Promise<void> {
			await rootInit;
			const path = normPath(p);
			const entry = await getEntryOrThrow(path);
			if (entry.is_directory !== 1) {
				throwENOTDIR(path);
			}
			const children = await getChildEntries(path);
			if (children.length > 0) {
				throwENOTEMPTY(path);
			}
			await db.execute(
				"DELETE FROM agent_os_fs_entries WHERE path = ?",
				path,
			);
		},

		async rename(oldPath: string, newPath: string): Promise<void> {
			await rootInit;
			const from = normPath(oldPath);
			const to = normPath(newPath);

			const entry = await getEntryOrThrow(from);
			await ensureParentExists(to);

			// Remove destination if it exists (overwrite semantics).
			const destEntry = await getEntry(to);
			if (destEntry) {
				if (destEntry.is_directory === 1) {
					const children = await getChildEntries(to);
					if (children.length > 0) {
						throwENOTEMPTY(to);
					}
				}
				await db.execute(
					"DELETE FROM agent_os_fs_entries WHERE path = ?",
					to,
				);
			}

			if (entry.is_directory === 1) {
				// Move all descendants by updating path prefixes.
				const prefix = from === "/" ? "/" : `${from}/`;
				const newPrefix = to === "/" ? "/" : `${to}/`;

				// Get all descendants first, then update them.
				const descendants = await db.execute<FsRow>(
					"SELECT path FROM agent_os_fs_entries WHERE path LIKE ?",
					`${prefix}%`,
				);

				for (const desc of descendants) {
					const newDescPath =
						newPrefix + desc.path.slice(prefix.length);
					await db.execute(
						"UPDATE agent_os_fs_entries SET path = ? WHERE path = ?",
						newDescPath,
						desc.path,
					);
				}
			}

			// Update the entry itself.
			const now = Date.now();
			await db.execute(
				"UPDATE agent_os_fs_entries SET path = ?, ctime_ms = ? WHERE path = ?",
				to,
				now,
				from,
			);
		},

		async realpath(p: string): Promise<string> {
			await rootInit;
			const path = normPath(p);
			const entry = await getEntryOrThrow(path);
			if (entry.symlink_target !== null) {
				return normPath(entry.symlink_target);
			}
			return path;
		},

		async symlink(target: string, linkPath: string): Promise<void> {
			await rootInit;
			const link = normPath(linkPath);
			await ensureParentExists(link);

			const existing = await getEntry(link);
			if (existing) {
				throwEEXIST(link);
			}

			const now = Date.now();
			await db.execute(
				`INSERT INTO agent_os_fs_entries (path, is_directory, content, mode, uid, gid, size, atime_ms, mtime_ms, ctime_ms, birthtime_ms, symlink_target, nlink) VALUES (?, 0, NULL, ?, 0, 0, ?, ?, ?, ?, ?, ?, 1)`,
				link,
				S_IFLNK | 0o777,
				target.length,
				now,
				now,
				now,
				now,
				target,
			);
		},

		async readlink(p: string): Promise<string> {
			await rootInit;
			const path = normPath(p);
			const entry = await getEntryOrThrow(path);
			if (entry.symlink_target === null) {
				const err = new Error(`EINVAL: not a symlink: ${path}`);
				err.name = "EINVAL";
				throw err;
			}
			return entry.symlink_target;
		},

		async lstat(p: string): Promise<VirtualStat> {
			// lstat does not follow symlinks; same as stat for our storage model.
			return backend.stat(p);
		},

		async link(oldPath: string, newPath: string): Promise<void> {
			throwENOSYS("link");
		},

		async chmod(p: string, mode: number): Promise<void> {
			await rootInit;
			const path = normPath(p);
			await getEntryOrThrow(path);
			const now = Date.now();
			await db.execute(
				"UPDATE agent_os_fs_entries SET mode = ?, ctime_ms = ? WHERE path = ?",
				mode,
				now,
				path,
			);
		},

		async chown(p: string, uid: number, gid: number): Promise<void> {
			await rootInit;
			const path = normPath(p);
			await getEntryOrThrow(path);
			const now = Date.now();
			await db.execute(
				"UPDATE agent_os_fs_entries SET uid = ?, gid = ?, ctime_ms = ? WHERE path = ?",
				uid,
				gid,
				now,
				path,
			);
		},

		async utimes(p: string, atime: number, mtime: number): Promise<void> {
			await rootInit;
			const path = normPath(p);
			await getEntryOrThrow(path);
			const now = Date.now();
			await db.execute(
				"UPDATE agent_os_fs_entries SET atime_ms = ?, mtime_ms = ?, ctime_ms = ? WHERE path = ?",
				atime,
				mtime,
				now,
				path,
			);
		},

		async truncate(p: string, length: number): Promise<void> {
			await rootInit;
			const path = normPath(p);
			const entry = await getEntryOrThrow(path);
			if (entry.is_directory === 1) {
				throwEISDIR(path);
			}

			const existing = entry.content ?? new Uint8Array(0);
			let newContent: Uint8Array;
			if (length >= existing.byteLength) {
				// Extend with zeros.
				newContent = new Uint8Array(length);
				newContent.set(existing);
			} else {
				// Truncate.
				newContent = existing.slice(0, length);
			}

			const now = Date.now();
			await db.execute(
				"UPDATE agent_os_fs_entries SET content = ?, size = ?, mtime_ms = ?, ctime_ms = ? WHERE path = ?",
				newContent,
				length,
				now,
				now,
				path,
			);
		},

		async pread(
			p: string,
			offset: number,
			length: number,
		): Promise<Uint8Array> {
			await rootInit;
			const path = normPath(p);
			const entry = await getEntryOrThrow(path);
			if (entry.is_directory === 1) {
				throwEISDIR(path);
			}
			const content = entry.content ?? new Uint8Array(0);
			const end = Math.min(offset + length, content.byteLength);
			if (offset >= content.byteLength) {
				return new Uint8Array(0);
			}
			return content.slice(offset, end);
		},

		async pwrite(
			p: string,
			offset: number,
			data: Uint8Array,
		): Promise<void> {
			await rootInit;
			const path = normPath(p);
			const entry = await getEntryOrThrow(path);
			if (entry.is_directory === 1) {
				throwEISDIR(path);
			}
			const content = entry.content ?? new Uint8Array(0);
			const end = offset + data.byteLength;
			const newSize = Math.max(content.byteLength, end);
			const buf = new Uint8Array(newSize);
			buf.set(content);
			buf.set(data, offset);
			const now = Date.now();
			await db.execute(
				`UPDATE agent_os_fs_entries SET content = ?, size = ?, mtime_ms = ?, ctime_ms = ? WHERE path = ?`,
				buf,
				newSize,
				now,
				now,
				path,
			);
		},
	};

	return backend;
}

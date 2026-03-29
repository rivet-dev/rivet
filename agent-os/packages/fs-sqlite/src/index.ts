/**
 * SQLite filesystem backend.
 *
 * Stores all files, directories, and symlinks in a SQLite database.
 * Supports the full VirtualFileSystem interface including symlinks,
 * hard links, permissions, and timestamps.
 */

import Database from "better-sqlite3";
import * as path from "node:path";
import {
	KernelError,
	type VirtualDirEntry,
	type VirtualFileSystem,
	type VirtualStat,
} from "@secure-exec/core";

export interface SqliteFsOptions {
	/** Path to the SQLite database file. Use ":memory:" for in-memory. */
	dbPath: string;
}

interface FsRow {
	path: string;
	entry_type: "file" | "directory" | "symlink";
	content: Buffer | null;
	mode: number;
	uid: number;
	gid: number;
	size: number;
	nlink: number;
	symlink_target: string | null;
	atime_ms: number;
	mtime_ms: number;
	ctime_ms: number;
	birthtime_ms: number;
}

const MAX_SYMLINK_DEPTH = 40;

function normalizePath(p: string): string {
	const normalized = path.posix.normalize(p);
	// Ensure leading slash and no trailing slash (except root).
	const withSlash = normalized.startsWith("/") ? normalized : `/${normalized}`;
	return withSlash === "/" ? "/" : withSlash.replace(/\/+$/, "");
}

/**
 * Escape LIKE metacharacters (% and _) in a string for use in SQLite LIKE patterns.
 */
function escapeLike(s: string): string {
	return s.replace(/[%_\\]/g, (ch) => `\\${ch}`);
}

function parentPath(p: string): string {
	const parent = path.posix.dirname(p);
	return parent === "." ? "/" : parent;
}

/**
 * Create a VirtualFileSystem backed by SQLite.
 */
export function createSqliteBackend(options: SqliteFsOptions): VirtualFileSystem {
	const db = new Database(options.dbPath);

	// Enable WAL mode for better concurrent read performance.
	db.pragma("journal_mode = WAL");

	// Create schema.
	db.exec(`
		CREATE TABLE IF NOT EXISTS fs_entries (
			path TEXT PRIMARY KEY,
			entry_type TEXT NOT NULL CHECK(entry_type IN ('file', 'directory', 'symlink')),
			content BLOB,
			mode INTEGER NOT NULL DEFAULT 33188,
			uid INTEGER NOT NULL DEFAULT 0,
			gid INTEGER NOT NULL DEFAULT 0,
			size INTEGER NOT NULL DEFAULT 0,
			nlink INTEGER NOT NULL DEFAULT 1,
			symlink_target TEXT,
			atime_ms INTEGER NOT NULL,
			mtime_ms INTEGER NOT NULL,
			ctime_ms INTEGER NOT NULL,
			birthtime_ms INTEGER NOT NULL
		)
	`);

	// Create root directory if it doesn't exist.
	const now = Date.now();
	db.prepare(`
		INSERT OR IGNORE INTO fs_entries (path, entry_type, mode, size, nlink, atime_ms, mtime_ms, ctime_ms, birthtime_ms)
		VALUES ('/', 'directory', 16877, 0, 1, ?, ?, ?, ?)
	`).run(now, now, now, now);

	// Prepared statements.
	const stmtGet = db.prepare("SELECT * FROM fs_entries WHERE path = ?");
	const stmtInsert = db.prepare(`
		INSERT OR REPLACE INTO fs_entries (path, entry_type, content, mode, uid, gid, size, nlink, symlink_target, atime_ms, mtime_ms, ctime_ms, birthtime_ms)
		VALUES (@path, @entry_type, @content, @mode, @uid, @gid, @size, @nlink, @symlink_target, @atime_ms, @mtime_ms, @ctime_ms, @birthtime_ms)
	`);
	const stmtDelete = db.prepare("DELETE FROM fs_entries WHERE path = ?");
	const stmtExists = db.prepare("SELECT 1 FROM fs_entries WHERE path = ? LIMIT 1");

	function getRow(p: string): FsRow | undefined {
		return stmtGet.get(normalizePath(p)) as FsRow | undefined;
	}

	/**
	 * Resolve a single path component that may be a symlink.
	 * Returns the resolved path if it's a symlink, or the original if not.
	 */
	function resolveOneSymlink(p: string, depth: number): string {
		if (depth > MAX_SYMLINK_DEPTH) {
			throw new KernelError("EINVAL", `too many symlinks: ${p}`);
		}
		const row = getRow(p);
		if (!row) return p;
		if (row.entry_type === "symlink" && row.symlink_target) {
			const target = row.symlink_target.startsWith("/")
				? row.symlink_target
				: path.posix.resolve(parentPath(p), row.symlink_target);
			return resolveSymlinks(target, depth + 1);
		}
		return p;
	}

	/**
	 * Resolve symlinks in all path components, not just the final one.
	 * For example, /symlink-to-dir/file.txt will follow the symlink in
	 * the directory component.
	 */
	function resolveSymlinks(p: string, depth = 0): string {
		if (depth > MAX_SYMLINK_DEPTH) {
			throw new KernelError("EINVAL", `too many symlinks: ${p}`);
		}
		const normalized = normalizePath(p);
		const parts = normalized.split("/").filter(Boolean);
		let current = "";
		for (let i = 0; i < parts.length; i++) {
			current += `/${parts[i]}`;
			current = resolveOneSymlink(current, depth);
		}
		return current || "/";
	}

	/**
	 * Resolve symlinks in all intermediate path components but NOT the final one.
	 * Used by removeFile so it deletes the symlink entry itself, not its target.
	 */
	function resolveIntermediateSymlinks(p: string): string {
		const normalized = normalizePath(p);
		const parent = parentPath(normalized);
		const name = path.posix.basename(normalized);
		if (parent === "/" && name === "") return "/";
		const resolvedParent = resolveSymlinks(parent);
		return resolvedParent === "/" ? `/${name}` : `${resolvedParent}/${name}`;
	}

	function ensureParentDirs(p: string): void {
		const parts = normalizePath(p).split("/").filter(Boolean);
		let current = "";
		for (let i = 0; i < parts.length - 1; i++) {
			current += `/${parts[i]}`;
			const existing = getRow(current);
			if (!existing) {
				const ts = Date.now();
				stmtInsert.run({
					path: current,
					entry_type: "directory",
					content: null,
					mode: 0o40755,
					uid: 0,
					gid: 0,
					size: 0,
					nlink: 1,
					symlink_target: null,
					atime_ms: ts,
					mtime_ms: ts,
					ctime_ms: ts,
					birthtime_ms: ts,
				});
			}
		}
	}

	function rowToStat(row: FsRow): VirtualStat {
		return {
			mode: row.mode,
			size: row.size,
			isDirectory: row.entry_type === "directory",
			isSymbolicLink: row.entry_type === "symlink",
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

	const backend: VirtualFileSystem = {
		async readFile(p: string): Promise<Uint8Array> {
			const resolved = resolveSymlinks(p);
			const row = getRow(resolved);
			if (!row) {
				throw new KernelError("ENOENT", `no such file: ${p}`);
			}
			if (row.entry_type === "directory") {
				throw new KernelError("EISDIR", `is a directory: ${p}`);
			}
			if (!row.content) return new Uint8Array(0);
			return new Uint8Array(row.content);
		},

		async readTextFile(p: string): Promise<string> {
			const data = await backend.readFile(p);
			return new TextDecoder().decode(data);
		},

		async readDir(p: string): Promise<string[]> {
			const resolved = resolveSymlinks(p);
			const normalized = normalizePath(resolved);
			const prefix = normalized === "/" ? "/" : `${normalized}/`;

			const rows = db.prepare(
				"SELECT path FROM fs_entries WHERE path LIKE ? ESCAPE '\\' AND path != ?",
			).all(`${escapeLike(prefix)}%`, normalized) as { path: string }[];

			const names: string[] = [];
			for (const row of rows) {
				const relative = row.path.slice(prefix.length);
				if (relative && !relative.includes("/")) {
					names.push(relative);
				}
			}
			return names;
		},

		async readDirWithTypes(p: string): Promise<VirtualDirEntry[]> {
			const resolved = resolveSymlinks(p);
			const normalized = normalizePath(resolved);
			const prefix = normalized === "/" ? "/" : `${normalized}/`;

			const rows = db.prepare(
				"SELECT path, entry_type FROM fs_entries WHERE path LIKE ? ESCAPE '\\' AND path != ?",
			).all(`${escapeLike(prefix)}%`, normalized) as { path: string; entry_type: string }[];

			const entries: VirtualDirEntry[] = [];
			for (const row of rows) {
				const relative = row.path.slice(prefix.length);
				if (relative && !relative.includes("/")) {
					entries.push({
						name: relative,
						isDirectory: row.entry_type === "directory",
						isSymbolicLink: row.entry_type === "symlink",
					});
				}
			}
			return entries;
		},

		async writeFile(p: string, content: string | Uint8Array): Promise<void> {
			const resolved = resolveSymlinks(p);
			const normalized = normalizePath(resolved);
			ensureParentDirs(normalized);

			const body =
				typeof content === "string"
					? Buffer.from(content, "utf-8")
					: Buffer.from(content);
			const ts = Date.now();

			const existing = getRow(normalized);
			stmtInsert.run({
				path: normalized,
				entry_type: "file",
				content: body,
				mode: existing?.mode ?? 0o100644,
				uid: existing?.uid ?? 0,
				gid: existing?.gid ?? 0,
				size: body.length,
				nlink: existing?.nlink ?? 1,
				symlink_target: null,
				atime_ms: ts,
				mtime_ms: ts,
				ctime_ms: existing?.ctime_ms ?? ts,
				birthtime_ms: existing?.birthtime_ms ?? ts,
			});
		},

		async createDir(p: string): Promise<void> {
			return backend.mkdir(p);
		},

		async mkdir(p: string, options?: { recursive?: boolean }): Promise<void> {
			const normalized = normalizePath(p);

			if (options?.recursive) {
				ensureParentDirs(normalized);
			} else {
				// Without recursive, verify the parent exists.
				const parent = parentPath(normalized);
				if (parent !== "/" && !getRow(parent)) {
					throw new KernelError("ENOENT", `parent directory does not exist: ${parent}`);
				}
			}

			const existing = getRow(normalized);
			if (existing) {
				if (existing.entry_type === "directory") return;
				throw new KernelError("EEXIST", `path already exists: ${p}`);
			}

			const ts = Date.now();
			stmtInsert.run({
				path: normalized,
				entry_type: "directory",
				content: null,
				mode: 0o40755,
				uid: 0,
				gid: 0,
				size: 0,
				nlink: 1,
				symlink_target: null,
				atime_ms: ts,
				mtime_ms: ts,
				ctime_ms: ts,
				birthtime_ms: ts,
			});
		},

		async exists(p: string): Promise<boolean> {
			try {
				const resolved = resolveSymlinks(p);
				const result = stmtExists.get(resolved);
				return result !== undefined;
			} catch {
				return false;
			}
		},

		async stat(p: string): Promise<VirtualStat> {
			const resolved = resolveSymlinks(p);
			const row = getRow(resolved);
			if (!row) {
				throw new KernelError("ENOENT", `no such file or directory: ${p}`);
			}
			return rowToStat(row);
		},

		async lstat(p: string): Promise<VirtualStat> {
			const normalized = normalizePath(p);
			const row = getRow(normalized);
			if (!row) {
				throw new KernelError("ENOENT", `no such file or directory: ${p}`);
			}
			return rowToStat(row);
		},

		async removeFile(p: string): Promise<void> {
			// Resolve intermediate symlinks but NOT the final component.
			const normalized = resolveIntermediateSymlinks(p);
			const row = getRow(normalized);
			if (!row) {
				throw new KernelError("ENOENT", `no such file: ${p}`);
			}
			if (row.entry_type === "directory") {
				throw new KernelError("EISDIR", `is a directory: ${p}`);
			}
			stmtDelete.run(normalized);
		},

		async removeDir(p: string): Promise<void> {
			const normalized = normalizePath(p);
			const row = getRow(normalized);
			if (!row) {
				throw new KernelError("ENOENT", `no such directory: ${p}`);
			}
			if (row.entry_type !== "directory") {
				throw new KernelError("ENOTDIR", `not a directory: ${p}`);
			}

			const children = await backend.readDir(p);
			if (children.length > 0) {
				throw new KernelError("ENOTEMPTY", `directory not empty: ${p}`);
			}

			stmtDelete.run(normalized);
		},

		async rename(oldPath: string, newPath: string): Promise<void> {
			const oldNorm = normalizePath(oldPath);
			const newNorm = normalizePath(newPath);

			const renameTransaction = db.transaction(() => {
				const row = getRow(oldNorm);
				if (!row) {
					throw new KernelError("ENOENT", `no such file or directory: ${oldPath}`);
				}

				ensureParentDirs(newNorm);

				const ts = Date.now();

				// Check if destination exists.
				const destRow = getRow(newNorm);
				if (destRow && destRow.entry_type === "directory") {
					// Check if the destination directory is non-empty.
					const destPrefix = newNorm === "/" ? "/" : `${newNorm}/`;
					const destChildren = db.prepare(
						"SELECT 1 FROM fs_entries WHERE path LIKE ? ESCAPE '\\' AND path != ? LIMIT 1",
					).get(`${escapeLike(destPrefix)}%`, newNorm);
					if (destChildren) {
						throw new KernelError("ENOTEMPTY", `destination directory not empty: ${newPath}`);
					}
				}

				if (row.entry_type === "directory") {
					// Move all children by updating their path prefix.
					const oldPrefix = oldNorm === "/" ? "/" : `${oldNorm}/`;
					const newPrefix = newNorm === "/" ? "/" : `${newNorm}/`;

					const children = db.prepare(
						"SELECT path FROM fs_entries WHERE path LIKE ? ESCAPE '\\'",
					).all(`${escapeLike(oldPrefix)}%`) as { path: string }[];

					for (const child of children) {
						const newChildPath = newPrefix + child.path.slice(oldPrefix.length);
						db.prepare("UPDATE fs_entries SET path = ?, mtime_ms = ? WHERE path = ?")
							.run(newChildPath, ts, child.path);
					}
				}

				// Delete any existing entry at the destination.
				stmtDelete.run(newNorm);
				// Move the entry itself.
				db.prepare("UPDATE fs_entries SET path = ?, mtime_ms = ? WHERE path = ?")
					.run(newNorm, ts, oldNorm);
			});

			renameTransaction();
		},

		async realpath(p: string): Promise<string> {
			return resolveSymlinks(p);
		},

		async symlink(target: string, linkPath: string): Promise<void> {
			const normalized = normalizePath(linkPath);
			ensureParentDirs(normalized);

			const existing = getRow(normalized);
			if (existing) {
				throw new KernelError("EEXIST", `path already exists: ${linkPath}`);
			}

			const ts = Date.now();
			stmtInsert.run({
				path: normalized,
				entry_type: "symlink",
				content: null,
				mode: 0o120777,
				uid: 0,
				gid: 0,
				size: target.length,
				nlink: 1,
				symlink_target: target,
				atime_ms: ts,
				mtime_ms: ts,
				ctime_ms: ts,
				birthtime_ms: ts,
			});
		},

		async readlink(p: string): Promise<string> {
			const normalized = normalizePath(p);
			const row = getRow(normalized);
			if (!row || row.entry_type !== "symlink" || !row.symlink_target) {
				throw new KernelError("EINVAL", `not a symlink: ${p}`);
			}
			return row.symlink_target;
		},

		async link(existingPath: string, newPath: string): Promise<void> {
			const resolved = resolveSymlinks(existingPath);
			const row = getRow(resolved);
			if (!row) {
				throw new KernelError("ENOENT", `no such file: ${existingPath}`);
			}
			if (row.entry_type === "directory") {
				throw new KernelError("EPERM", `cannot hard-link a directory: ${existingPath}`);
			}

			const newNorm = normalizePath(newPath);
			ensureParentDirs(newNorm);

			const ts = Date.now();
			stmtInsert.run({
				path: newNorm,
				entry_type: row.entry_type,
				content: row.content,
				mode: row.mode,
				uid: row.uid,
				gid: row.gid,
				size: row.size,
				nlink: row.nlink + 1,
				symlink_target: row.symlink_target,
				atime_ms: ts,
				mtime_ms: ts,
				ctime_ms: row.ctime_ms,
				birthtime_ms: row.birthtime_ms,
			});

			// Increment nlink on the original.
			db.prepare("UPDATE fs_entries SET nlink = nlink + 1 WHERE path = ?")
				.run(resolved);
		},

		async chmod(p: string, mode: number): Promise<void> {
			const resolved = resolveSymlinks(p);
			const normalized = normalizePath(resolved);
			const row = getRow(normalized);
			if (!row) {
				throw new KernelError("ENOENT", `no such file or directory: ${p}`);
			}
			// Preserve file type bits (upper bits) and only change permission bits (lower 12 bits).
			const typeBits = row.mode & 0o170000;
			const newMode = typeBits | (mode & 0o7777);
			const ts = Date.now();
			db.prepare("UPDATE fs_entries SET mode = ?, ctime_ms = ? WHERE path = ?")
				.run(newMode, ts, normalized);
		},

		async chown(p: string, uid: number, gid: number): Promise<void> {
			const resolved = resolveSymlinks(p);
			const normalized = normalizePath(resolved);
			const row = getRow(normalized);
			if (!row) {
				throw new KernelError("ENOENT", `no such file or directory: ${p}`);
			}
			const ts = Date.now();
			db.prepare("UPDATE fs_entries SET uid = ?, gid = ?, ctime_ms = ? WHERE path = ?")
				.run(uid, gid, ts, normalized);
		},

		async utimes(p: string, atime: number, mtime: number): Promise<void> {
			const resolved = resolveSymlinks(p);
			const normalized = normalizePath(resolved);
			const row = getRow(normalized);
			if (!row) {
				throw new KernelError("ENOENT", `no such file or directory: ${p}`);
			}
			db.prepare("UPDATE fs_entries SET atime_ms = ?, mtime_ms = ? WHERE path = ?")
				.run(atime, mtime, normalized);
		},

		async truncate(p: string, length: number): Promise<void> {
			const resolved = resolveSymlinks(p);
			const normalized = normalizePath(resolved);
			const row = getRow(normalized);
			if (!row || row.entry_type !== "file") {
				throw new KernelError("ENOENT", `no such file: ${p}`);
			}

			const existing = row.content ? new Uint8Array(row.content) : new Uint8Array(0);
			let newContent: Buffer;

			if (length === 0) {
				newContent = Buffer.alloc(0);
			} else if (length <= existing.length) {
				newContent = Buffer.from(existing.slice(0, length));
			} else {
				// Extend with zeros.
				const extended = new Uint8Array(length);
				extended.set(existing);
				newContent = Buffer.from(extended);
			}

			const ts = Date.now();
			db.prepare("UPDATE fs_entries SET content = ?, size = ?, mtime_ms = ? WHERE path = ?")
				.run(newContent, newContent.length, ts, normalized);
		},

		async pread(
			p: string,
			offset: number,
			length: number,
		): Promise<Uint8Array> {
			const resolved = resolveSymlinks(p);
			const row = getRow(resolved);
			if (!row || row.entry_type !== "file") {
				throw new KernelError("ENOENT", `no such file: ${p}`);
			}
			const content = row.content ? new Uint8Array(row.content) : new Uint8Array(0);
			return content.slice(offset, offset + length);
		},
	};

	return backend;
}

/**
 * Postgres filesystem backend.
 *
 * Stores all files, directories, and symlinks in a Postgres database.
 * Supports the full VirtualFileSystem interface including symlinks,
 * hard links, permissions, and timestamps.
 */

import pg from "pg";
import * as path from "node:path";
import {
	KernelError,
	type VirtualDirEntry,
	type VirtualFileSystem,
	type VirtualStat,
} from "@secure-exec/core";

export interface PostgresFsOptions {
	/** Postgres connection string (e.g. "postgresql://user:pass@host:port/db"). */
	connectionString?: string;
	/** Existing pg.Pool instance to use instead of creating one. */
	pool?: pg.Pool;
	/** Table name prefix for isolation (default: "fs"). Must match /^[a-zA-Z_][a-zA-Z0-9_]*$/. */
	prefix?: string;
	/** Postgres schema to use (default: "public"). Must match /^[a-zA-Z_][a-zA-Z0-9_]*$/. */
	schema?: string;
}

interface FsRow {
	path: string;
	entry_type: "file" | "directory" | "symlink";
	content: Buffer | null;
	mode: number;
	uid: number;
	gid: number;
	size: string; // BIGINT comes as string from pg
	nlink: number;
	symlink_target: string | null;
	atime_ms: number;
	mtime_ms: number;
	ctime_ms: number;
	birthtime_ms: number;
}

const MAX_SYMLINK_DEPTH = 40;
const IDENTIFIER_RE = /^[a-zA-Z_][a-zA-Z0-9_]*$/;

function normalizePath(p: string): string {
	const normalized = path.posix.normalize(p);
	const withSlash = normalized.startsWith("/") ? normalized : `/${normalized}`;
	return withSlash === "/" ? "/" : withSlash.replace(/\/+$/, "");
}

function parentPath(p: string): string {
	const parent = path.posix.dirname(p);
	return parent === "." ? "/" : parent;
}

/**
 * Escape LIKE metacharacters (%, _, \) for use in Postgres LIKE patterns.
 */
function escapeLike(s: string): string {
	return s.replace(/[%_\\]/g, (ch) => `\\${ch}`);
}

/**
 * Quote a Postgres identifier with double quotes, escaping embedded double quotes.
 */
function quoteIdent(s: string): string {
	return `"${s.replace(/"/g, '""')}"`;
}

function rowToStat(row: FsRow): VirtualStat {
	return {
		mode: row.mode,
		size: parseInt(row.size, 10),
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

/**
 * Create a VirtualFileSystem backed by Postgres.
 */
export async function createPostgresBackend(
	options: PostgresFsOptions,
): Promise<VirtualFileSystem> {
	const pool =
		options.pool ?? new pg.Pool({ connectionString: options.connectionString });
	const schema = options.schema ?? "public";
	const prefix = options.prefix ?? "fs";

	// Validate identifiers to prevent SQL injection.
	if (!IDENTIFIER_RE.test(schema)) {
		throw new Error(`Invalid schema name: ${schema}. Must match ${IDENTIFIER_RE}`);
	}
	if (!IDENTIFIER_RE.test(prefix)) {
		throw new Error(`Invalid prefix: ${prefix}. Must match ${IDENTIFIER_RE}`);
	}

	const quotedSchema = quoteIdent(schema);
	const table = `${quotedSchema}.${quoteIdent(`${prefix}_entries`)}`;

	// Create schema and table.
	await pool.query(`CREATE SCHEMA IF NOT EXISTS ${quotedSchema}`);
	await pool.query(`
		CREATE TABLE IF NOT EXISTS ${table} (
			path TEXT PRIMARY KEY,
			entry_type TEXT NOT NULL CHECK(entry_type IN ('file', 'directory', 'symlink')),
			content BYTEA,
			mode INTEGER NOT NULL DEFAULT 33188,
			uid INTEGER NOT NULL DEFAULT 0,
			gid INTEGER NOT NULL DEFAULT 0,
			size BIGINT NOT NULL DEFAULT 0,
			nlink INTEGER NOT NULL DEFAULT 1,
			symlink_target TEXT,
			atime_ms DOUBLE PRECISION NOT NULL,
			mtime_ms DOUBLE PRECISION NOT NULL,
			ctime_ms DOUBLE PRECISION NOT NULL,
			birthtime_ms DOUBLE PRECISION NOT NULL
		)
	`);

	// Create root directory if it doesn't exist.
	const now = Date.now();
	await pool.query(
		`INSERT INTO ${table} (path, entry_type, mode, size, nlink, atime_ms, mtime_ms, ctime_ms, birthtime_ms)
		 VALUES ('/', 'directory', 16877, 0, 1, $1, $2, $3, $4)
		 ON CONFLICT (path) DO NOTHING`,
		[now, now, now, now],
	);

	// Helper that runs a query on a specific client or the pool.
	type Queryable = pg.Pool | pg.PoolClient;

	async function getRow(p: string, q: Queryable = pool): Promise<FsRow | undefined> {
		const { rows } = await q.query(
			`SELECT * FROM ${table} WHERE path = $1`,
			[normalizePath(p)],
		);
		return rows[0] as FsRow | undefined;
	}

	/**
	 * Resolve a single path component that may be a symlink.
	 */
	async function resolveOneSymlink(p: string, depth: number, q: Queryable = pool): Promise<string> {
		if (depth > MAX_SYMLINK_DEPTH) {
			throw new KernelError("EINVAL", `too many symlinks: ${p}`);
		}
		const row = await getRow(p, q);
		if (!row) return p;
		if (row.entry_type === "symlink" && row.symlink_target) {
			const target = row.symlink_target.startsWith("/")
				? row.symlink_target
				: path.posix.resolve(parentPath(p), row.symlink_target);
			return resolveSymlinks(target, depth + 1, q);
		}
		return p;
	}

	/**
	 * Resolve symlinks in all path components, not just the final one.
	 */
	async function resolveSymlinks(p: string, depth = 0, q: Queryable = pool): Promise<string> {
		if (depth > MAX_SYMLINK_DEPTH) {
			throw new KernelError("EINVAL", `too many symlinks: ${p}`);
		}
		const normalized = normalizePath(p);
		const parts = normalized.split("/").filter(Boolean);
		let current = "";
		for (let i = 0; i < parts.length; i++) {
			current += `/${parts[i]}`;
			current = await resolveOneSymlink(current, depth, q);
		}
		return current || "/";
	}

	/**
	 * Resolve symlinks in all intermediate path components but NOT the final one.
	 * Used by removeFile so it deletes the symlink entry itself, not its target.
	 */
	async function resolveIntermediateSymlinks(p: string): Promise<string> {
		const normalized = normalizePath(p);
		const parent = parentPath(normalized);
		const name = path.posix.basename(normalized);
		if (parent === "/" && name === "") return "/";
		const resolvedParent = await resolveSymlinks(parent);
		return resolvedParent === "/" ? `/${name}` : `${resolvedParent}/${name}`;
	}

	async function ensureParentDirs(p: string, q: Queryable = pool): Promise<void> {
		const parts = normalizePath(p).split("/").filter(Boolean);
		let current = "";
		for (let i = 0; i < parts.length - 1; i++) {
			current += `/${parts[i]}`;
			const ts = Date.now();
			await q.query(
				`INSERT INTO ${table} (path, entry_type, mode, size, nlink, atime_ms, mtime_ms, ctime_ms, birthtime_ms)
				 VALUES ($1, 'directory', $2, 0, 1, $3, $4, $5, $6)
				 ON CONFLICT (path) DO NOTHING`,
				[current, 0o40755, ts, ts, ts, ts],
			);
		}
	}

	const backend: VirtualFileSystem = {
		async readFile(p: string): Promise<Uint8Array> {
			const resolved = await resolveSymlinks(p);
			const row = await getRow(resolved);
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
			const resolved = await resolveSymlinks(p);
			const normalized = normalizePath(resolved);
			const prefix_ = normalized === "/" ? "/" : `${normalized}/`;

			const { rows } = await pool.query(
				`SELECT path FROM ${table} WHERE path LIKE $1 ESCAPE E'\\\\' AND path != $2`,
				[`${escapeLike(prefix_)}%`, normalized],
			);

			const names: string[] = [];
			for (const row of rows as { path: string }[]) {
				const relative = row.path.slice(prefix_.length);
				if (relative && !relative.includes("/")) {
					names.push(relative);
				}
			}
			return names;
		},

		async readDirWithTypes(p: string): Promise<VirtualDirEntry[]> {
			const resolved = await resolveSymlinks(p);
			const normalized = normalizePath(resolved);
			const prefix_ = normalized === "/" ? "/" : `${normalized}/`;

			const { rows } = await pool.query(
				`SELECT path, entry_type FROM ${table} WHERE path LIKE $1 ESCAPE E'\\\\' AND path != $2`,
				[`${escapeLike(prefix_)}%`, normalized],
			);

			const entries: VirtualDirEntry[] = [];
			for (const row of rows as { path: string; entry_type: string }[]) {
				const relative = row.path.slice(prefix_.length);
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
			const resolved = await resolveSymlinks(p);
			const normalized = normalizePath(resolved);
			await ensureParentDirs(normalized);

			const body =
				typeof content === "string"
					? Buffer.from(content, "utf-8")
					: Buffer.from(content);
			const ts = Date.now();

			await pool.query(
				`INSERT INTO ${table} (path, entry_type, content, mode, uid, gid, size, nlink, symlink_target, atime_ms, mtime_ms, ctime_ms, birthtime_ms)
				 VALUES ($1, 'file', $2, 33188, 0, 0, $3, 1, NULL, $4, $5, $6, $7)
				 ON CONFLICT (path) DO UPDATE SET
					content = EXCLUDED.content,
					size = EXCLUDED.size,
					atime_ms = EXCLUDED.atime_ms,
					mtime_ms = EXCLUDED.mtime_ms,
					ctime_ms = EXCLUDED.ctime_ms`,
				[normalized, body, body.length, ts, ts, ts, ts],
			);
		},

		async createDir(p: string): Promise<void> {
			return backend.mkdir(p);
		},

		async mkdir(p: string, options?: { recursive?: boolean }): Promise<void> {
			const normalized = normalizePath(p);

			if (options?.recursive) {
				await ensureParentDirs(normalized);
			} else {
				// Without recursive, verify the parent exists.
				const parent = parentPath(normalized);
				if (parent !== "/") {
					const parentRow = await getRow(parent);
					if (!parentRow) {
						throw new KernelError("ENOENT", `parent directory does not exist: ${parent}`);
					}
				}
			}

			const existing = await getRow(normalized);
			if (existing) {
				if (existing.entry_type === "directory") return;
				throw new KernelError("EEXIST", `path already exists: ${p}`);
			}

			const ts = Date.now();
			await pool.query(
				`INSERT INTO ${table} (path, entry_type, mode, size, nlink, atime_ms, mtime_ms, ctime_ms, birthtime_ms)
				 VALUES ($1, 'directory', $2, 0, 1, $3, $4, $5, $6)
				 ON CONFLICT (path) DO NOTHING`,
				[normalized, 0o40755, ts, ts, ts, ts],
			);
		},

		async exists(p: string): Promise<boolean> {
			try {
				const resolved = await resolveSymlinks(p);
				const { rows } = await pool.query(
					`SELECT 1 FROM ${table} WHERE path = $1 LIMIT 1`,
					[resolved],
				);
				return rows.length > 0;
			} catch {
				return false;
			}
		},

		async stat(p: string): Promise<VirtualStat> {
			const resolved = await resolveSymlinks(p);
			const row = await getRow(resolved);
			if (!row) {
				throw new KernelError("ENOENT", `no such file or directory: ${p}`);
			}
			return rowToStat(row);
		},

		async lstat(p: string): Promise<VirtualStat> {
			const normalized = normalizePath(p);
			const row = await getRow(normalized);
			if (!row) {
				throw new KernelError("ENOENT", `no such file or directory: ${p}`);
			}
			return rowToStat(row);
		},

		async removeFile(p: string): Promise<void> {
			// Resolve intermediate symlinks but NOT the final component.
			const normalized = await resolveIntermediateSymlinks(p);
			const row = await getRow(normalized);
			if (!row) {
				throw new KernelError("ENOENT", `no such file: ${p}`);
			}
			if (row.entry_type === "directory") {
				throw new KernelError("EISDIR", `is a directory: ${p}`);
			}
			await pool.query(`DELETE FROM ${table} WHERE path = $1`, [normalized]);
		},

		async removeDir(p: string): Promise<void> {
			const normalized = normalizePath(p);
			const row = await getRow(normalized);
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

			await pool.query(`DELETE FROM ${table} WHERE path = $1`, [normalized]);
		},

		async rename(oldPath: string, newPath: string): Promise<void> {
			const oldNorm = normalizePath(oldPath);
			const newNorm = normalizePath(newPath);

			const client = await pool.connect();
			try {
				await client.query("BEGIN");

				const row = await getRow(oldNorm, client);
				if (!row) {
					throw new KernelError("ENOENT", `no such file or directory: ${oldPath}`);
				}

				await ensureParentDirs(newNorm, client);

				const ts = Date.now();

				// Check if destination exists.
				const destRow = await getRow(newNorm, client);
				if (destRow && destRow.entry_type === "directory") {
					// Check if the destination directory is non-empty.
					const destPrefix = newNorm === "/" ? "/" : `${newNorm}/`;
					const { rows: destChildren } = await client.query(
						`SELECT 1 FROM ${table} WHERE path LIKE $1 ESCAPE E'\\\\' AND path != $2 LIMIT 1`,
						[`${escapeLike(destPrefix)}%`, newNorm],
					);
					if (destChildren.length > 0) {
						throw new KernelError("ENOTEMPTY", `destination directory not empty: ${newPath}`);
					}
				}

				if (row.entry_type === "directory") {
					// Move all children by updating their path prefix.
					const oldPrefix = oldNorm === "/" ? "/" : `${oldNorm}/`;
					const newPrefix = newNorm === "/" ? "/" : `${newNorm}/`;

					const { rows: children } = await client.query(
						`SELECT path FROM ${table} WHERE path LIKE $1 ESCAPE E'\\\\'`,
						[`${escapeLike(oldPrefix)}%`],
					);

					for (const child of children as { path: string }[]) {
						const newChildPath = newPrefix + child.path.slice(oldPrefix.length);
						await client.query(
							`UPDATE ${table} SET path = $1, mtime_ms = $2 WHERE path = $3`,
							[newChildPath, ts, child.path],
						);
					}
				}

				// Delete any existing entry at the destination.
				await client.query(`DELETE FROM ${table} WHERE path = $1`, [newNorm]);
				// Move the entry itself.
				await client.query(
					`UPDATE ${table} SET path = $1, mtime_ms = $2 WHERE path = $3`,
					[newNorm, ts, oldNorm],
				);

				await client.query("COMMIT");
			} catch (err) {
				await client.query("ROLLBACK");
				throw err;
			} finally {
				client.release();
			}
		},

		async realpath(p: string): Promise<string> {
			return resolveSymlinks(p);
		},

		async symlink(target: string, linkPath: string): Promise<void> {
			const normalized = normalizePath(linkPath);
			await ensureParentDirs(normalized);

			// Check if path already exists and throw EEXIST.
			const existing = await getRow(normalized);
			if (existing) {
				throw new KernelError("EEXIST", `path already exists: ${linkPath}`);
			}

			const ts = Date.now();
			await pool.query(
				`INSERT INTO ${table} (path, entry_type, mode, size, nlink, symlink_target, atime_ms, mtime_ms, ctime_ms, birthtime_ms)
				 VALUES ($1, 'symlink', $2, $3, 1, $4, $5, $6, $7, $8)`,
				[normalized, 0o120777, target.length, target, ts, ts, ts, ts],
			);
		},

		async readlink(p: string): Promise<string> {
			const normalized = normalizePath(p);
			const row = await getRow(normalized);
			if (!row || row.entry_type !== "symlink" || !row.symlink_target) {
				throw new KernelError("EINVAL", `not a symlink: ${p}`);
			}
			return row.symlink_target;
		},

		async link(existingPath: string, newPath: string): Promise<void> {
			const resolved = await resolveSymlinks(existingPath);
			const row = await getRow(resolved);
			if (!row) {
				throw new KernelError("ENOENT", `no such file: ${existingPath}`);
			}
			if (row.entry_type === "directory") {
				throw new KernelError("EPERM", `cannot hard-link a directory: ${existingPath}`);
			}

			const newNorm = normalizePath(newPath);

			// Wrap in a transaction so the insert and nlink update are atomic.
			const client = await pool.connect();
			try {
				await client.query("BEGIN");

				await ensureParentDirs(newNorm, client);

				const ts = Date.now();
				await client.query(
					`INSERT INTO ${table} (path, entry_type, content, mode, uid, gid, size, nlink, symlink_target, atime_ms, mtime_ms, ctime_ms, birthtime_ms)
					 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)`,
					[
						newNorm, row.entry_type, row.content, row.mode, row.uid, row.gid,
						parseInt(row.size, 10), row.nlink + 1, row.symlink_target,
						ts, ts, row.ctime_ms, row.birthtime_ms,
					],
				);

				// Increment nlink on the original.
				await client.query(
					`UPDATE ${table} SET nlink = nlink + 1 WHERE path = $1`,
					[resolved],
				);

				await client.query("COMMIT");
			} catch (err) {
				await client.query("ROLLBACK");
				throw err;
			} finally {
				client.release();
			}
		},

		async chmod(p: string, mode: number): Promise<void> {
			const resolved = await resolveSymlinks(p);
			const normalized = normalizePath(resolved);
			const row = await getRow(normalized);
			if (!row) {
				throw new KernelError("ENOENT", `no such file or directory: ${p}`);
			}
			// Preserve file type bits (upper bits) and only change permission bits (lower 12 bits).
			const typeBits = row.mode & 0o170000;
			const newMode = typeBits | (mode & 0o7777);
			const ts = Date.now();
			await pool.query(
				`UPDATE ${table} SET mode = $1, ctime_ms = $2 WHERE path = $3`,
				[newMode, ts, normalized],
			);
		},

		async chown(p: string, uid: number, gid: number): Promise<void> {
			const resolved = await resolveSymlinks(p);
			const normalized = normalizePath(resolved);
			const row = await getRow(normalized);
			if (!row) {
				throw new KernelError("ENOENT", `no such file or directory: ${p}`);
			}
			const ts = Date.now();
			await pool.query(
				`UPDATE ${table} SET uid = $1, gid = $2, ctime_ms = $3 WHERE path = $4`,
				[uid, gid, ts, normalized],
			);
		},

		async utimes(p: string, atime: number, mtime: number): Promise<void> {
			const resolved = await resolveSymlinks(p);
			const normalized = normalizePath(resolved);
			const row = await getRow(normalized);
			if (!row) {
				throw new KernelError("ENOENT", `no such file or directory: ${p}`);
			}
			await pool.query(
				`UPDATE ${table} SET atime_ms = $1, mtime_ms = $2 WHERE path = $3`,
				[atime, mtime, normalized],
			);
		},

		async truncate(p: string, length: number): Promise<void> {
			const resolved = await resolveSymlinks(p);
			const normalized = normalizePath(resolved);
			const row = await getRow(normalized);
			if (!row || row.entry_type !== "file") {
				throw new KernelError("ENOENT", `no such file: ${p}`);
			}

			const ts = Date.now();
			const currentSize = parseInt(row.size, 10);

			if (length === 0) {
				await pool.query(
					`UPDATE ${table} SET content = E'\\\\x', size = 0, mtime_ms = $1 WHERE path = $2`,
					[ts, normalized],
				);
			} else if (length <= currentSize) {
				// Use SQL substring to truncate without reading into JS.
				await pool.query(
					`UPDATE ${table} SET content = substring(content FROM 1 FOR $1), size = $1, mtime_ms = $2 WHERE path = $3`,
					[length, ts, normalized],
				);
			} else {
				// Extend with null bytes using SQL overlay/concatenation.
				const padLen = length - currentSize;
				await pool.query(
					`UPDATE ${table} SET content = COALESCE(content, E'\\\\x') || decode(repeat('00', $1::int), 'hex'), size = $2, mtime_ms = $3 WHERE path = $4`,
					[padLen, length, ts, normalized],
				);
			}
		},

		async pread(
			p: string,
			offset: number,
			length: number,
		): Promise<Uint8Array> {
			const resolved = await resolveSymlinks(p);
			// Use SQL substring to read a slice without fetching the entire file.
			const { rows } = await pool.query(
				`SELECT substring(content FROM $1 FOR $2) AS slice, entry_type FROM ${table} WHERE path = $3`,
				[offset + 1, length, resolved],
			);
			const row = rows[0] as { slice: Buffer | null; entry_type: string } | undefined;
			if (!row || row.entry_type !== "file") {
				throw new KernelError("ENOENT", `no such file: ${p}`);
			}
			if (!row.slice) return new Uint8Array(0);
			return new Uint8Array(row.slice);
		},
	};

	return backend;
}

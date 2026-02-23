import { getRequireFn } from "@/utils/node";

type SqliteRuntimeKind = "bun" | "node" | "better-sqlite3";
type SqliteDatabaseCtor = new (path: string) => SqliteRawDatabase;

interface SqliteStatement {
	run(...params: unknown[]): unknown;
	get<T = Record<string, unknown>>(...params: unknown[]): T | undefined;
	all<T = Record<string, unknown>>(...params: unknown[]): T[];
}

interface SqliteRawDatabase {
	exec(sql: string): unknown;
	close(): unknown;
	prepare?(sql: string): SqliteStatement;
	query?(sql: string): SqliteStatement;
}

export interface SqliteRuntimeDatabase {
	exec(sql: string): void;
	run(sql: string, params?: readonly unknown[]): void;
	get<T = Record<string, unknown>>(
		sql: string,
		params?: readonly unknown[],
	): T | undefined;
	all<T = Record<string, unknown>>(
		sql: string,
		params?: readonly unknown[],
	): T[];
	close(): void;
}

export interface SqliteRuntime {
	kind: SqliteRuntimeKind;
	open(path: string): SqliteRuntimeDatabase;
}

function normalizeParams(params: readonly unknown[] | undefined): unknown[] {
	if (!params || params.length === 0) {
		return [];
	}

	return params.map((value) => {
		if (value instanceof Uint8Array) {
			return Buffer.from(value);
		}
		return value;
	});
}

function createPreparedDatabaseAdapter(
	rawDb: SqliteRawDatabase,
	prepare: (sql: string) => SqliteStatement,
): SqliteRuntimeDatabase {
	return {
		exec: (sql) => {
			rawDb.exec(sql);
		},
		run: (sql, params) => {
			const stmt = prepare(sql);
			stmt.run(...normalizeParams(params));
		},
		get: <T = Record<string, unknown>>(
			sql: string,
			params?: readonly unknown[],
		) => {
			const stmt = prepare(sql);
			return stmt.get<T>(...normalizeParams(params));
		},
		all: <T = Record<string, unknown>>(
			sql: string,
			params?: readonly unknown[],
		) => {
			const stmt = prepare(sql);
			return stmt.all<T>(...normalizeParams(params));
		},
		close: () => {
			rawDb.close();
		},
	};
}

function configureSqliteRuntimeDatabase(
	rawDb: SqliteRawDatabase,
	path: string,
): void {
	// Wait briefly when the database file is still being released by another
	// process during restarts to reduce transient "database is locked" failures.
	rawDb.exec("PRAGMA busy_timeout = 5000");

	// WAL improves concurrent read/write behavior for file-backed databases.
	if (path !== ":memory:") {
		rawDb.exec("PRAGMA journal_mode = WAL");
	}
}

export function loadSqliteRuntime(): SqliteRuntime {
	const requireFn = getRequireFn();
	const loadErrors: string[] = [];

	try {
		const bunSqlite = requireFn(/* webpackIgnore: true */ "bun:sqlite") as {
			Database?: SqliteDatabaseCtor;
		};
		const BunDatabase = bunSqlite.Database;
		if (BunDatabase) {
			return {
				kind: "bun",
				open: (path) => {
					const rawDb = new BunDatabase(path);
					configureSqliteRuntimeDatabase(rawDb, path);
					const query = rawDb.query?.bind(rawDb);
					if (!query) throw new Error("bun:sqlite database missing query method");
					return createPreparedDatabaseAdapter(rawDb, query);
				},
			};
		}
	} catch (error) {
		loadErrors.push(`bun:sqlite unavailable: ${String(error)}`);
	}

	try {
		const nodeSqlite = requireFn(/* webpackIgnore: true */ "node:sqlite") as {
			DatabaseSync?: SqliteDatabaseCtor;
		};
		const NodeDatabaseSync = nodeSqlite.DatabaseSync;
		if (NodeDatabaseSync) {
			return {
				kind: "node",
				open: (path) => {
					const rawDb = new NodeDatabaseSync(path);
					configureSqliteRuntimeDatabase(rawDb, path);
					const prepare = rawDb.prepare?.bind(rawDb);
					if (!prepare) {
						throw new Error("node:sqlite DatabaseSync missing prepare method");
					}
					return createPreparedDatabaseAdapter(rawDb, prepare);
				},
			};
		}
	} catch (error) {
		loadErrors.push(`node:sqlite unavailable: ${String(error)}`);
	}

	try {
		const betterSqlite3Module = requireFn(
			/* webpackIgnore: true */ "better-sqlite3",
		) as SqliteDatabaseCtor | { default?: SqliteDatabaseCtor };
		const BetterSqlite3 =
			typeof betterSqlite3Module === "function"
				? betterSqlite3Module
				: betterSqlite3Module.default;
		if (BetterSqlite3) {
			return {
				kind: "better-sqlite3",
				open: (path) => {
					const rawDb = new BetterSqlite3(path);
					configureSqliteRuntimeDatabase(rawDb, path);
					const prepare = rawDb.prepare?.bind(rawDb);
					if (!prepare) {
						throw new Error("better-sqlite3 database missing prepare method");
					}
					return createPreparedDatabaseAdapter(rawDb, prepare);
				},
			};
		}
	} catch (error) {
		loadErrors.push(`better-sqlite3 unavailable: ${String(error)}`);
		throw new Error(
			`No SQLite runtime available. Tried bun:sqlite, node:sqlite, and better-sqlite3. Install better-sqlite3 (e.g. "pnpm add better-sqlite3") if native runtimes are unavailable.\n${loadErrors.join("\n")}`,
		);
	}

	throw new Error(
		`No SQLite runtime available. Tried bun:sqlite, node:sqlite, and better-sqlite3.\n${loadErrors.join("\n")}`,
	);
}

export function computePrefixUpperBound(
	prefix: Uint8Array,
): Uint8Array | undefined {
	if (prefix.length === 0) {
		return undefined;
	}

	const upperBound = new Uint8Array(prefix);
	for (let i = upperBound.length - 1; i >= 0; i--) {
		if (upperBound[i] !== 0xff) {
			upperBound[i] += 1;
			return upperBound.slice(0, i + 1);
		}
	}
	return undefined;
}

export function ensureUint8Array(
	value: unknown,
	fieldName: string,
): Uint8Array {
	if (value instanceof Uint8Array) {
		return value;
	}
	if (value instanceof ArrayBuffer) {
		return new Uint8Array(value);
	}
	if (ArrayBuffer.isView(value)) {
		return new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
	}
	throw new Error(`SQLite row field "${fieldName}" is not binary data`);
}

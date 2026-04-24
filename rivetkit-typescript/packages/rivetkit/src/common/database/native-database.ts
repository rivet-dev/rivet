import { decodeBridgeRivetError } from "@/actor/errors";
import { AsyncMutex } from "./shared";
import type { SqliteBindings, SqliteDatabase } from "./config";

interface NativeBindParam {
	kind: "null" | "int" | "float" | "text" | "blob";
	intValue?: number;
	floatValue?: number;
	textValue?: string;
	blobValue?: Buffer;
}

interface NativeExecResult {
	columns: string[];
	rows: unknown[][];
}

interface NativeQueryResult {
	columns: string[];
	rows: unknown[][];
}

interface NativeRunResult {
	changes: number;
}

export interface SqliteVfsMetrics {
	requestBuildNs: number;
	serializeNs: number;
	transportNs: number;
	stateUpdateNs: number;
	totalNs: number;
	commitCount: number;
}

export interface JsNativeDatabaseLike {
	exec(sql: string): Promise<NativeExecResult>;
	query(
		sql: string,
		params?: NativeBindParam[] | null,
	): Promise<NativeQueryResult>;
	run(
		sql: string,
		params?: NativeBindParam[] | null,
	): Promise<NativeRunResult>;
	getSqliteVfsMetrics?(): SqliteVfsMetrics | null;
	takeLastKvError?(): string | null;
	close(): Promise<void>;
}

function shouldAttachNativeKvError(message: string): boolean {
	return /i\/o error|unable to open database file/i.test(message);
}

function enrichNativeDatabaseError(
	database: JsNativeDatabaseLike,
	error: unknown,
): never {
	const bridged =
		typeof error === "string"
			? decodeBridgeRivetError(error)
			: error instanceof Error
				? decodeBridgeRivetError(error.message)
				: undefined;
	if (bridged) {
		throw bridged;
	}

	const kvError = database.takeLastKvError?.();
	if (
		error instanceof Error &&
		kvError &&
		shouldAttachNativeKvError(error.message) &&
		!error.message.includes(kvError)
	) {
		error.message = `${error.message} (native sqlite kv error: ${kvError})`;
	}
	throw error;
}

function toNativeBinding(arg: unknown): NativeBindParam {
	if (arg === null || arg === undefined) {
		return { kind: "null" };
	}
	if (typeof arg === "bigint") {
		return { kind: "int", intValue: Number(arg) };
	}
	if (typeof arg === "number") {
		if (Number.isInteger(arg)) {
			return { kind: "int", intValue: arg };
		}
		return { kind: "float", floatValue: arg };
	}
	if (typeof arg === "string") {
		return { kind: "text", textValue: arg };
	}
	if (typeof arg === "boolean") {
		return { kind: "int", intValue: arg ? 1 : 0 };
	}
	if (arg instanceof Uint8Array) {
		return { kind: "blob", blobValue: Buffer.from(arg) };
	}
	throw new Error(`unsupported bind parameter type: ${typeof arg}`);
}

function extractNamedSqliteParameters(sql: string): string[] {
	const orderedNames: string[] = [];
	const seen = new Set<string>();
	const pattern = /([:@$][A-Za-z_][A-Za-z0-9_]*)/g;
	for (const match of sql.matchAll(pattern)) {
		const name = match[1];
		if (seen.has(name)) {
			continue;
		}
		seen.add(name);
		orderedNames.push(name);
	}
	return orderedNames;
}

function getNamedSqliteBinding(
	bindings: Record<string, unknown>,
	name: string,
): unknown {
	if (name in bindings) {
		return bindings[name];
	}

	const bareName = name.slice(1);
	if (bareName in bindings) {
		return bindings[bareName];
	}

	for (const prefix of [":", "@", "$"] as const) {
		const candidate = `${prefix}${bareName}`;
		if (candidate in bindings) {
			return bindings[candidate];
		}
	}

	return undefined;
}

function toNativeBindings(
	sql: string,
	params?: SqliteBindings,
): NativeBindParam[] | null {
	if (params === undefined) {
		return null;
	}

	if (Array.isArray(params)) {
		return params.map((arg) => toNativeBinding(arg));
	}

	const orderedNames = extractNamedSqliteParameters(sql);
	if (orderedNames.length === 0) {
		return Object.values(params).map((arg) => toNativeBinding(arg));
	}

	return orderedNames.map((name) => {
		const value = getNamedSqliteBinding(params, name);
		if (value === undefined) {
			throw new Error(`missing bind parameter: ${name}`);
		}
		return toNativeBinding(value);
	});
}

export function wrapJsNativeDatabase(
	database: JsNativeDatabaseLike,
): SqliteDatabase {
	const mutex = new AsyncMutex();
	let closed = false;

	const ensureOpen = () => {
		if (closed) {
			throw new Error(
				"Database is closed. This usually means a background timer (setInterval, setTimeout) or a stray promise is still running after the actor stopped. Use c.abortSignal to clean up timers before the actor shuts down.",
			);
		}
	};

	return {
		async exec(
			sql: string,
			callback?: (row: unknown[], columns: string[]) => void,
		): Promise<void> {
			const result = await mutex.run(async () => {
				ensureOpen();
				try {
					return await database.exec(sql);
				} catch (error) {
					enrichNativeDatabaseError(database, error);
				}
			});
			if (!callback) {
				return;
			}
			for (const row of result.rows) {
				callback(row, result.columns);
			}
		},
		async run(sql: string, params?: SqliteBindings): Promise<void> {
			await mutex.run(async () => {
				ensureOpen();
				try {
					await database.run(sql, toNativeBindings(sql, params));
				} catch (error) {
					enrichNativeDatabaseError(database, error);
				}
			});
		},
		async query(sql: string, params?: SqliteBindings) {
			return await mutex.run(async () => {
				ensureOpen();
				try {
					return await database.query(sql, toNativeBindings(sql, params));
				} catch (error) {
					enrichNativeDatabaseError(database, error);
				}
			});
		},
		getSqliteVfsMetrics() {
			return database.getSqliteVfsMetrics?.() ?? null;
		},
		async close(): Promise<void> {
			await mutex.run(async () => {
				if (closed) {
					return;
				}
				closed = true;
				await database.close();
			});
		},
	};
}

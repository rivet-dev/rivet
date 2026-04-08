import type { IDatabase } from "@rivetkit/sqlite-vfs";
import type { NativeSqliteConfig } from "./config";
import {
	getNativeModule,
	disconnectKvChannelIfCurrent,
	getOrCreateKvChannel,
	toNativeBindings,
	type NativeBindParam,
	type NativeKvChannel,
	type NativeDatabase,
} from "./native-sqlite";
import { AsyncMutex } from "./shared";

function isPlainObject(value: unknown): value is Record<string, unknown> {
	return (
		!!value &&
		typeof value === "object" &&
		!Array.isArray(value) &&
		Object.getPrototypeOf(value) === Object.prototype
	);
}

function extractNamedSqliteParameters(sql: string): string[] {
	const names: string[] = [];
	const pattern = /([:@$][A-Za-z_][A-Za-z0-9_]*)/g;
	for (const match of sql.matchAll(pattern)) {
		names.push(match[1]);
	}
	return names;
}

function resolveNamedSqliteBinding(
	params: Record<string, unknown>,
	name: string,
): unknown {
	if (name in params) {
		return params[name];
	}

	const bareName = name.slice(1);
	if (bareName in params) {
		return params[bareName];
	}

	for (const prefix of [":", "@", "$"] as const) {
		const candidate = `${prefix}${bareName}`;
		if (candidate in params) {
			return params[candidate];
		}
	}

	return undefined;
}

function normalizeNativeBindings(
	sql: string,
	params?: unknown,
): NativeBindParam[] {
	if (params === undefined || params === null) {
		return [];
	}

	if (Array.isArray(params)) {
		return toNativeBindings(params);
	}

	if (isPlainObject(params)) {
		const names = extractNamedSqliteParameters(sql);
		if (names.length === 0) {
			throw new Error(
				"native SQLite adapter only supports named parameter objects when the SQL statement uses named placeholders",
			);
		}

		return toNativeBindings(
			names.map((name) => {
				const value = resolveNamedSqliteBinding(params, name);
				if (value === undefined) {
					throw new Error(`missing bind parameter: ${name}`);
				}
				return value;
			}),
		);
	}

	throw new Error(
		"native SQLite adapter only supports positional parameter arrays or named parameter objects",
	);
}

function isStaleKvChannelError(error: unknown): boolean {
	const message =
		error instanceof Error ? error.message : String(error);
	return /kv channel (?:connection closed|shut down)/i.test(message);
}

async function clearKvChannelForConfig(
	channel: NativeKvChannel,
	config?: NativeSqliteConfig,
): Promise<void> {
	try {
		await disconnectKvChannelIfCurrent(channel, config);
	} catch {
		// Ignore disconnect errors. The cache entry is about to be replaced.
	}
}

async function openNativeDatabaseHandle(
	actorId: string,
	config?: NativeSqliteConfig,
): Promise<{
	nativeDb: NativeDatabase;
	channel: NativeKvChannel;
}> {
	const mod = getNativeModule();
	const maxAttempts = 3;

	for (let attempt = 0; attempt < maxAttempts; attempt++) {
		const channel = getOrCreateKvChannel(config);
		try {
			return {
				nativeDb: await mod.openDatabase(channel, actorId),
				channel,
			};
		} catch (error) {
			if (
				!isStaleKvChannelError(error) ||
				attempt === maxAttempts - 1
			) {
				throw error;
			}

			await clearKvChannelForConfig(channel, config);
		}
	}

	throw new Error("unreachable: native database open exhausted retries");
}

class NativeSqliteDatabase implements IDatabase {
	#module = getNativeModule();
	#nativeDb: NativeDatabase;
	#channel: NativeKvChannel;
	#config?: NativeSqliteConfig;
	#recoveryMutex = new AsyncMutex();
	readonly fileName: string;

	constructor(
		nativeDb: NativeDatabase,
		channel: NativeKvChannel,
		fileName: string,
		config?: NativeSqliteConfig,
	) {
		this.#nativeDb = nativeDb;
		this.#channel = channel;
		this.fileName = fileName;
		this.#config = config;
	}

	async #recoverFromStaleKvChannel(error: unknown): Promise<boolean> {
		if (!isStaleKvChannelError(error)) {
			return false;
		}

		await this.#recoveryMutex.run(async () => {
			await clearKvChannelForConfig(this.#channel, this.#config);
			const reopened = await openNativeDatabaseHandle(
				this.fileName,
				this.#config,
			);
			this.#nativeDb = reopened.nativeDb;
			this.#channel = reopened.channel;
		});

		return true;
	}

	async #runWithReconnect<T>(
		operation: (nativeDb: NativeDatabase) => Promise<T>,
	): Promise<T> {
		try {
			return await operation(this.#nativeDb);
		} catch (error) {
			const recovered = await this.#recoverFromStaleKvChannel(error);
			if (!recovered) {
				throw error;
			}

			return await operation(this.#nativeDb);
		}
	}

	async exec(
		sql: string,
		callback?: (row: unknown[], columns: string[]) => void,
	): Promise<void> {
		const result = await this.#runWithReconnect((nativeDb) => {
			return this.#module.exec(nativeDb, sql);
		});
		if (!callback) {
			return;
		}
		for (const row of result.rows) {
			callback(row, result.columns);
		}
	}

	async run(sql: string, params?: unknown): Promise<void> {
		const bindings = normalizeNativeBindings(sql, params);
		await this.#runWithReconnect((nativeDb) => {
			return this.#module.execute(nativeDb, sql, bindings);
		});
	}

	async query(
		sql: string,
		params?: unknown,
	): Promise<{ rows: unknown[][]; columns: string[] }> {
		const bindings = normalizeNativeBindings(sql, params);
		return await this.#runWithReconnect((nativeDb) => {
			return this.#module.query(nativeDb, sql, bindings);
		});
	}

	async close(): Promise<void> {
		try {
			await this.#module.closeDatabase(this.#nativeDb);
		} catch (error) {
			await this.#recoverFromStaleKvChannel(error);
			if (!isStaleKvChannelError(error)) {
				throw error;
			}
		}
	}
}

export async function openNativeDatabase(
	actorId: string,
	config?: NativeSqliteConfig,
): Promise<IDatabase> {
	const { nativeDb, channel } = await openNativeDatabaseHandle(
		actorId,
		config,
	);

	return new NativeSqliteDatabase(nativeDb, channel, actorId, config);
}

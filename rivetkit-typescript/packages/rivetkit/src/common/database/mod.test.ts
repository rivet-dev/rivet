import { describe, expect, test } from "vitest";
import type {
	DatabaseProviderContext,
	SqliteBindings,
	SqliteDatabase,
	SqliteExecuteResult,
} from "./config";
import { db } from "./mod";

class FakeSqliteDatabase implements SqliteDatabase {
	writeModeDepth = 0;
	executeCalls: {
		sql: string;
		params?: SqliteBindings;
		writeMode: boolean;
	}[] = [];

	async exec(): Promise<void> {}

	async execute(
		sql: string,
		params?: SqliteBindings,
	): Promise<SqliteExecuteResult> {
		this.executeCalls.push({
			sql,
			params,
			writeMode: this.writeModeDepth > 0,
		});
		return {
			columns: [],
			rows: [],
			changes: 0,
			lastInsertRowId: null,
		};
	}

	async run(sql: string, params?: SqliteBindings): Promise<void> {
		await this.execute(sql, params);
	}

	async query(sql: string, params?: SqliteBindings) {
		const { columns, rows } = await this.execute(sql, params);
		return { columns, rows };
	}

	async writeMode<T>(callback: () => Promise<T>): Promise<T> {
		this.writeModeDepth++;
		try {
			return await callback();
		} finally {
			this.writeModeDepth--;
		}
	}

	async close(): Promise<void> {}
}

function testProviderContext(
	database: SqliteDatabase,
): DatabaseProviderContext {
	return {
		actorId: "actor-a",
		kv: {
			batchPut: async () => {},
			batchGet: async (keys) => keys.map(() => null),
			batchDelete: async () => {},
			deleteRange: async () => {},
		},
		nativeDatabaseProvider: {
			open: async () => database,
		},
	};
}

describe("db", () => {
	test("runs onMigrate through sqlite write mode", async () => {
		const nativeDb = new FakeSqliteDatabase();
		const provider = db({
			onMigrate: async (client) => {
				await client.execute(
					"CREATE TABLE items(id INTEGER PRIMARY KEY, value TEXT)",
				);
				await client.execute("SELECT COUNT(*) AS count FROM items");
			},
		});
		const client = await provider.createClient(
			testProviderContext(nativeDb),
		);

		await provider.onMigrate(client);

		expect(nativeDb.executeCalls).toEqual([
			{
				sql: "CREATE TABLE items(id INTEGER PRIMARY KEY, value TEXT)",
				params: undefined,
				writeMode: true,
			},
			{
				sql: "SELECT COUNT(*) AS count FROM items",
				params: undefined,
				writeMode: true,
			},
		]);
	});
});

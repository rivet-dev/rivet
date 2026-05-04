import { describe, expect, test } from "vitest";
import type {
	DatabaseProviderContext,
	SqliteBindings,
	SqliteDatabase,
	SqliteExecuteResult,
} from "./config";
import { db } from "./mod";

class FakeSqliteDatabase implements SqliteDatabase {
	executeCalls: {
		sql: string;
		params?: SqliteBindings;
	}[] = [];

	async exec(): Promise<void> {}

	async execute(
		sql: string,
		params?: SqliteBindings,
	): Promise<SqliteExecuteResult> {
		this.executeCalls.push({
			sql,
			params,
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
	test("runs onMigrate inside a sqlite savepoint", async () => {
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
				sql: "SAVEPOINT __rivet_on_migrate",
				params: undefined,
			},
			{
				sql: "CREATE TABLE items(id INTEGER PRIMARY KEY, value TEXT)",
				params: undefined,
			},
			{
				sql: "SELECT COUNT(*) AS count FROM items",
				params: undefined,
			},
			{
				sql: "RELEASE SAVEPOINT __rivet_on_migrate",
				params: undefined,
			},
		]);
	});

	test("rolls back the migration savepoint when onMigrate fails", async () => {
		const nativeDb = new FakeSqliteDatabase();
		const provider = db({
			onMigrate: async (client) => {
				await client.execute(
					"CREATE TABLE items(id INTEGER PRIMARY KEY, value TEXT)",
				);
				throw new Error("migration failed");
			},
		});
		const client = await provider.createClient(
			testProviderContext(nativeDb),
		);

		await expect(provider.onMigrate(client)).rejects.toThrow(
			"migration failed",
		);

		expect(nativeDb.executeCalls).toEqual([
			{
				sql: "SAVEPOINT __rivet_on_migrate",
				params: undefined,
			},
			{
				sql: "CREATE TABLE items(id INTEGER PRIMARY KEY, value TEXT)",
				params: undefined,
			},
			{
				sql: "ROLLBACK TO SAVEPOINT __rivet_on_migrate",
				params: undefined,
			},
			{
				sql: "RELEASE SAVEPOINT __rivet_on_migrate",
				params: undefined,
			},
		]);
	});
});

import { DatabaseSync } from "node:sqlite";
import { afterEach, describe, expect, test } from "vitest";
import {
	DEFAULT_MIGRATION_TABLE_NAME,
	migrations,
	type MigrationDatabase,
} from "./migrations";

const databases: DatabaseSync[] = [];

afterEach(() => {
	for (const database of databases.splice(0)) database.close();
});

function createDatabase(): { raw: DatabaseSync; db: MigrationDatabase } {
	const raw = new DatabaseSync(":memory:");
	databases.push(raw);
	return {
		raw,
		db: {
			execute: async <TRow extends Record<string, unknown>>(
				sql: string,
				...args: unknown[]
			): Promise<TRow[]> => {
				if (args.length > 0 || /^\s*SELECT\b/i.test(sql)) {
					return raw.prepare(sql).all(...(args as never[])) as TRow[];
				}
				raw.exec(sql);
				return [];
			},
		},
	};
}

describe("migrations", () => {
	test("applies SQL and function migrations once in order", async () => {
		const { raw, db } = createDatabase();
		const applied: number[] = [];
		const migrate = migrations({
			tableName: "app_schema_version",
			migrations: [
				{
					version: 1,
					sql: "CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
				},
				{
					version: 2,
					up: async (database) => {
						applied.push(2);
						await database.execute(
							"ALTER TABLE items ADD COLUMN active INTEGER NOT NULL DEFAULT 1",
						);
					},
				},
			],
		});

		await migrate(db);
		await migrate(db);

		expect(applied).toEqual([2]);
		expect(
			raw
				.prepare(
					"SELECT schema_version FROM app_schema_version WHERE singleton = 1",
				)
				.get(),
		).toEqual({ schema_version: 2 });
		expect(
			raw
				.prepare("PRAGMA table_info(items)")
				.all()
				.map((row) => row.name),
		).toEqual(["id", "name", "active"]);
	});

	test("uses the default migration table", async () => {
		const { raw, db } = createDatabase();
		await migrations({
			migrations: [{ version: 1, sql: "CREATE TABLE item (id INTEGER)" }],
		})(db);

		expect(
			raw
				.prepare(
					`SELECT schema_version FROM ${DEFAULT_MIGRATION_TABLE_NAME} WHERE singleton = 1`,
				)
				.get(),
		).toEqual({ schema_version: 1 });
	});

	test("adopts an unversioned schema before applying later migrations", async () => {
		const { raw, db } = createDatabase();
		raw.exec(
			"CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
		);
		const applied: number[] = [];

		await migrations({
			tableName: "app_schema_version",
			adopt: async (database) => {
				const tables = await database.execute<{ name: string }>(
					"SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'items'",
				);
				return tables.length === 0 ? 0 : 1;
			},
			migrations: [
				{
					version: 1,
					up: () => {
						throw new Error("already applied migration ran");
					},
				},
				{
					version: 2,
					up: async (database) => {
						applied.push(2);
						await database.execute(
							"ALTER TABLE items ADD COLUMN active INTEGER NOT NULL DEFAULT 1",
						);
					},
				},
			],
		})(db);

		expect(applied).toEqual([2]);
		expect(
			raw
				.prepare(
					"SELECT schema_version FROM app_schema_version WHERE singleton = 1",
				)
				.get(),
		).toEqual({ schema_version: 2 });
	});

	test("rejects invalid ladders and table names before executing", () => {
		expect(() => migrations({ migrations: [] })).toThrow(
			"must not be empty",
		);
		expect(() =>
			migrations({
				migrations: [{ version: 2, sql: "SELECT 1" }],
			}),
		).toThrow("expected version 1");
		expect(() =>
			migrations({
				tableName: "schema; DROP TABLE items",
				migrations: [{ version: 1, sql: "SELECT 1" }],
			}),
		).toThrow("invalid migration table name");
	});

	test("rejects a database newer than the migration ladder", async () => {
		const { raw, db } = createDatabase();
		raw.exec(`
			CREATE TABLE app_schema_version (
				singleton INTEGER PRIMARY KEY,
				schema_version INTEGER NOT NULL
			) STRICT;
			INSERT INTO app_schema_version VALUES (1, 2);
		`);

		await expect(
			migrations({
				tableName: "app_schema_version",
				migrations: [{ version: 1, sql: "SELECT 1" }],
			})(db),
		).rejects.toThrow("newer than supported version 1");
	});
});

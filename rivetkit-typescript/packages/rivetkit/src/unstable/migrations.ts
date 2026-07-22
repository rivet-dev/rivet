import type { RawAccess } from "@/common/database/config";

export const DEFAULT_MIGRATION_TABLE_NAME = "rivetkit_schema_version";

export type MigrationDatabase = Pick<RawAccess, "execute">;

export type Migration =
	| {
			version: number;
			sql: string;
			up?: never;
	  }
	| {
			version: number;
			sql?: never;
			up: (db: MigrationDatabase) => Promise<void> | void;
	  };

export interface MigrationsOptions {
	/** A table owned by this schema. Override this when multiple schemas share a database. */
	tableName?: string;
	/** Detect the version of a schema that predates its migration table. */
	adopt?: (db: MigrationDatabase) => Promise<number> | number;
	/** Contiguous migrations numbered from 1. */
	migrations: readonly Migration[];
}

/**
 * Creates an `onMigrate` callback that applies each unapplied migration in order.
 * RivetKit runs the callback inside its migration savepoint, making the complete
 * ladder atomic when used with `db({ onMigrate })`.
 */
export function migrations({
	tableName = DEFAULT_MIGRATION_TABLE_NAME,
	adopt,
	migrations: ladder,
}: MigrationsOptions): (db: MigrationDatabase) => Promise<void> {
	validateTableName(tableName);
	validateLadder(ladder);

	return async (database) => {
		const quotedTableName = `"${tableName}"`;
		await database.execute(`
			CREATE TABLE IF NOT EXISTS ${quotedTableName} (
				singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
				schema_version INTEGER NOT NULL CHECK (
					schema_version BETWEEN 0 AND ${Number.MAX_SAFE_INTEGER}
				)
			) STRICT
		`);

		const rows = await database.execute<{ schema_version: unknown }>(
			`SELECT schema_version FROM ${quotedTableName} WHERE singleton = 1`,
		);
		if (rows.length > 1) {
			throw new Error(
				`invalid migration table "${tableName}": expected at most one version row, received ${rows.length}`,
			);
		}

		const rawCurrent = rows[0]?.schema_version;
		let current = rows.length === 0 ? 0 : rawCurrent;
		if (rows.length === 0 && adopt) {
			current = await adopt(database);
		}
		if (
			typeof current !== "number" ||
			!Number.isSafeInteger(current) ||
			current < 0
		) {
			throw new Error(
				`invalid schema version ${String(current)} in migration table "${tableName}"`,
			);
		}
		if (current > ladder.length) {
			throw new Error(
				`migration version ${current} in "${tableName}" is newer than supported version ${ladder.length}`,
			);
		}
		if (rows.length === 0 && current > 0) {
			await writeVersion(database, quotedTableName, current);
		}

		for (const migration of ladder.slice(current)) {
			if (migration.sql !== undefined) {
				await database.execute(migration.sql);
			} else {
				await migration.up(database);
			}
			await writeVersion(database, quotedTableName, migration.version);
		}
	};
}

async function writeVersion(
	database: MigrationDatabase,
	quotedTableName: string,
	version: number,
): Promise<void> {
	await database.execute(
		`INSERT INTO ${quotedTableName} (singleton, schema_version)
		 VALUES (1, ?)
		 ON CONFLICT(singleton) DO UPDATE SET schema_version = excluded.schema_version`,
		version,
	);
}

function validateTableName(tableName: string): void {
	if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(tableName)) {
		throw new Error(
			`invalid migration table name "${tableName}": expected a SQLite identifier`,
		);
	}
}

function validateLadder(ladder: readonly Migration[]): void {
	if (ladder.length === 0) {
		throw new Error("migration ladder must not be empty");
	}

	for (const [index, migration] of ladder.entries()) {
		const expectedVersion = index + 1;
		if (migration.version !== expectedVersion) {
			throw new Error(
				`invalid migration ladder: expected version ${expectedVersion}, received ${migration.version}`,
			);
		}
		const hasSql = typeof migration.sql === "string";
		const hasUp = typeof migration.up === "function";
		if (hasSql === hasUp) {
			throw new Error(
				`invalid migration ${migration.version}: expected exactly one of sql or up`,
			);
		}
		if (hasSql && migration.sql.trim().length === 0) {
			throw new Error(
				`invalid migration ${migration.version}: SQL is empty`,
			);
		}
	}
}

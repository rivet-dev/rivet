import { SqliteVfs, type KvVfsOptions } from "@rivetkit/sqlite-vfs";

let sqliteVfsInstance: SqliteVfs | null = null;

export async function getSqliteVfs(): Promise<SqliteVfs> {
	if (!sqliteVfsInstance) {
		sqliteVfsInstance = new SqliteVfs();
	}
	return sqliteVfsInstance;
}

export type { KvVfsOptions };

import { SqliteVfs } from "@rivetkit/sqlite-vfs";

export async function createSqliteVfs() {
	return new SqliteVfs();
}

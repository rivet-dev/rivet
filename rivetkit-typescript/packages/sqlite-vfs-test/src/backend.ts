import { SqliteVfs } from "@rivetkit/sqlite-wasm";

export async function createSqliteVfs() {
	return new SqliteVfs();
}

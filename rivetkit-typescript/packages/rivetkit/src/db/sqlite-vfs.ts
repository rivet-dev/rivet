import type { KvVfsOptions, SqliteVfsConfig } from "@rivetkit/sqlite-vfs";
import { KEYS } from "@/actor/instance/keys";

type SqliteVfsModule = typeof import("@rivetkit/sqlite-vfs/wasm");
type SqliteVfsClass = SqliteVfsModule["SqliteVfs"];

let sqliteVfsClassPromise: Promise<SqliteVfsClass> | null = null;
let sqliteVfsInstance: InstanceType<SqliteVfsClass> | null = null;

function shouldFallback(error: unknown): boolean {
	const err = error as NodeJS.ErrnoException;
	return (
		err?.code === "MODULE_NOT_FOUND" ||
		err?.code === "ERR_MODULE_NOT_FOUND" ||
		err?.code === "ERR_DLOPEN_FAILED"
	);
}

async function loadSqliteVfsClass(): Promise<SqliteVfsClass> {
	if (sqliteVfsClassPromise) {
		return sqliteVfsClassPromise;
	}

	const backend = process.env.RIVETKIT_SQLITE_BACKEND?.toLowerCase();
	const importNative = async () => (await import("@rivetkit/sqlite-vfs/native")).SqliteVfs;
	const importWasm = async () => (await import("@rivetkit/sqlite-vfs/wasm")).SqliteVfs;

	sqliteVfsClassPromise = (async () => {
		if (backend === "native") {
			return await importNative();
		}
		if (backend === "wasm") {
			return await importWasm();
		}

		try {
			return await importNative();
		} catch (error) {
			if (shouldFallback(error)) {
				return await importWasm();
			}
			throw error;
		}
	})();

	return sqliteVfsClassPromise;
}

export async function getSqliteVfs(
	config?: SqliteVfsConfig,
): Promise<InstanceType<SqliteVfsClass>> {
	if (!sqliteVfsInstance) {
		const SqliteVfs = await loadSqliteVfsClass();
		const resolvedConfig: SqliteVfsConfig = {
			...config,
			kvPrefix: KEYS.SQLITE_PREFIX[0],
		};
		sqliteVfsInstance = new SqliteVfs(resolvedConfig);
	}

	return sqliteVfsInstance;
}

export type { KvVfsOptions, SqliteVfsConfig };

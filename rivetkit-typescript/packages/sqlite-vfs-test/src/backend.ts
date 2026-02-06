import type { SqliteVfsConfig } from "@rivetkit/sqlite-vfs";

type SqliteVfsModule = typeof import("@rivetkit/sqlite-vfs/wasm");

function shouldFallback(error: unknown): boolean {
	const err = error as NodeJS.ErrnoException;
	return (
		err?.code === "MODULE_NOT_FOUND" ||
		err?.code === "ERR_MODULE_NOT_FOUND" ||
		err?.code === "ERR_DLOPEN_FAILED"
	);
}

export async function loadSqliteVfsModule(): Promise<SqliteVfsModule> {
	const backend = process.env.RIVETKIT_SQLITE_BACKEND?.toLowerCase();

	if (backend === "native") {
		return await import("@rivetkit/sqlite-vfs/native");
	}

	if (backend === "wasm") {
		return await import("@rivetkit/sqlite-vfs/wasm");
	}

	try {
		return await import("@rivetkit/sqlite-vfs/native");
	} catch (error) {
		if (shouldFallback(error)) {
			return await import("@rivetkit/sqlite-vfs/wasm");
		}
		throw error;
	}
}

export async function createSqliteVfs(config?: SqliteVfsConfig) {
	const module = await loadSqliteVfsModule();
	const resolvedConfig: SqliteVfsConfig = {
		kvPrefix: 9,
		...config,
	};
	return new module.SqliteVfs(resolvedConfig);
}

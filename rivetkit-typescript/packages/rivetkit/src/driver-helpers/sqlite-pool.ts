import type { ISqliteVfs } from "@rivetkit/sqlite-vfs";
import type { RegistryConfig } from "@/registry/config";

/**
 * Manages a lazily-created SqliteVfsPool shared across actors in a driver.
 * Uses dynamic import to keep @rivetkit/sqlite-vfs tree-shakeable.
 */
export class SqliteVfsPoolManager {
	#poolPromise:
		| Promise<{
				acquire(actorId: string): Promise<ISqliteVfs>;
				shutdown(): Promise<void>;
		  }>
		| undefined;
	#config: RegistryConfig;

	constructor(config: RegistryConfig) {
		this.#config = config;
	}

	async acquire(actorId: string): Promise<ISqliteVfs> {
		if (!this.#poolPromise) {
			const poolConfig = this.#config.sqlitePool;
			// Use Array.join() to prevent Turbopack from tracing into the
			// @rivetkit/sqlite-vfs module graph at compile time. Without this,
			// Turbopack resolves the dynamic import statically and follows
			// transitive imports into @rivetkit/sqlite's WASM loader, which
			// Turbopack cannot handle.
			const specifier = ["@rivetkit", "sqlite-vfs"].join("/");
			this.#poolPromise = import(specifier).then(
				({ SqliteVfsPool }: { SqliteVfsPool: new (opts: { actorsPerInstance: number; idleDestroyMs: number }) => { acquire(actorId: string): Promise<ISqliteVfs>; shutdown(): Promise<void> } }) =>
					new SqliteVfsPool({
						actorsPerInstance: poolConfig.actorsPerInstance,
						idleDestroyMs: poolConfig.idleDestroyMs,
					}),
			);
		}
		const pool = await this.#poolPromise;
		return await pool.acquire(actorId);
	}

	async shutdown(): Promise<void> {
		if (this.#poolPromise) {
			const pool = await this.#poolPromise;
			await pool.shutdown();
		}
	}
}

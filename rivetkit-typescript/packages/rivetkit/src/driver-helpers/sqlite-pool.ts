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
			this.#poolPromise = import("@rivetkit/sqlite-vfs").then(
				({ SqliteVfsPool }) =>
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

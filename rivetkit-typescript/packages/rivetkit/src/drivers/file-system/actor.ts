import type { AnyClient } from "@/client/client";
import type { RawDatabaseClient } from "@/db/config";
import type { SqliteVfs } from "@rivetkit/sqlite-vfs";
import type {
	ActorDriver,
	AnyActorInstance,
	ManagerDriver,
} from "@/driver-helpers/mod";
import type { FileSystemGlobalState } from "./global-state";
import { RegistryConfig } from "@/registry/config";

export type ActorDriverContext = Record<never, never>;

/**
 * File System implementation of the Actor Driver
 */
export class FileSystemActorDriver implements ActorDriver {
	#config: RegistryConfig;
	#managerDriver: ManagerDriver;
	#inlineClient: AnyClient;
	#state: FileSystemGlobalState;

	constructor(
		config: RegistryConfig,
		managerDriver: ManagerDriver,
		inlineClient: AnyClient,
		state: FileSystemGlobalState,
	) {
		this.#config = config;
		this.#managerDriver = managerDriver;
		this.#inlineClient = inlineClient;
		this.#state = state;
	}

	async loadActor(actorId: string): Promise<AnyActorInstance> {
		return this.#state.startActor(
			this.#config,
			this.#inlineClient,
			this,
			actorId,
		);
	}

	/**
	 * Get the current storage directory path
	 */
	get storagePath(): string {
		return this.#state.storagePath;
	}

	getContext(_actorId: string): ActorDriverContext {
		return {};
	}

	async kvBatchPut(
		actorId: string,
		entries: [Uint8Array, Uint8Array][],
	): Promise<void> {
		await this.#state.kvBatchPut(actorId, entries);
	}

	async kvBatchGet(
		actorId: string,
		keys: Uint8Array[],
	): Promise<(Uint8Array | null)[]> {
		return await this.#state.kvBatchGet(actorId, keys);
	}

	async kvBatchDelete(actorId: string, keys: Uint8Array[]): Promise<void> {
		await this.#state.kvBatchDelete(actorId, keys);
	}

	async kvListPrefix(
		actorId: string,
		prefix: Uint8Array,
	): Promise<[Uint8Array, Uint8Array][]> {
		return await this.#state.kvListPrefix(actorId, prefix);
	}

	async setAlarm(actor: AnyActorInstance, timestamp: number): Promise<void> {
		await this.#state.setActorAlarm(actor.id, timestamp);
	}

	/** Creates a SQLite VFS instance for creating KV-backed databases */
	async createSqliteVfs(): Promise<SqliteVfs> {
		// Dynamic import keeps @rivetkit/sqlite out of the main entrypoint bundle,
		// preserving tree-shakeability for environments that don't use SQLite.
		// The async @rivetkit/sqlite build is not re-entrant per module instance.
		// Returning a fresh SqliteVfs here gives each actor its own module,
		// allowing actor-level parallelism without cross-actor re-entry.
		//
		// The specifier is built via concatenation so that bundlers like
		// wrangler's esbuild cannot statically analyze and attempt to
		// bundle the module (it is never used on Cloudflare Workers).
		const specifier = "@rivetkit/" + "sqlite-vfs";
		const { SqliteVfs } = await import(specifier);
		return new SqliteVfs();
	}

	startSleep(actorId: string): void {
		// Spawns the sleepActor promise
		this.#state.sleepActor(actorId);
	}

	async startDestroy(actorId: string): Promise<void> {
		await this.#state.destroyActor(actorId);
	}

	async onBeforeActorStart(actor: AnyActorInstance): Promise<void> {
		await actor.cleanupPersistedConnections("file-system-driver.start");
	}
}

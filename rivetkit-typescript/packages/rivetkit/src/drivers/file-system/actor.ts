import type { AnyClient } from "@/client/client";
import type { NativeSqliteConfig, RawDatabaseClient } from "@/db/config";
import type { ISqliteVfs } from "@rivetkit/sqlite-vfs";
import {
	type ActorDriver,
	type AnyActorInstance,
	type ManagerDriver,
} from "@/driver-helpers/mod";
import { SqliteVfsPoolManager } from "@/driver-helpers/sqlite-pool";
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
	#sqlitePool: SqliteVfsPoolManager;
	startSleep?: (actorId: string) => void;

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
		this.#sqlitePool = new SqliteVfsPoolManager(config);

		if (this.#state.persist) {
			// Only define startSleep when persistence is enabled. The actor runtime
			// checks for this property to determine whether the driver supports sleep.
			this.startSleep = (actorId: string) => {
				// Spawns the sleepActor promise.
				this.#state.sleepActor(actorId);
			};
		}
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

	getNativeSqliteConfig(_actorId: string): NativeSqliteConfig | undefined {
		return this.#state.nativeSqliteConfig;
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

	async kvDeleteRange(
		actorId: string,
		start: Uint8Array,
		end: Uint8Array,
	): Promise<void> {
		await this.#state.kvDeleteRange(actorId, start, end);
	}

	async kvListPrefix(
		actorId: string,
		prefix: Uint8Array,
		options?: {
			reverse?: boolean;
			limit?: number;
		},
	): Promise<[Uint8Array, Uint8Array][]> {
		return await this.#state.kvListPrefix(actorId, prefix, options);
	}

	async kvListRange(
		actorId: string,
		start: Uint8Array,
		end: Uint8Array,
		options?: {
			reverse?: boolean;
			limit?: number;
		},
	): Promise<[Uint8Array, Uint8Array][]> {
		return await this.#state.kvListRange(actorId, start, end, options);
	}

	cancelAlarm(actorId: string): void {
		this.#state.cancelAlarmTimeout(actorId);
	}

	async setAlarm(actor: AnyActorInstance, timestamp: number): Promise<void> {
		await this.#state.setActorAlarm(actor.id, timestamp);
	}

	/** Creates a SQLite VFS instance for creating KV-backed databases */
	async createSqliteVfs(actorId: string): Promise<ISqliteVfs> {
		return await this.#sqlitePool.acquire(actorId);
	}

	async shutdownRunner(_immediate: boolean): Promise<void> {
		await this.#sqlitePool.shutdown();
	}

	async hardCrashActor(actorId: string): Promise<void> {
		await this.#state.hardCrashActor(actorId);
	}

	startSleep(actorId: string): void {
		// Spawns the sleepActor promise
		this.#state.sleepActor(actorId);
	}

	ackHibernatableWebSocketMessage(
		gatewayId: ArrayBuffer,
		requestId: ArrayBuffer,
		serverMessageIndex: number,
	): void {
		this.#state.ackHibernatableWebSocketMessage(
			gatewayId,
			requestId,
			serverMessageIndex,
		);
	}

	async startDestroy(actorId: string): Promise<void> {
		await this.#state.destroyActor(actorId);
	}
}

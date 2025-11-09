import type { AnyClient } from "@/client/client";
import type {
	ActorDriver,
	AnyActorInstance,
	ManagerDriver,
} from "@/driver-helpers/mod";
import type { RegistryConfig, RunConfig } from "@/mod";
import type { FileSystemGlobalState } from "./global-state";

export type ActorDriverContext = Record<never, never>;

/**
 * File System implementation of the Actor Driver
 */
export class FileSystemActorDriver implements ActorDriver {
	#registryConfig: RegistryConfig;
	#runConfig: RunConfig;
	#managerDriver: ManagerDriver;
	#inlineClient: AnyClient;
	#state: FileSystemGlobalState;

	constructor(
		registryConfig: RegistryConfig,
		runConfig: RunConfig,
		managerDriver: ManagerDriver,
		inlineClient: AnyClient,
		state: FileSystemGlobalState,
	) {
		this.#registryConfig = registryConfig;
		this.#runConfig = runConfig;
		this.#managerDriver = managerDriver;
		this.#inlineClient = inlineClient;
		this.#state = state;
	}

	async loadActor(actorId: string): Promise<AnyActorInstance> {
		return this.#state.startActor(
			this.#registryConfig,
			this.#runConfig,
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

	getDatabase(actorId: string): Promise<unknown | undefined> {
		return this.#state.createDatabase(actorId);
	}

	startSleep(actorId: string): void {
		// Spawns the sleepActor promise
		this.#state.sleepActor(actorId);
	}
}

import invariant from "invariant";
import type {
	ActorKey,
	ActorRouter,
	AnyActorInstance as CoreAnyActorInstance,
	RegistryConfig,
	RunConfig,
} from "rivetkit";
import { lookupInRegistry } from "rivetkit";
import type { Client } from "rivetkit/client";
import type {
	ActorDriver,
	AnyActorInstance,
	ManagerDriver,
} from "rivetkit/driver-helpers";
import { promiseWithResolvers } from "rivetkit/utils";
import { parseActorId } from "./actor-id";
import { kvDelete, kvGet, kvListPrefix, kvPut } from "./actor-kv";
import { GLOBAL_KV_KEYS } from "./global-kv";
import { getCloudflareAmbientEnv } from "./handler";

interface DurableObjectGlobalState {
	ctx: DurableObjectState;
	env: unknown;
}

/**
 * Cloudflare DO can have multiple DO running within the same global scope.
 *
 * This allows for storing the actor context globally and looking it up by ID in `CloudflareActorsActorDriver`.
 */
export class CloudflareDurableObjectGlobalState {
	// Map of actor ID -> DO state
	#dos: Map<string, DurableObjectGlobalState> = new Map();

	// WeakMap of DO state -> ActorGlobalState for proper GC
	#actors: WeakMap<DurableObjectState, ActorGlobalState> = new WeakMap();

	getDOState(doId: string): DurableObjectGlobalState {
		const state = this.#dos.get(doId);
		invariant(
			state !== undefined,
			"durable object state not in global state",
		);
		return state;
	}

	setDOState(doId: string, state: DurableObjectGlobalState) {
		this.#dos.set(doId, state);
	}

	getActorState(ctx: DurableObjectState): ActorGlobalState | undefined {
		return this.#actors.get(ctx);
	}

	setActorState(ctx: DurableObjectState, actorState: ActorGlobalState): void {
		this.#actors.set(ctx, actorState);
	}
}

export interface DriverContext {
	state: DurableObjectState;
}

interface InitializedData {
	name: string;
	key: ActorKey;
	generation: number;
}

interface LoadedActor {
	actorRouter: ActorRouter;
	actorDriver: ActorDriver;
	generation: number;
}

// Actor global state to track running instances
export class ActorGlobalState {
	// Initialization state
	initialized?: InitializedData;

	// Loaded actor state
	actor?: LoadedActor;
	actorInstance?: AnyActorInstance;
	actorPromise?: ReturnType<typeof promiseWithResolvers<void>>;

	/**
	 * Indicates if `startDestroy` has been called.
	 *
	 * This is stored in memory instead of SQLite since the destroy may be cancelled.
	 *
	 * See the corresponding `destroyed` property in SQLite metadata.
	 */
	destroying: boolean = false;

	reset() {
		this.initialized = undefined;
		this.actor = undefined;
		this.actorInstance = undefined;
		this.actorPromise = undefined;
		this.destroying = false;
	}
}

export class CloudflareActorsActorDriver implements ActorDriver {
	#registryConfig: RegistryConfig;
	#runConfig: RunConfig;
	#managerDriver: ManagerDriver;
	#inlineClient: Client<any>;
	#globalState: CloudflareDurableObjectGlobalState;

	constructor(
		registryConfig: RegistryConfig,
		runConfig: RunConfig,
		managerDriver: ManagerDriver,
		inlineClient: Client<any>,
		globalState: CloudflareDurableObjectGlobalState,
	) {
		this.#registryConfig = registryConfig;
		this.#runConfig = runConfig;
		this.#managerDriver = managerDriver;
		this.#inlineClient = inlineClient;
		this.#globalState = globalState;
	}

	#getDOCtx(actorId: string) {
		// Parse actor ID to get DO ID
		const [doId] = parseActorId(actorId);
		return this.#globalState.getDOState(doId).ctx;
	}

	async loadActor(actorId: string): Promise<AnyActorInstance> {
		// Parse actor ID to get DO ID and generation
		const [doId, expectedGeneration] = parseActorId(actorId);

		// Get the DO state
		const doState = this.#globalState.getDOState(doId);

		// Check if actor is already loaded
		let actorState = this.#globalState.getActorState(doState.ctx);
		if (actorState?.actorInstance) {
			// Actor is already loaded, return it
			return actorState.actorInstance;
		}

		// Create new actor state if it doesn't exist
		if (!actorState) {
			actorState = new ActorGlobalState();
			actorState.actorPromise = promiseWithResolvers();
			this.#globalState.setActorState(doState.ctx, actorState);
		} else if (actorState.actorPromise) {
			// Another request is already loading this actor, wait for it
			await actorState.actorPromise.promise;
			if (!actorState.actorInstance) {
				throw new Error(
					`Actor ${actorId} failed to load in concurrent request`,
				);
			}
			return actorState.actorInstance;
		}

		// Load actor metadata
		const sql = doState.ctx.storage.sql;
		const cursor = sql.exec(
			"SELECT name, key, destroyed, generation FROM _rivetkit_metadata LIMIT 1",
		);
		const result = cursor.raw().next();

		if (result.done || !result.value) {
			throw new Error(
				`Actor ${actorId} is not initialized - missing metadata`,
			);
		}

		const name = result.value[0] as string;
		const key = JSON.parse(result.value[1] as string) as string[];
		const destroyed = result.value[2] as number;
		const generation = result.value[3] as number;

		// Check if actor is destroyed
		if (destroyed) {
			throw new Error(`Actor ${actorId} is destroyed`);
		}

		// Check if generation matches
		if (generation !== expectedGeneration) {
			throw new Error(
				`Actor ${actorId} generation mismatch - expected ${expectedGeneration}, got ${generation}`,
			);
		}

		// Create actor instance
		const definition = lookupInRegistry(this.#registryConfig, name);
		actorState.actorInstance = definition.instantiate();

		// Start actor
		await actorState.actorInstance.start(
			this,
			this.#inlineClient,
			actorId,
			name,
			key,
			"unknown", // TODO: Support regions in Cloudflare
		);

		// Finish
		actorState.actorPromise?.resolve();
		actorState.actorPromise = undefined;

		return actorState.actorInstance;
	}

	getContext(actorId: string): DriverContext {
		// Parse actor ID to get DO ID
		const [doId] = parseActorId(actorId);
		const state = this.#globalState.getDOState(doId);
		return { state: state.ctx };
	}

	async setAlarm(actor: AnyActorInstance, timestamp: number): Promise<void> {
		await this.#getDOCtx(actor.id).storage.setAlarm(timestamp);
	}

	async getDatabase(actorId: string): Promise<unknown | undefined> {
		return this.#getDOCtx(actorId).storage.sql;
	}

	// Batch KV operations
	async kvBatchPut(
		actorId: string,
		entries: [Uint8Array, Uint8Array][],
	): Promise<void> {
		const sql = this.#getDOCtx(actorId).storage.sql;

		for (const [key, value] of entries) {
			kvPut(sql, key, value);
		}
	}

	async kvBatchGet(
		actorId: string,
		keys: Uint8Array[],
	): Promise<(Uint8Array | null)[]> {
		const sql = this.#getDOCtx(actorId).storage.sql;

		const results: (Uint8Array | null)[] = [];
		for (const key of keys) {
			results.push(kvGet(sql, key));
		}

		return results;
	}

	async kvBatchDelete(actorId: string, keys: Uint8Array[]): Promise<void> {
		const sql = this.#getDOCtx(actorId).storage.sql;

		for (const key of keys) {
			kvDelete(sql, key);
		}
	}

	async kvListPrefix(
		actorId: string,
		prefix: Uint8Array,
	): Promise<[Uint8Array, Uint8Array][]> {
		const sql = this.#getDOCtx(actorId).storage.sql;

		return kvListPrefix(sql, prefix);
	}

	startDestroy(actorId: string): void {
		// Parse actor ID to get DO ID and generation
		const [doId, generation] = parseActorId(actorId);

		// Get the DO state
		const doState = this.#globalState.getDOState(doId);
		const actorState = this.#globalState.getActorState(doState.ctx);

		// Actor not loaded, nothing to destroy
		if (!actorState?.actorInstance) {
			return;
		}

		// Check if already destroying
		if (actorState.destroying) {
			return;
		}
		actorState.destroying = true;

		// Spawn onStop in background
		this.#callOnStopAsync(actorId, doId, actorState.actorInstance);
	}

	async #callOnStopAsync(
		actorId: string,
		doId: string,
		actor: CoreAnyActorInstance,
	) {
		// Stop
		await actor.onStop("destroy");

		// Remove state
		const doState = this.#globalState.getDOState(doId);
		const sql = doState.ctx.storage.sql;
		sql.exec("UPDATE _rivetkit_metadata SET destroyed = 1 WHERE 1=1");
		sql.exec("DELETE FROM _rivetkit_kv_storage");

		// Clear any scheduled alarms
		await doState.ctx.storage.deleteAlarm();

		// Delete from ACTOR_KV in the background - use full actorId including generation
		const env = getCloudflareAmbientEnv();
		doState.ctx.waitUntil(
			env.ACTOR_KV.delete(GLOBAL_KV_KEYS.actorMetadata(actorId)),
		);

		// Reset global state using the DO context
		const actorHandle = this.#globalState.getActorState(doState.ctx);
		actorHandle?.reset();
	}
}

export function createCloudflareActorsActorDriverBuilder(
	globalState: CloudflareDurableObjectGlobalState,
) {
	return (
		config: RegistryConfig,
		runConfig: RunConfig,
		managerDriver: ManagerDriver,
		inlineClient: Client<any>,
	) => {
		return new CloudflareActorsActorDriver(
			config,
			runConfig,
			managerDriver,
			inlineClient,
			globalState,
		);
	};
}

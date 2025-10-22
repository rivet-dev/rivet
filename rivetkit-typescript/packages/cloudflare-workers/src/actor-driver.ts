import invariant from "invariant";
import type {
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
import { KEYS } from "./actor-handler-do";

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
	// Single map for all actor state
	#dos: Map<string, DurableObjectGlobalState> = new Map();

	getDOState(actorId: string): DurableObjectGlobalState {
		const state = this.#dos.get(actorId);
		invariant(
			state !== undefined,
			"durable object state not in global state",
		);
		return state;
	}

	setDOState(actorId: string, state: DurableObjectGlobalState) {
		this.#dos.set(actorId, state);
	}
}

export interface DriverContext {
	state: DurableObjectState;
}

// Actor handler to track running instances
class ActorHandler {
	actor?: AnyActorInstance;
	actorPromise?: ReturnType<typeof promiseWithResolvers<void>> =
		promiseWithResolvers();
}

export class CloudflareActorsActorDriver implements ActorDriver {
	#registryConfig: RegistryConfig;
	#runConfig: RunConfig;
	#managerDriver: ManagerDriver;
	#inlineClient: Client<any>;
	#globalState: CloudflareDurableObjectGlobalState;
	#actors: Map<string, ActorHandler> = new Map();

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
		return this.#globalState.getDOState(actorId).ctx;
	}

	async loadActor(actorId: string): Promise<AnyActorInstance> {
		// Check if actor is already loaded
		let handler = this.#actors.get(actorId);
		if (handler) {
			if (handler.actorPromise) await handler.actorPromise.promise;
			if (!handler.actor) throw new Error("Actor should be loaded");
			return handler.actor;
		}

		// Create new actor handler
		handler = new ActorHandler();
		this.#actors.set(actorId, handler);

		// Get the actor metadata from Durable Object storage
		const doState = this.#globalState.getDOState(actorId);
		const storage = doState.ctx.storage;

		// Load actor metadata
		const [name, key] = await Promise.all([
			storage.get<string>(KEYS.NAME),
			storage.get<string[]>(KEYS.KEY),
		]);

		if (!name) {
			throw new Error(
				`Actor ${actorId} is not initialized - missing name`,
			);
		}
		if (!key) {
			throw new Error(
				`Actor ${actorId} is not initialized - missing key`,
			);
		}

		// Create actor instance
		const definition = lookupInRegistry(this.#registryConfig, name);
		handler.actor = definition.instantiate();

		// Start actor
		await handler.actor.start(
			this,
			this.#inlineClient,
			actorId,
			name,
			key,
			"unknown", // TODO: Support regions in Cloudflare
		);

		// Finish
		handler.actorPromise?.resolve();
		handler.actorPromise = undefined;

		return handler.actor;
	}

	getContext(actorId: string): DriverContext {
		const state = this.#globalState.getDOState(actorId);
		return { state: state.ctx };
	}

	async readPersistedData(actorId: string): Promise<Uint8Array | undefined> {
		return await this.#getDOCtx(actorId).storage.get(KEYS.PERSIST_DATA);
	}

	async writePersistedData(actorId: string, data: Uint8Array): Promise<void> {
		await this.#getDOCtx(actorId).storage.put(KEYS.PERSIST_DATA, data);
	}

	async setAlarm(actor: AnyActorInstance, timestamp: number): Promise<void> {
		await this.#getDOCtx(actor.id).storage.setAlarm(timestamp);
	}

	async getDatabase(actorId: string): Promise<unknown | undefined> {
		return this.#getDOCtx(actorId).storage.sql;
	}
}

export function createCloudflareActorsActorDriverBuilder(
	globalState: CloudflareDurableObjectGlobalState,
) {
	return (
		registryConfig: RegistryConfig,
		runConfig: RunConfig,
		managerDriver: ManagerDriver,
		inlineClient: Client<any>,
	) => {
		return new CloudflareActorsActorDriver(
			registryConfig,
			runConfig,
			managerDriver,
			inlineClient,
			globalState,
		);
	};
}

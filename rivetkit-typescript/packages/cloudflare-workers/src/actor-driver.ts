import invariant from "invariant";
import type {
	AnyActorInstance as CoreAnyActorInstance,
	RegistryConfig,
	RunConfig,
} from "rivetkit";
import { lookupInRegistry } from "rivetkit";
import type { Client } from "rivetkit/client";
import {
	type ActorDriver,
	type AnyActorInstance,
	type ManagerDriver,
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

	async setAlarm(actor: AnyActorInstance, timestamp: number): Promise<void> {
		await this.#getDOCtx(actor.id).storage.setAlarm(timestamp);
	}

	async getDatabase(actorId: string): Promise<unknown | undefined> {
		return this.#getDOCtx(actorId).storage.sql;
	}

	// Batch KV operations - convert between Uint8Array and Cloudflare's string-based API
	async kvBatchPut(
		actorId: string,
		entries: [Uint8Array, Uint8Array][],
	): Promise<void> {
		const storage = this.#getDOCtx(actorId).storage;
		const encoder = new TextDecoder();

		// Convert Uint8Array entries to object for Cloudflare batch put
		const storageObj: Record<string, Uint8Array> = {};
		for (const [key, value] of entries) {
			// Convert key from Uint8Array to string
			const keyStr = this.#uint8ArrayToKey(key);
			storageObj[keyStr] = value;
		}

		await storage.put(storageObj);
	}

	async kvBatchGet(
		actorId: string,
		keys: Uint8Array[],
	): Promise<(Uint8Array | null)[]> {
		const storage = this.#getDOCtx(actorId).storage;

		// Convert keys to strings
		const keyStrs = keys.map((k) => this.#uint8ArrayToKey(k));

		// Get values from storage
		const results = await storage.get<Uint8Array>(keyStrs);

		// Convert Map results to array in same order as input keys
		return keyStrs.map((k) => results.get(k) ?? null);
	}

	async kvBatchDelete(actorId: string, keys: Uint8Array[]): Promise<void> {
		const storage = this.#getDOCtx(actorId).storage;

		// Convert keys to strings
		const keyStrs = keys.map((k) => this.#uint8ArrayToKey(k));

		await storage.delete(keyStrs);
	}

	async kvListPrefix(
		actorId: string,
		prefix: Uint8Array,
	): Promise<[Uint8Array, Uint8Array][]> {
		const storage = this.#getDOCtx(actorId).storage;

		// Convert prefix to string
		const prefixStr = this.#uint8ArrayToKey(prefix);

		// List with prefix
		const results = await storage.list<Uint8Array>({ prefix: prefixStr });

		// Convert Map to array of [key, value] tuples
		const entries: [Uint8Array, Uint8Array][] = [];
		for (const [key, value] of results) {
			entries.push([this.#keyToUint8Array(key), value]);
		}

		return entries;
	}

	// Helper to convert Uint8Array key to string for Cloudflare storage
	#uint8ArrayToKey(key: Uint8Array): string {
		// Check if this is a connection key (starts with [2])
		if (key.length > 0 && key[0] === 2) {
			// Connection key - extract connId
			const connId = new TextDecoder().decode(key.slice(1));
			return `${KEYS.CONN_PREFIX}${connId}`;
		}
		// Otherwise, treat as persist data key [1]
		return KEYS.PERSIST_DATA;
	}

	// Helper to convert string key back to Uint8Array
	#keyToUint8Array(key: string): Uint8Array {
		if (key.startsWith(KEYS.CONN_PREFIX)) {
			// Connection key
			const connId = key.slice(KEYS.CONN_PREFIX.length);
			const encoder = new TextEncoder();
			const connIdBytes = encoder.encode(connId);
			const result = new Uint8Array(1 + connIdBytes.length);
			result[0] = 2; // Connection prefix
			result.set(connIdBytes, 1);
			return result;
		}
		// Persist data key
		return Uint8Array.from([1]);
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

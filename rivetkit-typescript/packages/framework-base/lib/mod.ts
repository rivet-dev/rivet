import { Derived, Effect, Store, type Updater } from "@tanstack/store";
import type { AnyActorDefinition, Registry } from "rivetkit";
import {
	type ActorConn,
	ActorConnStatus,
	type ActorHandle,
	type Client,
	type ExtractActorsFromRegistry,
} from "rivetkit/client";

export type AnyActorRegistry = Registry<any>;

export { ActorConnStatus };

interface ActorStateReference<AD extends AnyActorDefinition> {
	/**
	 * The unique identifier for the actor.
	 * This is a hash generated from the actor's options.
	 * It is used to identify the actor instance in the store.
	 * @internal
	 */
	hash: string;
	/**
	 * The state of the actor, derived from the store.
	 * This includes the actor's connection and handle.
	 */
	handle: ActorHandle<AD> | null;
	/**
	 * The connection to the actor.
	 * This is used to communicate with the actor in realtime.
	 */
	connection: ActorConn<AD> | null;
	/**
	 * The connection status of the actor.
	 */
	connStatus: ActorConnStatus;
	/**
	 * The error that occurred while trying to connect to the actor, if any.
	 */
	error: Error | null;
	/**
	 * Options for the actor, including its name, key, parameters, and whether it is enabled.
	 */
	opts: {
		name: keyof AD;
		/**
		 * Unique key for the actor instance.
		 * This can be a string or an array of strings to create multiple instances.
		 * @example "abc" or ["abc", "def"]
		 */
		key: string | string[];
		/**
		 * Parameters for the actor.
		 * These are additional options that can be passed to the actor.
		 */
		params?: Record<string, string>;
		/** Region to create the actor in if it doesn't exist. */
		createInRegion?: string;
		/** Input data to pass to the actor. */
		createWithInput?: unknown;
		/**
		 * Whether the actor is enabled.
		 * Defaults to true.
		 */
		enabled?: boolean;
	};
}

interface InternalRivetKitStore<
	Registry extends AnyActorRegistry,
	Actors extends ExtractActorsFromRegistry<Registry>,
> {
	actors: Record<string, ActorStateReference<Actors>>;
}

/**
 * Options for configuring a actor in RivetKit.
 */
export interface ActorOptions<
	Registry extends AnyActorRegistry,
	ActorName extends keyof ExtractActorsFromRegistry<Registry>,
> {
	/**
	 * Typesafe name of the actor.
	 * This should match the actor's name in the app's actor definitions.
	 * @example "chatRoom"
	 */
	name: ActorName;
	/**
	 * Unique key for the actor instance.
	 * This can be a string or an array of strings to create multiple instances.
	 * @example "abc" or ["abc", "def"]
	 */
	key: string | string[];
	/**
	 * Parameters for the actor.
	 */
	params?: Registry[ExtractActorsFromRegistry<Registry>]["params"];
	/** Region to create the actor in if it doesn't exist. */
	createInRegion?: string;
	/** Input data to pass to the actor. */
	createWithInput?: unknown;
	/**
	 * Whether the actor is enabled.
	 * Defaults to true.
	 */
	enabled?: boolean;
}

export type ActorsStateDerived<
	Registry extends AnyActorRegistry,
	WorkerName extends keyof ExtractActorsFromRegistry<Registry>,
> = Derived<
	Omit<
		InternalRivetKitStore<
			Registry,
			ExtractActorsFromRegistry<Registry>
		>["actors"][string],
		"handle" | "connection"
	> & {
		handle: ActorHandle<
			ExtractActorsFromRegistry<Registry>[WorkerName]
		> | null;
		connection: ActorConn<
			ExtractActorsFromRegistry<Registry>[WorkerName]
		> | null;
	}
>;

export type AnyActorOptions = ActorOptions<AnyActorRegistry, any>;

export interface CreateRivetKitOptions<Registry extends AnyActorRegistry> {
	hashFunction?: (opts: ActorOptions<Registry, any>) => string;
}

type ActorCache<
	Registry extends AnyActorRegistry,
	Actors extends ExtractActorsFromRegistry<Registry>,
> = Map<
	string,
	{
		state: Derived<
			InternalRivetKitStore<Registry, Actors>["actors"][string]
		>;
		key: string;
		mount: () => () => void;
		setState: (
			set: Updater<
				InternalRivetKitStore<Registry, Actors>["actors"][string]
			>,
		) => void;
		create: () => void;
		refCount: number;
	}
>;

export function createRivetKit<
	Registry extends AnyActorRegistry,
	Actors extends ExtractActorsFromRegistry<Registry>,
>(client: Client<Registry>, createOpts: CreateRivetKitOptions<Registry> = {}) {
	type RivetKitStore = InternalRivetKitStore<Registry, Actors>;

	const store = new Store<RivetKitStore>({
		actors: {},
	});

	const cache: ActorCache<Registry, Actors> = new Map();

	return {
		getOrCreateActor: <ActorName extends keyof Actors>(
			actorOpts: ActorOptions<Registry, ActorName>,
		) => getOrCreateActor(client, createOpts, store, cache, actorOpts),
		store,
	};
}

function getOrCreateActor<
	Registry extends AnyActorRegistry,
	Actors extends ExtractActorsFromRegistry<Registry>,
	ActorName extends keyof Actors,
>(
	client: Client<Registry>,
	createOpts: CreateRivetKitOptions<Registry>,
	store: Store<InternalRivetKitStore<Registry, Actors>>,
	cache: ActorCache<Registry, Actors>,
	actorOpts: ActorOptions<Registry, ActorName>,
) {
	type RivetKitStore = InternalRivetKitStore<Registry, Actors>;

	const hash = createOpts.hashFunction || defaultHashFunction;

	const key = hash(actorOpts);
	const cached = cache.get(key);
	if (cached) {
		return {
			...cached,
			state: cached.state as ActorsStateDerived<Registry, ActorName>,
		};
	}

	const derived = new Derived({
		fn: ({ currDepVals: [store] }) => {
			return store.actors[key];
		},
		deps: [store],
	});

	// Connect to actor
	const effect = new Effect({
		fn: () => {
			const actor = store.state.actors[key];

			// Dispose connection if disabled
			if (!actor.opts.enabled && actor.connection) {
				actor.connection.dispose();

				// Clear state references
				store.setState((prev) => ({
					...prev,
					actors: {
						...prev.actors,
						[key]: {
							...prev.actors[key],
							connection: null,
							handle: null,
						},
					},
				}));
				return;
			}

			// Create actor connection
			//
			// actor-conn.ts automatically handles reconnect, this is only
			// required if being called for the first time if enabled
			if (
				actor.connStatus === ActorConnStatus.Idle &&
				actor.opts.enabled
			) {
				create<Registry, Actors, ActorName>(client, store, key);
			}
		},
		deps: [derived],
	});

	store.setState((prev) => {
		if (prev.actors[key]) {
			return prev;
		}
		return {
			...prev,
			actors: {
				...prev.actors,
				[key]: {
					hash: key,
					connStatus: ActorConnStatus.Idle,
					connection: null,
					handle: null,
					error: null,
					opts: actorOpts,
				},
			},
		};
	});

	function setState(updater: Updater<RivetKitStore["actors"][string]>) {
		store.setState((prev) => {
			const actor = prev.actors[key];
			if (!actor) {
				throw new Error(`Actor with key "${key}" does not exist.`);
			}

			let newState: RivetKitStore["actors"][string];

			if (typeof updater === "function") {
				newState = updater(actor);
			} else {
				// If updater is a direct value, we assume it replaces the entire actor state
				newState = updater;
			}
			return {
				...prev,
				actors: {
					...prev.actors,
					[key]: newState,
				},
			};
		});
	}

	// Track subscriptions for ref counting
	let unsubscribeDerived: (() => void) | null = null;
	let unsubscribeEffect: (() => void) | null = null;

	const mount = () => {
		const cached = cache.get(key);
		if (!cached) {
			throw new Error(`Actor with key "${key}" not found in cache`);
		}

		// Increment ref count
		cached.refCount++;

		// Mount derived/effect on first reference
		if (cached.refCount === 1) {
			unsubscribeDerived = derived.mount();
			unsubscribeEffect = effect.mount();
		}

		return () => {
			// Decrement ref count
			cached.refCount--;

			// Only cleanup when last reference is removed
			if (cached.refCount === 0) {
				// Unsubscribe from derived/effect
				unsubscribeDerived?.();
				unsubscribeEffect?.();
				unsubscribeDerived = null;
				unsubscribeEffect = null;

				// Dispose connection
				const actor = store.state.actors[key];
				if (actor?.connection) {
					actor.connection.dispose();
				}

				// Remove from store and cache
				store.setState((prev) => {
					const { [key]: _, ...rest } = prev.actors;
					return { ...prev, actors: rest };
				});
				cache.delete(key);
			}
		};
	};

	cache.set(key, {
		state: derived,
		key,
		mount,
		setState,
		create: create.bind(undefined, client, store, key),
		refCount: 0,
	});

	return {
		mount,
		setState,
		state: derived as ActorsStateDerived<Registry, ActorName>,
		create,
		key,
	};
}

function create<
	Registry extends AnyActorRegistry,
	Actors extends ExtractActorsFromRegistry<Registry>,
	ActorName extends keyof Actors,
>(
	client: Client<Registry>,
	store: Store<InternalRivetKitStore<Registry, Actors>>,
	key: string,
) {
	// Save actor to map
	store.setState((prev) => ({
		...prev,
		actors: {
			...prev.actors,
			[key]: {
				...prev.actors[key],
				connStatus: ActorConnStatus.Connecting,
				error: null,
			},
		},
	}));

	const actor = store.state.actors[key];
	try {
		const handle = client.getOrCreate(
			actor.opts.name as string,
			actor.opts.key,
			{
				params: actor.opts.params,
				createInRegion: actor.opts.createInRegion,
				createWithInput: actor.opts.createWithInput,
			},
		);

		const connection = handle.connect();

		// Subscribe to connection state changes
		connection.onStatusChange((status) => {
			store.setState((prev) => {
				// Only update if this is still the active connection
				if (prev.actors[key]?.connection !== connection) return prev;
				return {
					...prev,
					actors: {
						...prev.actors,
						[key]: {
							...prev.actors[key],
							connStatus: status,
							// Only clear error when successfully connected
							...(status === ActorConnStatus.Connected
								? { error: null }
								: {}),
						},
					},
				};
			});
		});

		// onError is followed by onClose which will set connStatus to Disconnected
		connection.onError((error) => {
			store.setState((prev) => {
				// Only update if this is still the active connection
				if (prev.actors[key]?.connection !== connection) return prev;
				return {
					...prev,
					actors: {
						...prev.actors,
						[key]: {
							...prev.actors[key],
							error,
						},
					},
				};
			});
		});

		store.setState((prev) => ({
			...prev,
			actors: {
				...prev.actors,
				[key]: {
					...prev.actors[key],
					handle: handle as ActorHandle<Actors[ActorName]>,
					connection: connection as ActorConn<Actors[ActorName]>,
				},
			},
		}));
	} catch (error) {
		console.error("Failed to create actor connection", error);
		store.setState((prev) => ({
			...prev,
			actors: {
				...prev.actors,
				[key]: {
					...prev.actors[key],
					// Use Disconnected so Effect won't auto-retry
					// User must re-enable or take action to retry
					connStatus: ActorConnStatus.Disconnected,
					error: error as Error,
				},
			},
		}));
	}
}

function defaultHashFunction({ name, key, params }: AnyActorOptions) {
	return JSON.stringify({ name, key, params });
}

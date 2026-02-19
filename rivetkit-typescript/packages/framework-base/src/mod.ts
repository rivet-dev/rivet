import { Derived, Effect, Store } from "@tanstack/store";
import equal from "fast-deep-equal";
import type { AnyActorDefinition, Registry } from "rivetkit";
import {
	type ActorConn,
	type ActorConnStatus,
	type ActorHandle,
	type Client,
	type ExtractActorsFromRegistry,
} from "rivetkit/client";

export type AnyActorRegistry = Registry<any>;

export type { ActorConnStatus };

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
		/**
		 * If true, only gets the actor if it already exists. Does not create the actor.
		 * Throws an error if the actor is not found.
		 * Defaults to false.
		 */
		noCreate?: boolean;
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
	/**
	 * If true, only gets the actor if it already exists. Does not create the actor.
	 * Throws an error if the actor is not found.
	 * Defaults to false.
	 */
	noCreate?: boolean;
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
		/** @deprecated Use `connStatus === "connected"` instead */
		isConnected: boolean;
	}
>;

export type AnyActorOptions = ActorOptions<AnyActorRegistry, any>;

export interface CreateRivetKitOptions<Registry extends AnyActorRegistry> {
	hashFunction?: (opts: ActorOptions<Registry, any>) => string;
}

type ComputedActorState<
	Registry extends AnyActorRegistry,
	Actors extends ExtractActorsFromRegistry<Registry>,
> = InternalRivetKitStore<Registry, Actors>["actors"][string] & {
	/** @deprecated Use `connStatus === "connected"` instead */
	isConnected: boolean;
};

type ActorCache<
	Registry extends AnyActorRegistry,
	Actors extends ExtractActorsFromRegistry<Registry>,
> = Map<
	string,
	{
		state: Derived<ComputedActorState<Registry, Actors>>;
		key: string;
		mount: () => () => void;
		create: () => void;
		refCount: number;
		cleanupTimeout: ReturnType<typeof setTimeout> | null;
	}
>;

export function createRivetKit<
	Registry extends AnyActorRegistry,
	Actors extends ExtractActorsFromRegistry<Registry>,
>(client: Client<Registry>, createOpts: CreateRivetKitOptions<Registry> = {}) {
	const store = new Store<InternalRivetKitStore<Registry, Actors>>({
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

type ActorUpdates<
	Registry extends AnyActorRegistry,
	Actors extends ExtractActorsFromRegistry<Registry>,
> = Partial<InternalRivetKitStore<Registry, Actors>["actors"][string]>;

function updateActor<
	Registry extends AnyActorRegistry,
	Actors extends ExtractActorsFromRegistry<Registry>,
>(
	store: Store<InternalRivetKitStore<Registry, Actors>>,
	key: string,
	updates: ActorUpdates<Registry, Actors>,
) {
	store.setState((prev) => ({
		...prev,
		actors: {
			...prev.actors,
			[key]: { ...prev.actors[key], ...updates },
		},
	}));
}

// See README.md for lifecycle documentation.
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
	const hash = createOpts.hashFunction || defaultHashFunction;

	const normalizedOpts = {
		...actorOpts,
		enabled: actorOpts.enabled ?? true,
	};

	const key = hash(normalizedOpts);

	// Sync opts to store on every call (even for cached entries)
	// Use queueMicrotask for updates to avoid "Cannot update a component while rendering" React error
	const existing = store.state.actors[key];
	if (!existing) {
		store.setState((prev) => ({
			...prev,
			actors: {
				...prev.actors,
				[key]: {
					hash: key,
					connStatus: "idle",
					connection: null,
					handle: null,
					error: null,
					opts: normalizedOpts,
				},
			},
		}));
	} else if (!optsEqual(existing.opts, normalizedOpts)) {
		// Defer opts update to avoid triggering re-render during render
		queueMicrotask(() => {
			updateActor(store, key, { opts: normalizedOpts });
		});
	}

	const cached = cache.get(key);
	if (cached) {
		return {
			...cached,
			state: cached.state as ActorsStateDerived<Registry, ActorName>,
		};
	}

	const derived = new Derived({
		fn: ({ currDepVals: [store] }) => {
			const actor = store.actors[key];
			return {
				...actor,
				/** @deprecated Use `connStatus === "connected"` instead */
				isConnected: actor.connStatus === "connected",
			};
		},
		deps: [store],
	});

	// Handle enabled/disabled state changes.
	// Initial connection is triggered directly in mount() since Effect
	// only runs on state changes, not on mount.
	const effect = new Effect({
		fn: () => {
			const actor = store.state.actors[key];
			if (!actor) {
				throw new Error(
					`Actor with key "${key}" not found in store. This indicates a bug in cleanup logic.`,
				);
			}

			// Dispose connection if disabled
			if (!actor.opts.enabled && actor.connection) {
				actor.connection.dispose();

				// Reset state so re-enabling will reconnect
				updateActor(store, key, {
					connection: null,
					handle: null,
					connStatus: "idle",
				});
				return;
			}

			// Reconnect when re-enabled after being disabled
			// Defer to avoid "Cannot update a component while rendering" React error
			if (
				actor.connStatus === "idle" &&
				actor.opts.enabled
			) {
				queueMicrotask(() => {
					// Re-check state after microtask in case it changed
					const currentActor = store.state.actors[key];
					if (
						currentActor &&
						currentActor.connStatus === "idle" &&
						currentActor.opts.enabled
					) {
						create<Registry, Actors, ActorName>(client, store, key);
					}
				});
			}
		},
		deps: [derived],
	});

	// Track subscriptions for ref counting
	let unsubscribeDerived: (() => void) | null = null;
	let unsubscribeEffect: (() => void) | null = null;

	const mount = () => {
		const cached = cache.get(key);
		if (!cached) {
			throw new Error(
				`Actor with key "${key}" not found in cache. This indicates a bug in cleanup logic.`,
			);
		}

		// Cancel pending cleanup
		if (cached.cleanupTimeout !== null) {
			clearTimeout(cached.cleanupTimeout);
			cached.cleanupTimeout = null;
		}

		// Increment ref count
		cached.refCount++;

		// Mount derived/effect on first reference (or re-mount after cleanup)
		if (cached.refCount === 1) {
			unsubscribeDerived = derived.mount();
			unsubscribeEffect = effect.mount();

			// Effect doesn't run immediately on mount, only on state changes.
			// Trigger initial connection if actor is enabled and idle.
			const actor = store.state.actors[key];
			if (
				actor &&
				actor.opts.enabled &&
				actor.connStatus === "idle"
			) {
				create<Registry, Actors, ActorName>(client, store, key);
			}
		}

		return () => {
			// Decrement ref count
			cached.refCount--;

			if (cached.refCount === 0) {
				// Deferred cleanup prevents needless reconnection when:
				// - React Strict Mode's unmount/remount cycle
				// - useActor hook moves between components in the same render cycle
				cached.cleanupTimeout = setTimeout(() => {
					cached.cleanupTimeout = null;
					if (cached.refCount > 0) return;

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
				}, 0);
			}
		};
	};

	cache.set(key, {
		state: derived,
		key,
		mount,
		create: create.bind(undefined, client, store, key),
		refCount: 0,
		cleanupTimeout: null,
	});

	return {
		mount,
		state: derived as ActorsStateDerived<Registry, ActorName>,
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
	const actor = store.state.actors[key];
	if (!actor) {
		throw new Error(
			`Actor with key "${key}" not found in store. This indicates a bug in cleanup logic.`,
		);
	}

	// Save actor to map
	updateActor(store, key, {
		connStatus: "connecting",
		error: null,
	});

	try {
		const handle = actor.opts.noCreate
			? client.get(
					actor.opts.name as string,
					actor.opts.key,
					{
						params: actor.opts.params,
					},
				)
			: client.getOrCreate(
					actor.opts.name as string,
					actor.opts.key,
					{
						params: actor.opts.params,
						createInRegion: actor.opts.createInRegion,
						createWithInput: actor.opts.createWithInput,
					},
				);

		const connection = handle.connect();

		// Store connection BEFORE registering callbacks to avoid race condition
		// where status change fires before connection is stored
		updateActor(store, key, {
			handle: handle as ActorHandle<Actors[ActorName]>,
			connection: connection as ActorConn<Actors[ActorName]>,
		});

		// Subscribe to connection state changes
		connection.onStatusChange((status) => {
			store.setState((prev) => {
				// Only update if this is still the active connection
				const isActiveConnection = prev.actors[key]?.connection === connection;
				if (!isActiveConnection) return prev;
				return {
					...prev,
					actors: {
						...prev.actors,
						[key]: {
							...prev.actors[key],
							connStatus: status,
							// Only clear error when successfully connected
							...(status === "connected"
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
	} catch (error) {
		console.error("Failed to create actor connection", error);
		// Use Disconnected so Effect won't auto-retry
		// User must re-enable or take action to retry
		updateActor(store, key, {
			connStatus: "disconnected",
			error: error as Error,
		});
	}
}

function defaultHashFunction({ name, key, params, noCreate }: AnyActorOptions) {
	return JSON.stringify({ name, key, params, noCreate });
}

function optsEqual(a: AnyActorOptions, b: AnyActorOptions) {
	return equal(a, b);
}

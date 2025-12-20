import { Derived, Effect, Store, type Updater } from "@tanstack/store";
import type { AnyActorDefinition, Registry } from "rivetkit";
import type {
	ActorConn,
	ActorHandle,
	Client,
	ExtractActorsFromRegistry,
} from "rivetkit/client";

export type AnyActorRegistry = Registry<any>;

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
	 * Whether the actor is enabled.
	 */
	isConnected?: boolean;
	/**
	 * Whether the actor is currently connecting, indicating that a connection attempt is in progress.
	 */
	isConnecting?: boolean;
	/**
	 * Whether there was an error connecting to the actor.
	 */
	isError?: boolean;
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

export function createRivetKit<
	Registry extends AnyActorRegistry,
	Actors extends ExtractActorsFromRegistry<Registry>,
	ActorNames extends keyof Actors,
>(client: Client<Registry>, opts: CreateRivetKitOptions<Registry> = {}) {
	type RivetKitStore = InternalRivetKitStore<Registry, Actors>;

	const store = new Store<RivetKitStore>({
		actors: {},
	});

	const hash = opts.hashFunction || defaultHashFunction;

	const cache = new Map<
		string,
		{
			state: Derived<RivetKitStore["actors"][string]>;
			key: string;
			mount: () => void;
			setState: (set: Updater<RivetKitStore["actors"][string]>) => void;
			create: () => void;
		}
	>();

	function getOrCreateActor<ActorName extends ActorNames>(
		opts: ActorOptions<Registry, ActorName>,
	) {
		const key = hash(opts);
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

		function create() {
			async function createActorConnection() {
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
					connection.onOpen(() => {
						store.setState((prev) => {
							// Only update if this is still the active connection
							if (prev.actors[key]?.connection !== connection) return prev;
							return {
								...prev,
								actors: {
									...prev.actors,
									[key]: {
										...prev.actors[key],
										isConnected: true,
									},
								},
							};
						});
					});

					connection.onClose(() => {
						store.setState((prev) => {
							// Only update if this is still the active connection
							if (prev.actors[key]?.connection !== connection) return prev;
							return {
								...prev,
								actors: {
									...prev.actors,
									[key]: {
										...prev.actors[key],
										isConnected: false,
									},
								},
							};
						});
					});

					await handle.resolve(
						/*{ signal: AbortSignal.timeout(0) }*/
					);
					store.setState((prev) => {
						return {
							...prev,
							actors: {
								...prev.actors,
								[key]: {
									...prev.actors[key],
									isConnected: connection.isConnected,
									isConnecting: false,
									handle: handle as ActorHandle<
										Actors[ActorName]
									>,
									connection: connection as ActorConn<
										Actors[ActorName]
									>,
									isError: false,
									error: null,
								},
							},
						};
					});
				} catch (error) {
					store.setState((prev) => {
						return {
							...prev,
							actors: {
								...prev.actors,
								[key]: {
									...prev.actors[key],
									isError: true,
									isConnecting: false,
									error: error as Error,
								},
							},
						};
					});
				}
			}

			store.setState((prev) => {
				prev.actors[key].isConnecting = true;
				prev.actors[key].isError = false;
				prev.actors[key].error = null;
				createActorConnection();
				return prev;
			});
		}

		// connect effect
		const effect = new Effect({
			fn: () => {
				// check if prev state is different from current state
				// do a shallow comparison
				const actor = store.state.actors[key];

				const isSame =
					JSON.stringify(store.prevState.actors[key].opts) ===
					JSON.stringify(store.state.actors[key].opts);

				if (
					isSame &&
					!actor.isConnected &&
					!actor.isConnecting &&
					!actor.isError &&
					actor.opts.enabled
				) {
					create();
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
						isConnected: false,
						isConnecting: false,
						connection: null,
						handle: null,
						isError: false,
						error: null,
						opts,
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

		const mount = () => {
			const unsubscribeDerived = derived.mount();
			const unsubscribeEffect = effect.mount();

			return () => {
				unsubscribeDerived();
				unsubscribeEffect();
			};
		};

		cache.set(key, {
			state: derived,
			key,
			mount,
			setState,
			create,
		});

		return {
			mount,
			setState,
			state: derived as ActorsStateDerived<Registry, ActorName>,
			create,
			key,
		};
	}

	return {
		getOrCreateActor,
		store,
	};
}

function defaultHashFunction({ name, key, params }: AnyActorOptions) {
	return JSON.stringify({ name, key, params });
}

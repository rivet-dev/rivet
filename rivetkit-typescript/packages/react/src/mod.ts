import {
	type ActorOptions,
	type ActorsStateDerived,
	type AnyActorRegistry,
	type CreateRivetKitOptions,
	createRivetKit as createVanillaRivetKit,
} from "@rivetkit/framework-base";
import {
	createActorQueryKey,
	type QueryClientLike,
	type QueryCoreOptions,
	type QueryKey,
	type QueryKeyFn,
	syncActorToQueryClient,
} from "@rivetkit/query-core";
import {
	type UseQueryOptions,
	type UseQueryResult,
	useQuery,
	useQueryClient,
} from "@tanstack/react-query";
import { useStore } from "@tanstack/react-store";
import { useEffect, useRef } from "react";
import {
	type ActorConn,
	type ActorConnOn,
	type AnyActorDefinition,
	type Client,
	type ClientConfigInput,
	createClient,
	type ExtractActorsFromRegistry,
} from "rivetkit/client";

export type { ActorConnStatus } from "@rivetkit/framework-base";
export type {
	QueryClientLike,
	QueryKey,
	QueryKeyFn,
} from "@rivetkit/query-core";
export {
	createActorQueryKey,
	syncActorToQueryClient,
} from "@rivetkit/query-core";
export { ActorConnDisposed, createClient } from "rivetkit/client";

export type CreateRivetKitReactOptions<Registry extends AnyActorRegistry> =
	CreateRivetKitOptions<Registry> & QueryCoreOptions<Registry>;

export type UseActorEvent<AD extends AnyActorDefinition> = ActorConnOn<AD>;

export type RivetKitReactBase<Registry extends AnyActorRegistry> = {
	useActor: <
		ActorName extends Exclude<
			keyof ExtractActorsFromRegistry<Registry>,
			number | symbol
		>,
	>(
		actorOpts: ActorOptions<Registry, ActorName>,
	) => ActorsStateDerived<Registry, ActorName>["state"] & {
		useEvent: ActorConnOn<ExtractActorsFromRegistry<Registry>[ActorName]>;
	};
};

// The serializable slice of actor state stored in the query cache.
// connection and handle are live actor proxies — they are stripped before
// setQueryData so TanStack Query never serializes them (which would trigger
// a spurious toJSON remote action call on the actor).
export type ActorConnState<
	Registry extends AnyActorRegistry,
	ActorName extends keyof ExtractActorsFromRegistry<Registry> & string,
> = Omit<ActorsStateDerived<Registry, ActorName>["state"], "connection" | "handle">;

export type RivetKitReactWithQuery<Registry extends AnyActorRegistry> =
	RivetKitReactBase<Registry> & {
		useActorQuery: <
			ActorName extends Exclude<
				keyof ExtractActorsFromRegistry<Registry>,
				number | symbol
			>,
			QueryData = ActorConnState<Registry, ActorName>,
			ErrorType = Error,
		>(
			actorOpts: ActorOptions<Registry, ActorName>,
			options?: UseActorQueryOptions<
				Registry,
				ActorName,
				QueryData,
				ErrorType
			>,
		) => UseQueryResult<QueryData, ErrorType>;
		createActorQueryKey: <
			ActorName extends Exclude<
				keyof ExtractActorsFromRegistry<Registry>,
				number | symbol
			>,
		>(
			actorOpts: ActorOptions<Registry, ActorName>,
			queryKeyFn?: QueryKeyFn<Registry, ActorName>,
		) => QueryKey;
	};

export type UseActorQueryOptions<
	Registry extends AnyActorRegistry,
	ActorName extends Exclude<
		keyof ExtractActorsFromRegistry<Registry>,
		number | symbol
	>,
	QueryData,
	ErrorType,
> = Omit<
	UseQueryOptions<
		ActorConnState<Registry, ActorName>,
		ErrorType,
		QueryData,
		QueryKey
	>,
	"queryKey" | "queryFn"
> & {
	queryKeyFn?: QueryKeyFn<Registry, ActorName>;
};

export function createRivetKit<Registry extends AnyActorRegistry>(
	clientInput: string | ClientConfigInput | undefined,
	opts: CreateRivetKitReactOptions<Registry> & {
		queryClient: QueryClientLike;
	},
): RivetKitReactWithQuery<Registry>;
export function createRivetKit<Registry extends AnyActorRegistry>(
	clientInput?: string | ClientConfigInput,
	opts?: CreateRivetKitReactOptions<Registry>,
): RivetKitReactBase<Registry>;
export function createRivetKit<Registry extends AnyActorRegistry>(
	clientInput: string | ClientConfigInput | undefined = undefined,
	opts: CreateRivetKitReactOptions<Registry> = {},
) {
	return createRivetKitWithClient<Registry>(
		createClient<Registry>(clientInput),
		opts,
	);
}

// biome-ignore lint: Type instantiation can be excessively deep for complex registries.
// @ts-ignore
export function createRivetKitWithClient<Registry extends AnyActorRegistry>(
	client: Client<Registry>,
	opts: CreateRivetKitReactOptions<Registry> & {
		queryClient: QueryClientLike;
	},
): RivetKitReactWithQuery<Registry>;
export function createRivetKitWithClient<Registry extends AnyActorRegistry>(
	client: Client<Registry>,
	opts?: CreateRivetKitReactOptions<Registry>,
): RivetKitReactBase<Registry>;
export function createRivetKitWithClient<Registry extends AnyActorRegistry>(
	client: Client<Registry>,
	opts: CreateRivetKitReactOptions<Registry> = {},
) {
	// biome-ignore lint: Type instantiation can be excessively deep for complex registries.
	// @ts-ignore Type instantiation can be excessively deep for complex registries.
	const { getOrCreateActor } = createVanillaRivetKit(client, opts);
	const queryClient = opts.queryClient;
	const queryKeyFn = opts.queryKeyFn;

	/**
	 * Hook to connect to a actor and retrieve its state. Using this hook with the same options
	 * will return the same actor instance. This simplifies passing around the actor state in your components.
	 * It also provides a method to listen for events emitted by the actor.
	 * @param opts - Options for the actor, including its name, key, and parameters.
	 * @returns An object containing the actor's state and a method to listen for events.
	 */
	function useActor<
		ActorName extends Exclude<
			keyof ExtractActorsFromRegistry<Registry>,
			number | symbol
		>,
	>(
		opts: ActorOptions<Registry, ActorName>,
) {
		// getOrCreateActor syncs opts to store on every call
		const { mount, state } = getOrCreateActor<ActorName>(opts);

		useEffect(() => {
			return mount();
		}, [mount]);

		// Serialize the actor identity to a stable string so the effect only
		// re-runs when the actor actually changes, not on every render due to
		// new inline object references for opts.
		const actorKeyStr = queryClient
			? JSON.stringify(createActorQueryKey(opts, queryKeyFn))
			: null;
		// biome-ignore lint/correctness/useExhaustiveDependencies: actorKeyStr is the serialized identity of opts; re-subscribing on every render would cause an infinite loop
		useEffect(() => {
			if (!queryClient) return;
			const { unsubscribe } = syncActorToQueryClient({
				actorOpts: opts,
				getOrCreateActor,
				queryClient,
				queryKeyFn,
				mount: false,
			});
			return unsubscribe;
		}, [actorKeyStr]);

		const actorState = useStore(state);
		type UseEvent = (typeof actorState)["connection"] extends ActorConn<
			infer AD
		> | null
			? ActorConn<AD>["on"]
			: never;

		/**
		 * Hook to listen for events emitted by the actor.
		 * This hook allows you to subscribe to specific events emitted by the actor and execute a handler function
		 * when the event occurs.
		 * It uses the `useEffect` hook to set up the event listener when the actor connection is established.
		 * It cleans up the listener when the component unmounts or when the actor connection changes.
		 * @param eventName The name of the event to listen for.
		 * @param handler The function to call when the event is emitted.
		 */
		const useEvent = ((
			eventName: string,
			handler: (...args: unknown[]) => void,
		) => {
			// biome-ignore lint/correctness/useHookAtTopLevel: hooks are used in a dedicated hook factory
			const ref = useRef(handler);
			// biome-ignore lint/correctness/useHookAtTopLevel: hooks are used in a dedicated hook factory
			const actorState = useStore(state);

			// biome-ignore lint/correctness/useHookAtTopLevel: hooks are used in a dedicated hook factory
			useEffect(() => {
				ref.current = handler;
			}, [handler]);

			// biome-ignore lint/correctness/useExhaustiveDependencies: it's okay to not include all dependencies here
			// biome-ignore lint/correctness/useHookAtTopLevel: hooks are used in a dedicated hook factory
			useEffect(() => {
				const connection = actorState.connection as {
					on: (
						eventName: string,
						callback: (...args: unknown[]) => void,
					) => () => void;
				} | null;
				if (!connection) return;

				function eventHandler(...args: unknown[]) {
					ref.current(...args);
				}
				return connection.on(eventName, eventHandler);
			}, [
				actorState.connection,
				actorState.connStatus,
				actorState.hash,
				eventName,
			]);
		}) as UseEvent;

		return {
			...actorState,
			useEvent,
		} as unknown as ActorsStateDerived<Registry, ActorName>["state"] & { useEvent: ActorConnOn<ExtractActorsFromRegistry<Registry>[ActorName]> };
	}

	function useActorQuery<
		ActorName extends Exclude<
			keyof ExtractActorsFromRegistry<Registry>,
			number | symbol
		>,
		QueryData = ActorConnState<Registry, ActorName>,
		ErrorType = Error,
	>(
		actorOpts: ActorOptions<Registry, ActorName>,
		options: UseActorQueryOptions<
			Registry,
			ActorName,
			QueryData,
			ErrorType
		> = {},
	) {
		const { queryKeyFn: queryKeyFnOverride, ...queryOptions } = options;
		const resolvedQueryKeyFn = queryKeyFnOverride ?? queryKeyFn;
		// biome-ignore lint/correctness/useHookAtTopLevel: this hook is the top-level public API
		const queryKey = createActorQueryKey(actorOpts, resolvedQueryKeyFn);
		// biome-ignore lint/correctness/useHookAtTopLevel: this hook is the top-level public API
		const localQueryClient = useQueryClient();
		// biome-ignore lint/correctness/useHookAtTopLevel: this hook is the top-level public API
		useEffect(() => {
			const { unsubscribe } = syncActorToQueryClient({
				actorOpts,
				getOrCreateActor,
				queryClient: localQueryClient,
				queryKeyFn: resolvedQueryKeyFn,
			});
			return unsubscribe;
			// biome-ignore lint/correctness/useExhaustiveDependencies: actorOpts changes should resubscribe
		}, [actorOpts, localQueryClient, resolvedQueryKeyFn]);
		// biome-ignore lint/correctness/useHookAtTopLevel: this hook is the top-level public API
		return useQuery<
			ActorConnState<Registry, ActorName>,
			ErrorType,
			QueryData,
			QueryKey
		>({
			...queryOptions,
			queryKey,
			// Actor state is pushed into the cache via setQueryData from the sync
			// subscription above. There is no way to fetch actor state on demand,
			// so we disable automatic fetching and rely entirely on the subscription.
			queryFn: () => Promise.resolve(null as unknown as ActorConnState<Registry, ActorName>),
			enabled: false,
		});
	}

	return {
		useActor,
		useActorQuery,
		createActorQueryKey: <
			ActorName extends Exclude<
				keyof ExtractActorsFromRegistry<Registry>,
				number | symbol
			>,
		>(
			actorOpts: ActorOptions<Registry, ActorName>,
			queryKeyFnOverride?: QueryKeyFn<Registry, ActorName>,
		) => createActorQueryKey(actorOpts, queryKeyFnOverride ?? queryKeyFn),
	};
}

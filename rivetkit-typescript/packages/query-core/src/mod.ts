import type {
	ActorOptions,
	ActorsStateDerived,
	AnyActorRegistry,
	CreateRivetKitOptions,
} from "@rivetkit/framework-base";
import type { ExtractActorsFromRegistry } from "rivetkit/client";

export type QueryKey = ReadonlyArray<unknown>;

export type QueryKeyFn<
	Registry extends AnyActorRegistry,
	ActorName extends keyof ExtractActorsFromRegistry<Registry> & string,
> = (opts: ActorOptions<Registry, ActorName>) => QueryKey;

export interface QueryClientLike {
	setQueryData: (queryKey: QueryKey, updater: unknown | ((prev: unknown) => unknown)) => void;
	getQueryData: (queryKey: QueryKey) => unknown;
}

export interface SyncActorOptions<
	Registry extends AnyActorRegistry,
	ActorName extends keyof ExtractActorsFromRegistry<Registry> & string,
> {
	actorOpts: ActorOptions<Registry, ActorName>;
	getOrCreateActor: <Name extends ActorName>(
		actorOpts: ActorOptions<Registry, Name>,
	) => {
		state: ActorsStateDerived<Registry, Name>;
		mount: () => () => void;
		key: string;
	};
	queryClient: QueryClientLike;
	queryKeyFn?: QueryKeyFn<Registry, ActorName>;
	mount?: boolean;
}

export type QueryCoreOptions<Registry extends AnyActorRegistry> = Pick<
	CreateRivetKitOptions<Registry>,
	"hashFunction"
> & {
	queryClient?: QueryClientLike;
	queryKeyFn?: QueryKeyFn<Registry, any>;
};

export function createActorQueryKey<
	Registry extends AnyActorRegistry,
	ActorName extends keyof ExtractActorsFromRegistry<Registry> & string,
>(
	actorOpts: ActorOptions<Registry, ActorName>,
	queryKeyFn?: QueryKeyFn<Registry, ActorName>,
) {
	if (queryKeyFn) {
		return queryKeyFn(actorOpts);
	}

	return [
		"rivetkit",
		"actor",
		actorOpts.name,
		actorOpts.key,
		actorOpts.params ?? null,
		actorOpts.noCreate ?? false,
	] as const satisfies QueryKey;
}

export function syncActorToQueryClient<
	Registry extends AnyActorRegistry,
	ActorName extends keyof ExtractActorsFromRegistry<Registry> & string,
>({
	actorOpts,
	getOrCreateActor,
	queryClient,
	queryKeyFn,
	mount = true,
}: SyncActorOptions<Registry, ActorName>) {
	const { mount: mountActor, state } = getOrCreateActor(actorOpts);
	const queryKey = createActorQueryKey(actorOpts, queryKeyFn);

	const cleanupMount = mount ? mountActor() : () => {};

	// Strip live, non-serializable objects before storing in the query cache.
	// The connection and handle are actor proxies — storing them would cause
	// TanStack Query to call toJSON() on them, which the actor interprets as
	// a remote action invocation.
	function serializeState(s: typeof state.state) {
		const { connection: _connection, handle: _handle, ...serializable } = s as typeof state.state & { connection: unknown; handle: unknown };
		return serializable;
	}

	// Use the updater form so any extra fields merged into the cache entry
	// (e.g. business state pushed from useEvent) are preserved across
	// connection metadata updates.
	function updateCache(s: typeof state.state) {
		queryClient.setQueryData(queryKey, (prev: unknown) => ({
			...(prev as object),
			...serializeState(s),
		}));
	}

	const unsubscribe = state.subscribe(
		(value: { currentVal: typeof state.state }) => {
			updateCache(value.currentVal);
		},
	);

	updateCache(state.state);

	return {
		queryKey,
		unsubscribe: () => {
			unsubscribe();
			cleanupMount();
		},
	};
}
